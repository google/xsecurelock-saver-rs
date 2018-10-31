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

#[macro_use]
extern crate serde_derive;

extern crate dirs;
extern crate nalgebra;
extern crate num_complex;
extern crate num_traits;
extern crate rand;
extern crate sfml;
extern crate serde;
extern crate serde_json;

extern crate specs;
extern crate circle_collision;
extern crate gravity;
extern crate xsecurelock_saver;

use gravity::{
    GravitationalConstant,
    GravitySource,
    GravitySystem,
    GravityTarget,
};
use circle_collision::{
    BruteForceCollisionDetector,
    CircleCollider,
    CollisionMatrix,
    LastUpdateCollisions,
};
use xsecurelock_saver::engine::{
    EngineBuilder,
    systems::physics::{
        SympleticEulerForceStep,
        SympleticEulerVelocityStep,
    },
};

use collision::{
    DeleteCollidedPlanets,
    MergeCollidedPlanets,
    MergedInto,
};
use statustracker::{ActiveWorld, ScoreKeeper};
use storage::sqlite::SqliteStorage;
use worldgenerator::WorldGenerator;

mod collision;
mod model;
mod storage;
mod statustracker;
mod worldgenerator;

fn main() {
    let storage = open_storage();

    EngineBuilder::new()
        .with_initial_sceneloader(WorldGenerator::<SqliteStorage>::default())
        .with_resource(GravitationalConstant(1000.))
        .with_resource(LastUpdateCollisions::default())
        .with_resource({
            let mut matrix = CollisionMatrix::default();
            matrix.enable_collision(collision::planet(), collision::planet());
            matrix
        })
        .with_resource(storage)
        .with_resource(ActiveWorld::default())
        .with_component::<CircleCollider>()
        .with_component::<GravitySource>()
        .with_component::<GravityTarget>()
        .with_component::<MergedInto>()
        // Run scorekeeping during the first stage of physics updates.
        .with_physics_update_sys(ScoreKeeper::<SqliteStorage>::default(), "score-keeper", &[])
        .with_physics_update_sys(GravitySystem, "apply-gravity", &[])
        // Barrier between adding forces and adding integration.
        .with_physics_update_barrier()
        .with_physics_update_sys(SympleticEulerForceStep, "integrate-forces", &[])
        .with_physics_update_sys(
            BruteForceCollisionDetector, "detect-collisions", &["integrate-forces"])
        .with_physics_update_sys(
            MergeCollidedPlanets, "merge-collided", &["detect-collisions"])
        .with_physics_update_sys(
            DeleteCollidedPlanets, "delete-collided", &["merge-collided"])
        .with_physics_update_sys(
            SympleticEulerVelocityStep,
            "integrate-velocities",
            &["integrate-forces", "detect-collisions", "merge-collided"],
        )
        .build()
        .run();
}

/// Open SqliteStorage somewhere, either in the user data dir or in memory.
fn open_storage() -> SqliteStorage {
    use std::fs;
    use std::io::ErrorKind;
    if let Some(mut data_dir) = dirs::data_dir() {
        data_dir.push("xsecurelock-saver-genetic-orbits");
        let create_res = fs::create_dir(&data_dir)
            .or_else(|err| if err.kind() == ErrorKind::AlreadyExists {
                Ok(())
            } else {
                Err(err)
            });
        match create_res {
            Ok(()) => {
                data_dir.push("scenario-db.sqlite3");
                return SqliteStorage::open(data_dir).unwrap();
            },
            Err(err) => println!("Unable to create storage directory ({}), opening in memory", err),
        }
    }
    SqliteStorage::open_in_memory().unwrap()
}
