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
use bevy_skybox_cubemap::SkyboxPlugin;
use xsecurelock_saver::engine::XSecurelockSaverPlugins;

mod config;
mod model;
mod skyboxes;
mod statustracker;
mod storage;
mod world;
mod worldgenerator;

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(XSecurelockSaverPlugins)
        .add_plugin(SkyboxPlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(config::ConfigPlugin)
        .add_state(SaverState::Generate)
        .add_plugin(storage::StoragePlugin)
        .add_plugin(worldgenerator::WorldGeneratorPlugin)
        .add_plugin(statustracker::ScoringPlugin)
        .add_plugin(world::WorldPlugin)
        .add_plugin(skyboxes::SkyboxesPlugin)
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
