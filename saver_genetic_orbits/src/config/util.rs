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

//! Contains serializable utility structs which are useful for other config structs.

/// Fully generic serializable vector. xsecurelock::engine::components::physics::Vector should be
/// used instead when possible.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Vector<T> {
    pub x: T,
    pub y: T,
}

/// A range over a generic group of elements. May be inclusive or exclusive depending on context.
/// Both parameters must be specified when this is specified explicitly.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Range<T> {
    pub min: T,
    pub max: T,
}

impl<T> Range<T> where T: PartialOrd + Clone {
    /// Clamps the value to be within this range, assuming the range is inclusive.
    pub fn clamp_inclusive(&self, val: T) -> T {
        assert!(self.min <= self.max, "min must be less than max");
        if val < self.min {
            self.min.clone()
        } else if val > self.max {
            self.max.clone()
        } else {
            val
        }
    }
}

/// A random distribution. This enum is used in places where the configuration should have a choice
/// of several different distribution types.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Distribution {
    /// Use an exponential distribution.
    #[serde(rename = "exponential")]
    Exponential(ExponentialDistribution),
    /// Use a normal distribution.
    #[serde(rename = "normal")]
    Normal(NormalDistribution),
    /// Use a uniform distribution.
    #[serde(rename = "uniform")]
    Uniform(UniformDistribution<f64>),
}

impl Distribution {
    /// Fix invalid to the given default, assuming inclusive uniform.
    pub fn fix_invalid_helper_iu(&mut self, path: &str, name: &str, default: fn() -> Self) {
        ::config::fix_invalid_helper(
            path, name,
            "Exponential must have lambda > 0, Normal must have standard_deviation >= 0, \
            or Uniform must have max >= min",
            self,
            |v| match v {
                Distribution::Exponential(ExponentialDistribution(lambda)) => *lambda > 0.,
                Distribution::Normal(NormalDistribution { standard_deviation, .. }) =>
                    *standard_deviation >= 0.,
                Distribution::Uniform(UniformDistribution { min, max }) => max >= min,
            },
            default,
        );
    }
}

/// A distribution that is required to be exponential. Serializable rand::distributions::Exp.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExponentialDistribution(pub f64);

/// A distribution that is required to be normal. Serializable rand::distributions::Normal.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NormalDistribution {
    /// The mean of the normal distribution.
    pub mean: f64,
    /// The standard deviation of the normal distribution.
    pub standard_deviation: f64,
}

/// A distribution that is required to be uniform. Serializable rand::distributions::Uniform.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UniformDistribution<T> {
    /// The min value for the uniform distribution (always inclusive).
    pub min: T,
    /// The max value for the uniform distribution (may be inclusive or exclusive depending on
    /// context).
    pub max: T,
}
