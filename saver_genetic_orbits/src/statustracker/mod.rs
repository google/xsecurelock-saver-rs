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
use std::str::FromStr;

use specs::{
    Join,
    Read,
    ReadStorage,
    System,
    Write,
};

use physics::components::{
    Mass,
    Position,
};
use xsecurelock_saver::engine::{
    components::delete::Deleted,
    resources::{
        draw::View,
        scene::SceneChange,
    },
};

use crate::{
    config::scoring::ScoringConfig,
    model::{Scenario, World},
    storage::Storage,
    worldgenerator::WorldGenerator,
};

use self::scoring_function::Expression;

mod scoring_function;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct ScoringFunction(Expression);

impl ScoringFunction {
    /// Evaluate the expression given the scoring function inputs.
    pub fn eval(&self, tick: f64, total_mass: f64, mass_count: f64) -> f64 {
        self.0.eval(tick, total_mass, mass_count)
    }
}

impl FromStr for ScoringFunction {
    type Err = String;

    fn from_str(source: &str) -> Result<ScoringFunction, String> {
        source.parse().map(ScoringFunction)
    }
}

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

#[derive(Default)]
pub struct ScoreKeeper<T>(PhantomData<T>);

impl<'a, T> System<'a> for ScoreKeeper<T> where T: Storage + Default + Send + Sync + 'static {
    type SystemData = (
        Read<'a, ScoringConfig>,
        Read<'a, View>,
        Write<'a, ActiveWorld>,
        Write<'a, T>,
        Write<'a, SceneChange>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Mass>,
        ReadStorage<'a, Deleted>,
    );

    fn run(
        &mut self,
        (
            scoring,
            view,
            mut world_track,
            mut storage,
            mut scene_change,
            positions,
            masses,
            deleted,
        ): Self::SystemData,
    ) {
        if world_track.ticks_completed < scoring.scored_ticks {
            let vertical_half_extent = scoring.scored_area.height / 2.;
            let horizontal_half_extent = if scoring.scored_area.scale_width_by_aspect {
                // x / y = w / w_0; w_0 * x / y = w
                let aspect = view.size.x / view.size.y;
                scoring.scored_area.width * aspect
            } else {
                scoring.scored_area.width
            } / 2.;

            let mut mass_count = 0f64;
            let mut total_mass = 0f64;
            for (position, mass, _) in (&positions, &masses, !&deleted).join() {
                let pos = position.pos();
                if pos.x.abs() <= horizontal_half_extent && pos.y.abs() <= vertical_half_extent {
                    mass_count += 1.;
                    total_mass += mass.linear as f64
                }
            }
            world_track.cumulative_score += scoring.per_frame_scoring
                .eval(world_track.ticks_completed as f64, total_mass, mass_count);
            world_track.ticks_completed += 1;
        } else {
            info!("Storing scored world");
            let world = mem::replace(&mut world_track.world, World::default());
            let parent = mem::replace(&mut world_track.parent, None);
            let score = match mem::replace(&mut world_track.cumulative_score, 0.) {
                score if score.is_nan() => {
                    warn!("Score was NaN, replacing with -inf");
                    ::std::f64::NEG_INFINITY
                },
                score => score,
            };
            world_track.ticks_completed = 0;
            let store_result = match parent {
                Some(parent) => storage.add_child_scenario(world, score, &parent),
                None => storage.add_root_scenario(world, score),
            };
            match store_result {
                Err(error) => error!("Error while storing finished scenario: {}", error),
                Ok(scenario) => info!(
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
