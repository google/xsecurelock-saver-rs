// Copyright 2018-2021 Google LLC
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

use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer, Serialize};

/// Fully serializable generic vector.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Vector<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

/// A range over a generic group of elements. May be inclusive or exclusive depending on context.
/// Both parameters must be specified when this is specified explicitly.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Range<T> {
    pub min: T,
    pub max: T,
}

impl<'de, T> Range<T>
where
    T: PartialOrd + Deserialize<'de>,
{
    pub(super) fn deserialize_reorder<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut res = Self::deserialize(deserializer)?;
        if res.min > res.max {
            std::mem::swap(&mut res.min, &mut res.max);
        }
        Ok(res)
    }
}

impl<T> Range<T>
where
    T: PartialOrd + Clone,
{
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
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Distribution {
    /// Use an exponential distribution.
    Exponential(ExponentialDistribution),
    /// Use a normal distribution.
    Normal(NormalDistribution),
    /// Use a uniform distribution.
    Uniform(UniformDistribution),
}

/// A distribution that is required to be exponential. Serializable rand::distributions::Exp.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExponentialDistribution(
    #[serde(deserialize_with = "deserialize_exponential_lambda")] pub f64,
);

/// Deserializes an exponential distribution, flipping negative values and setting 0 to 1.
fn deserialize_exponential_lambda<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let val = f64::deserialize(deserializer)?;
    if val <= 0.0 {
        Err(D::Error::invalid_value(
            Unexpected::Float(val),
            &"a float > 0",
        ))
    } else {
        Ok(val)
    }
}

/// A distribution that is required to be normal. Serializable rand::distributions::Normal.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NormalDistribution {
    /// The mean of the normal distribution.
    pub mean: f64,
    /// The standard deviation of the normal distribution.
    #[serde(deserialize_with = "deserialize_normal_mean")]
    pub standard_deviation: f64,
}

/// Deserializes an exponential distribution, flipping negative values.
fn deserialize_normal_mean<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(f64::deserialize(deserializer)?.abs())
}

/// A distribution that is required to be uniform. Serializable rand::distributions::Uniform.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(try_from = "uniform_distribution_de::UniformDistribution")]
pub struct UniformDistribution {
    /// The min value for the uniform distribution (always inclusive).
    pub min: f64,
    /// The max value for the uniform distribution (may be inclusive or exclusive depending on
    /// context).
    pub max: f64,
}

mod uniform_distribution_de {
    use serde::Deserialize;

    /// Shadow type that can implement Deserialize.
    #[derive(Deserialize, Debug, Clone)]
    pub(super) struct UniformDistribution {
        /// The min value for the uniform distribution (always inclusive).
        min: f64,
        /// The max value for the uniform distribution (may be inclusive or exclusive depending on
        /// context).
        max: f64,
    }

    impl From<UniformDistribution> for super::UniformDistribution {
        fn from(mut ud: UniformDistribution) -> Self {
            if ud.min > ud.max {
                std::mem::swap(&mut ud.min, &mut ud.max);
            }
            Self {
                min: ud.min,
                max: ud.max,
            }
        }
    }
}
