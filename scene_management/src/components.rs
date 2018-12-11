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

use specs::{Component, NullStorage, World};

pub(crate) fn register_all(world: &mut World) {
    world.register::<InScene>();
    world.register::<Deleted>();
}

/// A component which marks an object as being part of the current "Scene", it will be removed when
/// the scene changes.
#[derive(Default)]
pub struct InScene;
impl Component for InScene { type Storage = NullStorage<Self>; }

/// A component which marks an object as deleted. Will be cleaned up at the end of the current
/// dispatcher run before the next call to maintain.
/// This is useful because Entities.delete(ent) does not apply until the next call to maintain, but
/// there's no way to query for deleted entities. If systems want to delete entities early and let
/// other systems see that the entities are meant to be deleted before the next call to maintain,
/// they can use this.
#[derive(Default)]
pub struct Deleted;
impl Component for Deleted { type Storage = NullStorage<Self>; }
