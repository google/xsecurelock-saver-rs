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

use rand;
use rand::{Error, RngCore};

/// An RNG which just uses the TheadRng for the current thread for all calls. This is essentially a
/// dependency-injection wrapper arount thread_rng, i.e. to let you say "please just use
/// thread_rng()" to something which takes an Rng or RngCore as a parameter. May cause unnecessary
/// repeated increment/decrement of the ThreadRng reference counter.
pub struct PerThreadRng;

impl RngCore for PerThreadRng {
    fn next_u32(&mut self) -> u32 {
        rand::thread_rng().next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        rand::thread_rng().next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        rand::thread_rng().fill_bytes(dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        rand::thread_rng().try_fill_bytes(dest)
    }
}
