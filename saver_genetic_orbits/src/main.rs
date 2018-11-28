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
extern crate serde_yaml;

extern crate specs;
extern crate circle_collision;
extern crate gravity;
extern crate xsecurelock_saver;

use std::fs;
use std::fs::File;
use std::io::{BufReader, ErrorKind};
use std::path::{Path, PathBuf};

use gravity::{
    GravitationalConstant,
    GravitySource,
    GravitySystem,
    GravityTarget,
};
use circle_collision::{
    BruteForceCollisionDetector,
    ClearCollisionsInvolvingSceneEntities,
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
use config::GeneticOrbitsConfig;
use statustracker::{ActiveWorld, ScoreKeeper};
use storage::sqlite::SqliteStorage;
use worldgenerator::WorldGenerator;

mod collision;
mod config;
mod model;
mod storage;
mod statustracker;
mod worldgenerator;

fn main() {
    let config = get_config();
    let storage = match &config.database.database_path {
        Some(path) => {
            println!("using database {}", path);
            SqliteStorage::open(path).unwrap()
        },
        None => open_default_storage(),
    };
    
    let GeneticOrbitsConfig { scoring, generator, .. } = config;


    EngineBuilder::new()
        .with_initial_sceneloader(WorldGenerator::<SqliteStorage>::default())
        .with_resource(GravitationalConstant(1000.))
        .with_resource(LastUpdateCollisions::default())
        .with_resource({
            let mut matrix = CollisionMatrix::default();
            matrix.enable_collision(collision::planet(), collision::planet());
            matrix
        })
        .with_resource(scoring)
        .with_resource(generator)
        .with_resource(storage)
        .with_resource(ActiveWorld::default())
        .with_component::<CircleCollider>()
        .with_component::<GravitySource>()
        .with_component::<GravityTarget>()
        .with_component::<MergedInto>()
        .with_scene_change_sys(ClearCollisionsInvolvingSceneEntities, "", &[])
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

/// The screensaver folder name, used both for saving the database in the user data directory and
/// for looking for configs in the 
const SAVER_DIR: &'static str = "xsecurelock-saver-genetic-orbits";

/// Open SqliteStorage somewhere, either in the user data dir or in memory.
fn open_default_storage() -> SqliteStorage {
    if let Some(mut data_dir) = dirs::data_dir() {
        data_dir.push(SAVER_DIR);
        let create_res = fs::create_dir(&data_dir)
            .or_else(|err| if err.kind() == ErrorKind::AlreadyExists {
                Ok(())
            } else {
                Err(err)
            });
        match create_res {
            Ok(()) => {
                data_dir.push("scenario-db.sqlite3");
                println!("using database {:?}", data_dir);
                return SqliteStorage::open(data_dir).unwrap();
            },
            Err(err) => println!("Unable to create storage directory ({}), opening in memory", err),
        }
    }
    println!("using in-memory database");
    SqliteStorage::open_in_memory().unwrap()
}

/// Load the config from the user config directory.
fn get_config() -> GeneticOrbitsConfig {
    match find_config_file() {
        Some(config_file) => {
            match serde_yaml::from_reader::<_, GeneticOrbitsConfig>(config_file) {
                Ok(mut config) => {
                    config.fix_invalid();
                    config
                },
                Err(err) => {
                    println!("Error loading config: {}", err);
                    Default::default()
                },
            }
        },
        None => Default::default(),
    }
}

fn find_config_file() -> Option<BufReader<File>> {
    let config_file = dirs::home_dir()
        .and_then(|mut home| {
            home.push(".xsecurelock-saver-genetic-orbits.yaml");
            try_open_config_file(home)
        })
        .or_else(|| {
            dirs::config_dir()
                .and_then(|mut config| {
                    config.push(SAVER_DIR);
                    config.push("config.yaml");
                    try_open_config_file(config)
                })
        });
    let config_file = if cfg!(target_family = "unix") {
        config_file.or_else(|| try_open_config_file(
                PathBuf::from(r"/etc/xsecurelock-saver-genetic-orbits/config.yaml")))
    } else {
        config_file
    };
    config_file.map(BufReader::new)
}

fn try_open_config_file<P: AsRef<Path>>(b: P) -> Option<File> {
    File::open(b.as_ref())
        .or_else(|err| if err.kind() == ErrorKind::NotFound {
            Err(())
        } else {
            println!("unable to read config file {:?}: {}", b.as_ref(), err);
            Err(())
        })
        .ok()
}
