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
#[macro_use]
extern crate log;
#[macro_use]
extern crate lalrpop_util;

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

#[cfg(feature = "graphical")]
use xsecurelock_saver::engine::EngineBuilder;

use physics::systems::{SympleticEulerForceStep, SympleticEulerVelocityStep};

use crate::{
    collision::{
        DeleteCollidedPlanets,
        MergeCollidedPlanets,
        MergedInto,
    },
    config::{
        GeneticOrbitsConfig,
        generator::GeneratorConfig,
        scoring::ScoringConfig,
    },
    statustracker::{ActiveWorld, ScoreKeeper},
    storage::{
        Storage,
        sqlite::SqliteStorage,
    },
    worldgenerator::WorldGenerator,
};

mod collision;
mod config;
mod model;
mod pruner;
mod storage;
mod statustracker;
mod timer;
mod worldgenerator;

fn main() {
    simple_logger::init().unwrap();

    let config = get_config();

    let GeneticOrbitsConfig { database, generator, scoring } = config;

    let stop_pruning = if let Some(max_scenarios_to_keep) = database.max_scenarios_to_keep {
        let storage = open_storage(database.database_path.as_ref().map(|s| &**s));
        let prune_interval = ::std::time::Duration::from_secs(database.prune_interval_seconds);
        Some(pruner::prune_scenarios(prune_interval, max_scenarios_to_keep, storage))
    } else {
        None
    };

    let storage = open_storage(database.database_path.as_ref().map(|s| &**s));

    use clap::{App, Arg};
    let args = App::new("saver_genetic_orbits")
        .arg(Arg::with_name("headless")
            .long("headless")
            .help(
                "run in headless mode with no grpahics (no effect if not compiled with graphics \
                enabled)"))
        .get_matches();

    run_saver(args.is_present("headless"), scoring, generator, storage);

    if let Some(stop_pruning) = stop_pruning {
        stop_pruning.shutdown();
    }
}

/// internal saver-runner.
fn run_saver<S: Storage + Default + Send + Sync + 'static>(
    headless: bool, scoring: ScoringConfig, generator: GeneratorConfig, storage: S,
) {
    if !headless {
        #[cfg(feature = "graphical")] {
            return run_saver_graphical(scoring, generator, storage);
        }
    } else if cfg!(not(feature = "graphical")) {
        warn!("Headless flag has no effect if not compiled with graphics support");
    }
    run_saver_headless(scoring, generator, storage);
}

#[cfg(feature = "graphical")]
fn run_saver_graphical<S: Storage + Default + Send + Sync + 'static>(
    scoring: ScoringConfig, generator: GeneratorConfig, storage: S,
) {
    info!("Running in graphical mode");
    EngineBuilder::new()
        .with_initial_sceneloader(WorldGenerator::<S>::default())
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
        // Run the real merge just before scoring.
        .with_physics_update_sys(MergeCollidedPlanets, "merge-collided", &[])
        .with_physics_update_sys(DeleteCollidedPlanets, "delete-collided", &["merge-collided"])
        .with_physics_update_barrier()
        // Run scorekeeping before integrating.
        .with_physics_update_sys(ScoreKeeper::<S>::default(), "score-keeper", &[])
        .with_physics_update_sys(GravitySystem, "apply-gravity", &[])
        // Barrier between adding forces and adding integration.
        .with_physics_update_barrier()
        .with_physics_update_sys(SympleticEulerForceStep, "integrate-forces", &[])
        .with_physics_update_sys(
            BruteForceCollisionDetector, "detect-collisions", &["integrate-forces"])
        .with_physics_update_sys(
            SympleticEulerVelocityStep, "integrate-velocities", &["detect-collisions"])
        .build()
        .run();
}

fn run_saver_headless<S: Storage + Default + Send + Sync + 'static>(
    scoring: ScoringConfig, generator: GeneratorConfig, storage: S,
) {
    info!("Running in headless mode");
    use sigint;
    use specs::{World, DispatcherBuilder};
    use scene_management::resources::SceneChange;

    sigint::init();

    let mut world = World::new();
    physics::register(&mut world);
    scene_management::register(&mut world);
    world.register::<CircleCollider>();
    world.register::<GravitySource>();
    world.register::<GravityTarget>();
    world.register::<MergedInto>();
    world.add_resource(GravitationalConstant(1000.));
    world.add_resource(LastUpdateCollisions::default());
    world.add_resource({
        let mut matrix = CollisionMatrix::default();
        matrix.enable_collision(collision::planet(), collision::planet());
        matrix
    });
    world.add_resource(scoring);
    world.add_resource(generator);
    world.add_resource(storage);
    world.add_resource(ActiveWorld::default());
    world.write_resource::<SceneChange>().change_scene(WorldGenerator::<S>::default());

    use physics::systems::{SetupNextPhysicsPosition, ClearForceAccumulators};
    use scene_management::{
        SceneChangeHandlerBuilder,
        systems::DeleteSystem,
    };
    use std::sync::Arc;
    let threadpool = Arc::new(rayon::ThreadPoolBuilder::new().build().unwrap());

    let mut dispatcher = DispatcherBuilder::new()
        .with_pool(Arc::clone(&threadpool))
        .with(SetupNextPhysicsPosition, "", &[])
        .with(ClearForceAccumulators, "", &[])
        .with_barrier()
        // Run the real merge just before scoring.
        .with(MergeCollidedPlanets, "merge-collided", &[])
        .with(DeleteCollidedPlanets, "delete-collided", &["merge-collided"])
        .with_barrier()
        // Run scorekeeping before integrating.
        .with(ScoreKeeper::<S>::default(), "score-keeper", &[])
        .with(GravitySystem, "apply-gravity", &[])
        // Barrier between adding forces and adding integration.
        .with_barrier()
        .with(SympleticEulerForceStep, "integrate-forces", &[])
        .with(
            BruteForceCollisionDetector, "detect-collisions", &["integrate-forces"])
        .with(
            SympleticEulerVelocityStep, "integrate-velocities", &["detect-collisions"])
        .with_barrier()
        .with(DeleteSystem, "", &[])
        .build();

    let mut change_handler = SceneChangeHandlerBuilder::new()
        .with_threadpool(threadpool)
        .with_pre_load_sys(ClearCollisionsInvolvingSceneEntities, "", &[])
        .build();

    dispatcher.setup(&mut world.res);
    change_handler.setup(&mut world);

    while !sigint::received_sigint() {
        dispatcher.dispatch(&mut world.res);
        world.maintain();
        change_handler.handle_scene_change(&mut world);
    }
}

/// The screensaver folder name, used both for saving the database in the user data directory and
/// for looking for configs in the 
const SAVER_DIR: &'static str = "xsecurelock-saver-genetic-orbits";

/// Checks for the config path and either tries to open it or opens the default location.
fn open_storage(config_path: Option<&str>) -> SqliteStorage {
    match config_path {
        Some(path) => {
            info!("using database {}", path);
            SqliteStorage::open(path).unwrap()
        },
        None => open_default_storage(),
    }
}

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
                info!("using database {:?}", data_dir);
                return SqliteStorage::open(data_dir).unwrap();
            },
            Err(err) => error!("Unable to create storage directory ({}), opening in memory", err),
        }
    }
    info!("using in-memory database");
    SqliteStorage::open_in_memory_named("default").unwrap()
}

/// Load the config from the user config directory.
fn get_config() -> GeneticOrbitsConfig {
    match find_config_file() {
        Some(config_file) => {
            match serde_yaml::from_reader::<_, GeneticOrbitsConfig>(config_file) {
                Ok(mut config) => {
                    info!("Successfully loaded config");
                    config.fix_invalid();
                    config
                },
                Err(err) => {
                    error!("Error loading config: {}", err);
                    Default::default()
                },
            }
        },
        None => {
            info!("No config file found, using default config");
            Default::default()
        },
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
    let config_load = File::open(b.as_ref())
        .or_else(|err| if err.kind() == ErrorKind::NotFound {
            Err(())
        } else {
            error!("Unable to read config file {:?}: {}", b.as_ref(), err);
            Err(())
        });
    match config_load {
        Err(()) => None,
        Ok(config) => {
            info!("Using config file {:?}", b.as_ref());
            Some(config)
        },
    }
}
