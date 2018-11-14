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

fn default_create_new_scenario_probability() -> f64 { 0.05 }

/// Tuning parameters for the world generator/mutator.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeneratorConfig {
    /// The probability of generating a new scenario. Parent scenarios are chosen with an
    /// exponential distribution over all current scenarios. The lambda for the exponential
    /// distribution is chosen so that there is `create_new_scenario_probability` of getting an
    /// index outside of the existing scenario range, which triggers generating a new scenario.
    #[serde(default = "default_create_new_scenario_probability")]
    pub create_new_scenario_probability: f64,

    /// The parameters affecting world mutation.
    #[serde(default)]
    pub mutation_parameters: MutationParameters,

    /// The parameters affecting new world generation.
    #[serde(default)]
    pub new_world_parameters: NewWorldParameters,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        GeneratorConfig {
            create_new_scenario_probability: default_create_new_scenario_probability(),
            mutation_parameters: Default::default(),
            new_world_parameters: Default::default(),
        }
    }
}

/// Parameters that control initial world generation.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MutationParameters {
}

impl Default for MutationParameters {
    fn default() -> Self {
        MutationParameters {
        }
    }
}

fn default_num_planets_range() -> Range<usize> { Range { min: 1, max: 1000 } }
fn default_num_planets_dist() -> Distribution {
    // -ln(1 - .99999) / 1000 = 99.999% chance of choosing fewer than 1000 planets.
    Distribution::Exponential(ExponentialDistribution(0.01151292546497023))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewWorldParameters {
    /// Inclusive range over the number of planets that should be generated. Used to cap
    /// distributions with long tails. Defaults to [1, 1000].
    #[serde(default = "default_num_planets_range")]
    pub num_planets_range: Range<usize>,
    /// Distribution used for selecting the number of planets to distribute over. If using a
    /// uniform distribution, the range is inclusive.
    /// The default value is an exponential distribution with lambda chosen to have a 99.999%
    /// chance of picking fewer than 1000 planets.
    #[serde(default = "default_num_planets_dist")]
    pub num_planets_dist: Distribution,
}

impl Default for NewWorldParameters {
    fn default() -> Self {
        NewWorldParameters {
            num_planets_range: default_num_planets_range(),
            num_planets_dist: default_num_planets_dist(),
        }
    }
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
