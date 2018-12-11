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

use crate::time::Time;

pub(crate) fn add_default_resources(world: &mut ::specs::World) {
    world.add_resource(PhysicsDeltaTime::default());
    world.add_resource(PhysicsElapsed::default());
}

/// The amount of time elapsed in the physics simulation.
pub struct PhysicsDeltaTime(pub Time);

impl Default for PhysicsDeltaTime {
    fn default() -> Self { PhysicsDeltaTime(Time::milliseconds(16)) }
}

/// The total elapsed time in the physics simulation.
pub struct PhysicsElapsed {
    pub previous: Time,
    pub current: Time,
}

impl Default for PhysicsElapsed {
    fn default() -> Self { 
        PhysicsElapsed {
            current: Time::ZERO,
            previous: Time::ZERO,
        }
    }
}
