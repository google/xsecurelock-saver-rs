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

use specs::{SystemData, World};

pub(crate) fn add_default_resources(world: &mut World) {
    world.add_resource(SceneChange(None));
}

/// Builds a new scene after a previous one has been cleaned up.
pub(crate) trait SceneLoaderDispatcher {
    fn dispatch(&mut self, world: &mut World);
}

pub trait SceneLoader<'a> {
    type SystemData: SystemData<'a>;

    fn load(&mut self, data: Self::SystemData);
}

impl<L> SceneLoaderDispatcher for L where L: for<'a> SceneLoader<'a> {
    fn dispatch(&mut self, world: &mut World) {
        world.exec(|data: L::SystemData| self.load(data));
    }
}

/// Resource used to set a scene change.
#[derive(Default)]
pub struct SceneChange(Option<Box<dyn SceneLoaderDispatcher + Send + Sync>>);

impl SceneChange {
    /// Will cause the scene to change after the current frame. If another scene change is already
    /// configured, this will override it.
    pub fn change_scene<T>(&mut self, scene_loader: T) 
        where T: for<'a> SceneLoader<'a> + Send + Sync + 'static
    {
        self.0 = Some(Box::new(scene_loader));
    }

    /// True if a scene change is scheduled for after the end of the current frame.
    pub fn is_scene_change_scheduled(&self) -> bool {
        self.0.is_some()
    }

    /// Cancels a scene change scheduled for after the end of the current frame.
    pub fn cancel_scene_change(&mut self) {
        self.0 = None;
    }

    /// Retrieves the pending scene changer, clearing it in the process. This is needed because the
    /// scene change resource cannot be borrowed when the scene loader is executed.
    pub(crate) fn take_scene_changer(&mut self) 
        -> Option<Box<dyn SceneLoaderDispatcher + Send + Sync>>
    {
        self.0.take()
    }
}
