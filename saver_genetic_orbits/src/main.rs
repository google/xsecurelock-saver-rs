// Copyright 2018-2021 Google LLC
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

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use xsecurelock_saver::engine::XSecurelockSaverPlugins;

mod config;
mod model;
mod statustracker;
mod storage;
mod world;
mod worldgenerator;

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(XSecurelockSaverPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(config::ConfigPlugin)
        .add_state(SaverState::Generate)
        .add_plugin(storage::StoragePlugin)
        .add_plugin(worldgenerator::WorldGeneratorPlugin)
        .add_plugin(statustracker::ScoringPlugin)
        .add_plugin(world::WorldPlugin)
        .run();
}

/// Game state of the generator.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
enum SaverState {
    /// Loading state, world will be replaced.
    Generate,
    /// Run the game.
    Run,
}

// use std::fs;
// use std::fs::File;
// use std::io::{BufReader, ErrorKind};
// use std::path::{Path, PathBuf};
//
// // use crate::config::GeneticOrbitsConfig;
//
// // mod collision;
// mod config;
// mod model;
// mod pruner;
// mod storage;
// // mod statustracker;
// mod timer;
// // mod worldgenerator;
//
// fn main() {
//     let GeneticOrbitsConfig { database, generator, scoring } = get_config();
//
//     let stop_pruning = if let Some(max_scenarios_to_keep) = database.max_scenarios_to_keep {
//         let storage = open_storage(database.database_path.as_ref().map(|s| &**s));
//         let prune_interval = ::std::time::Duration::from_secs(database.prune_interval_seconds);
//         Some(pruner::prune_scenarios(prune_interval, max_scenarios_to_keep, storage))
//     } else {
//         None
//     };
//
//     let storage = open_storage(database.database_path.as_ref().map(|s| &**s));
//
//     use clap::{App, Arg};
//     let args = App::new("saver_genetic_orbits")
//         .arg(Arg::with_name("headless")
//             .long("headless")
//             .help("run in headless mode with no grpahics"))
//         .get_matches_from(
//             // XSecureLock runs with "-root" for XScreenSaver compatibility, and we don't need it,
//             // so skip it.
//             ::std::env::args_os()
//                 .filter(|arg| arg != "-root"));
//
//     run_saver(args.is_present("headless"), scoring, generator, storage);
//
//     if let Some(stop_pruning) = stop_pruning {
//         stop_pruning.shutdown();
//     }
// }
//
// fn run_saver_graphical<S: Storage + Default + Send + Sync + 'static>(
//     scoring: ScoringConfig, generator: GeneratorConfig, storage: S,
// ) {
//     info!("Running in graphical mode");
//     EngineBuilder::new()
//         .with_initial_sceneloader(WorldGenerator::<S>::default())
//         .with_resource(GravitationalConstant(1000.))
//         .with_resource(LastUpdateCollisions::default())
//         .with_resource({
//             let mut matrix = CollisionMatrix::default();
//             matrix.enable_collision(collision::planet(), collision::planet());
//             matrix
//         })
//         .with_resource(scoring)
//         .with_resource(generator)
//         .with_resource(storage)
//         .with_resource(ActiveWorld::default())
//         .with_component::<CircleCollider>()
//         .with_component::<GravitySource>()
//         .with_component::<GravityTarget>()
//         .with_component::<MergedInto>()
//         .with_scene_change_sys(ClearCollisionsInvolvingSceneEntities, "", &[])
//         // Run the real merge just before scoring.
//         .with_physics_update_sys(MergeCollidedPlanets, "merge-collided", &[])
//         .with_physics_update_sys(DeleteCollidedPlanets, "delete-collided", &["merge-collided"])
//         .with_physics_update_barrier()
//         // Run scorekeeping before integrating.
//         .with_physics_update_sys(ScoreKeeper::<S>::default(), "score-keeper", &[])
//         .with_physics_update_sys(GravitySystem, "apply-gravity", &[])
//         // Barrier between adding forces and adding integration.
//         .with_physics_update_barrier()
//         .with_physics_update_sys(SympleticEulerForceStep, "integrate-forces", &[])
//         .with_physics_update_sys(
//             BruteForceCollisionDetector, "detect-collisions", &["integrate-forces"])
//         .with_physics_update_sys(
//             SympleticEulerVelocityStep, "integrate-velocities", &["detect-collisions"])
//         .build()
//         .run();
// }
//
// /// Checks for the config path and either tries to open it or opens the default location.
// fn open_storage(config_path: Option<&str>) -> SqliteStorage {
//     match config_path {
//         Some(path) => {
//             info!("using database {}", path);
//             SqliteStorage::open(path).unwrap()
//         },
//         None => open_default_storage(),
//     }
// }
//
// /// Open SqliteStorage somewhere, either in the user data dir or in memory.
// fn open_default_storage() -> SqliteStorage {
//     if let Some(mut data_dir) = dirs::data_dir() {
//         data_dir.push(SAVER_DIR);
//         let create_res = fs::create_dir(&data_dir)
//             .or_else(|err| if err.kind() == ErrorKind::AlreadyExists {
//                 Ok(())
//             } else {
//                 Err(err)
//             });
//         match create_res {
//             Ok(()) => {
//                 data_dir.push("scenario-db.sqlite3");
//                 info!("using database {:?}", data_dir);
//                 return SqliteStorage::open(data_dir).unwrap();
//             },
//             Err(err) => error!("Unable to create storage directory ({}), opening in memory", err),
//         }
//     }
//     info!("using in-memory database");
//     SqliteStorage::open_in_memory_named("default").unwrap()
// }
