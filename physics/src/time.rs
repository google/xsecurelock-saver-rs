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

pub use self::time_impl::Time;

#[cfg(not(feature = "graphical"))] 
mod time_impl {
    use std::ops::{Add, AddAssign, Sub, SubAssign, Neg};
    
    /// A partial copy fo the SFML time api that doesn't depend on SFML.
    #[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd)]
    pub struct Time(/* microseconds */ i64);

    
    impl Time {
        const MICROSECONDS_PER_SECOND: f32 = 1000000.;
        const MICROSECONDS_PER_MILLISECOND: i64 = 1000;

        pub const ZERO: Time = Time(0);

        pub fn seconds(seconds: f32) -> Self {
            Time((seconds * Self::MICROSECONDS_PER_SECOND) as i64)
        }
    
        pub fn milliseconds(ms: i32) -> Self {
            Time(ms as i64 * Self::MICROSECONDS_PER_MILLISECOND)
        }
    
        pub fn as_seconds(self) -> f32 {
            self.0 as f32 / Self::MICROSECONDS_PER_SECOND
        }
    }
    
    impl Add for Time {
        type Output = Self;
    
        fn add(mut self, rhs: Self) -> Self { self += rhs; self }
    }
    
    impl AddAssign for Time {
        fn add_assign(&mut self, Time(rhs): Self) { self.0 += rhs }
    }
    
    impl Neg for Time {
        type Output = Self;
    
        fn neg(self) -> Self { Time(-self.0) }
    }
    
    impl Sub for Time {
        type Output = Self;
    
        fn sub(mut self, rhs: Self) -> Self { self -= rhs; self }
    }
    
    impl SubAssign for Time {
        fn sub_assign(&mut self, Time(rhs): Self) { self.0 -= rhs; }
    }
}

#[cfg(feature = "graphical")] 
mod time_impl {
    extern crate sfml;

    pub type Time = sfml::system::Time;
}
