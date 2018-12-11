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

use std::sync::Arc;

use rayon::ThreadPool;
use specs::{Dispatcher, DispatcherBuilder, System, World};

use crate::{
    resources::SceneChange,
    systems::{DeleteSystem, ClearCurrentScene},
};

pub mod components;
pub mod resources;
pub mod systems;

pub fn register(world: &mut World) {
    components::register_all(world);
    resources::add_default_resources(world);
}

/// Builder for scene change handler.
pub struct SceneChangeHandlerBuilder<'a, 'b> {
    scene_change_dispatcher: DispatcherBuilder<'a, 'b>,
}

impl<'a, 'b> SceneChangeHandlerBuilder<'a, 'b> {
    pub fn new() -> Self {
        SceneChangeHandlerBuilder {
            scene_change_dispatcher: DispatcherBuilder::new(),
        }
    }

    /// Set the threadpool to use when dispatching.
    pub fn with_threadpool(mut self, pool: Arc<ThreadPool>) -> Self {
        self.set_threadpool(pool);
        self
    }

    /// Set the threadpool to use when dispatching.
    pub fn set_threadpool(&mut self, pool: Arc<ThreadPool>) {
        self.scene_change_dispatcher.add_pool(pool);
    }

    /// Add a system to run on scene_changes. Arguments are the same as for
    /// `DispatcherBuilder.with`.  Dependencies can only refer to other scene_change systems added
    /// previously. This system will be run only when a scene change has been scheduled and can be
    /// used to clean up and reset resources or entities for the new scene.
    pub fn with_pre_load_sys<S>(mut self, sys: S, name: &str, dep: &[&str]) -> Self
        where S: for<'c> System<'c> + Send + 'a
    {
        self.add_pre_load_sys(sys, name, dep);
        self
    }

    /// Add a system to run on scene_changes. Arguments are the same as for
    /// `DispatcherBuilder.with`.  Dependencies can only refer to other scene_change systems added
    /// previously. This system will be run only when a scene change has been scheduled and can be
    /// used to clean up and reset resources or entities for the new scene.
    pub fn add_pre_load_sys<S>(&mut self, sys: S, name: &str, dep: &[&str])
        where S: for<'c> System<'c> + Send + 'a
    {
        self.scene_change_dispatcher.add(sys, name, dep);
    }

    /// Add a barrier to the pre-load dispatcher.
    pub fn with_pre_load_barrier(mut self) -> Self { self.add_pre_load_barrier(); self }

    /// Add a barrier to the pre-load dispatcher.
    pub fn add_pre_load_barrier(&mut self) { self.scene_change_dispatcher.add_barrier(); }

    pub fn build(self) -> SceneChangeHandler<'a, 'b> {
        SceneChangeHandler { 
            scene_change_dispatcher: self.scene_change_dispatcher
                .with_barrier()
                .with(DeleteSystem, "", &[])
                .with(ClearCurrentScene, "", &[])
                .build(),
        }
    }
}

/// Provides simple handling of scene changes.
pub struct SceneChangeHandler<'a, 'b> {
    scene_change_dispatcher: Dispatcher<'a, 'b>,
}

impl<'a, 'b> SceneChangeHandler<'a, 'b> {
    pub fn setup(&mut self, world: &mut World) {
        self.scene_change_dispatcher.setup(&mut world.res);
    }

    /// Check if a scene loader is set, and if so, clean up the current scene and load the new one.
    pub fn handle_scene_change(&mut self, world: &mut World) {
        let loader = world.write_resource::<SceneChange>().take_scene_changer();
        if let Some(mut loader) = loader {
            self.scene_change_dispatcher.dispatch(&world.res);
            world.maintain();
            loader.dispatch(world);
            world.maintain();
        }
    }
}
