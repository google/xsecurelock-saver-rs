// Copyright 2018 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::marker::PhantomData;
use std::mem;

use specs::{
    Join,
    ReadStorage,
    System,
    Write,
};

use xsecurelock_saver::engine::{
    components::physics::{
        Mass,
        Position,
    },
    resources::scene::SceneChange,
};

use model::{Scenario, World};
use storage::Storage;
use worldgenerator::WorldGenerator;

/// Resource for tracking the status of the currently active scene.
pub struct ActiveWorld {
    /// The world being scored.
    pub world: World,
    /// The parent of the world being scored.
    pub parent: Option<Scenario>,
    /// The score the world has received so far.
    pub cumulative_score: f64,
    /// The number of physics ticks that the world has been scored on so far.
    pub ticks_completed: u32,
}

impl Default for ActiveWorld {
    fn default() -> Self {
        ActiveWorld {
            world: World { planets: vec![] },
            parent: None,
            cumulative_score: 0.,
            ticks_completed: 0,
        }
    }
}

/// 60,000 milliseconds (1 minute) / 16 milliseconds per tick
const SCORED_TICKS: u32 = 3750;

pub const SCORED_SCREEN_WIDTH: f32 = 2000.;
pub const SCORED_SCREEN_HEIGHT: f32 = 2000.;

#[derive(Default)]
pub struct ScoreKeeper<T>(PhantomData<T>);

impl<'a, T> System<'a> for ScoreKeeper<T> where T: Storage + Default + Send + Sync + 'static {
    type SystemData = (
        Write<'a, ActiveWorld>,
        Write<'a, T>,
        Write<'a, SceneChange>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Mass>,
    );

    fn run(
        &mut self,
        (
            mut world_track,
            mut storage,
            mut scene_change,
            positions,
            masses,
        ): Self::SystemData,
    ) {
        if world_track.ticks_completed < SCORED_TICKS {
            let mut mass_count = 0f64;
            let mut total_mass = 0f64;
            for (position, mass) in (&positions, &masses).join() {
                let pos = position.pos();
                if pos.x.abs() < SCORED_SCREEN_WIDTH && pos.y.abs() < SCORED_SCREEN_HEIGHT {
                    mass_count += 1.;
                    total_mass += mass.linear as f64
                }
            }
            world_track.cumulative_score += total_mass * mass_count;
            world_track.ticks_completed += 1;
        } else {
            println!("Storing scored world");
            let world = mem::replace(&mut world_track.world, World::default());
            let parent = mem::replace(&mut world_track.parent, None);
            let score = mem::replace(&mut world_track.cumulative_score, 0.);
            world_track.ticks_completed = 0;
            let store_result = match parent {
                Some(parent) => storage.add_child_scenario(world, score, &parent),
                None => storage.add_root_scenario(world, score),
            };
            match store_result {
                Err(error) => println!("Error while storing finished scenario: {}", error),
                Ok(scenario) => println!(
                    "Saved scenario {} (parent: {:?}, family: {}, generation: {}) with score {}",
                    scenario.id,
                    scenario.parent,
                    scenario.family,
                    scenario.generation,
                    scenario.score,
                ),
            }
            scene_change.change_scene(WorldGenerator::<T>::default());
        }
    }
}
