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

use specs::{Resources, System, SystemData};

pub trait SpecializedSystem<'a, T> {
    type SystemData: SystemData<'a>;

    fn run_special(&mut self, specialized: T, data: Self::SystemData);

    fn setup_special(&mut self, _specialized: T, res: &mut Resources) {
        Self::SystemData::setup(res);
    }
}

pub(crate) trait SpecializedSystemObject<'a, T> {
    fn run(&mut self, special_data: T, res: &'a Resources);

    fn setup(&mut self, special_data: T, res: &mut Resources);
}

impl<'a, T, S> SpecializedSystemObject<'a, T> for S
    where S: SpecializedSystem<'a, T>,
{
    fn run(&mut self, special_data: T, res: &'a Resources) {
        use specs::RunNow;
        SpecializedSystemWrapper{
            system: self,
            special_data: Some(special_data),
        }.run_now(res);
    }

    fn setup(&mut self, special_data: T, res: &mut Resources) {
        self.setup_special(special_data, res);
    }
}

/// Wraps a SpecializedSystem as a System, allowing us to pass through extra
/// data. Should only be used once, and only for run, not setup. Becomes 
/// invalidated after use. Should only be used by the generic implementation of
/// SpecializedSystemObject.
struct SpecializedSystemWrapper<'b, T, S> 
    where T: 'b,
          S: 'b
{
    system: &'b mut S,
    special_data: Option<T>,
}

impl<'a, 'b, T, S> System<'a> for SpecializedSystemWrapper<'b, T, S>
    where S: SpecializedSystem<'a, T>,
{
    type SystemData = S::SystemData;

    fn run(&mut self, data: Self::SystemData) {
        self.system.run_special(
            self.special_data.take().expect("Cannot re-use the wrapper"),
            data,
        );
    }

    fn setup(&mut self, _res: &mut Resources) {
        panic!("Don't use the wrapper to set up SpecializedSystems.");
   }
}
