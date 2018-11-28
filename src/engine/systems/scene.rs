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

use specs::{
    Entities,
    Join,
    ReadStorage,
    System,
};

use engine::components::scene::InScene;

pub struct ClearCurrentScene;
impl<'a> System<'a> for ClearCurrentScene {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, InScene>,
    );

    fn run(&mut self, (entities, in_scene): Self::SystemData) {
        for (entity, _) in (&*entities, &in_scene).join() {
            entities.delete(entity).unwrap();
        }
    }
}
