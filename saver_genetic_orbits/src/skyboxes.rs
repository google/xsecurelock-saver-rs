// Copyright 2021 Google LLC
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
use bevy_skybox_cubemap::{SkyboxBundle, SkyboxMaterial, SkyboxTextureConversion};
use rand::seq::SliceRandom;

use crate::SaverState;

pub struct SkyboxesPlugin;

impl Plugin for SkyboxesPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<Skyboxes>()
            .add_startup_system(setup.system())
            .add_system_set(
                SystemSet::on_enter(SaverState::Run).with_system(change_skybox.system()),
            );
    }
}

#[derive(Default)]
struct Skyboxes(Vec<Handle<SkyboxMaterial>>);

/// Loads skybox textures.
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut skyboxes: ResMut<Skyboxes>,
    mut materials: ResMut<Assets<SkyboxMaterial>>,
    mut skybox_conversion: ResMut<SkyboxTextureConversion>,
) {
    for tex in &[
        "skyboxes/1.png",
        "skyboxes/2.png",
        "skyboxes/3.png",
        "skyboxes/4.png",
    ] {
        let tex = asset_server.load(*tex);
        skybox_conversion.make_array(tex.clone());
        let mat = materials.add(SkyboxMaterial::from_texture(tex));
        skyboxes.0.push(mat);
    }

    commands.spawn_bundle(SkyboxBundle::default());
}

/// Randomly selects a new skybox texture.
fn change_skybox(mut query: Query<&mut Handle<SkyboxMaterial>>, skyboxes: Res<Skyboxes>) {
    let skybox = skyboxes.0.choose(&mut rand::thread_rng()).unwrap().clone();
    *query.single_mut().unwrap() = skybox;
}
