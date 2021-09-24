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

//! Contains configuration structs for the world generator.

use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer, Serialize};

use crate::config::util::{
    Distribution, ExponentialDistribution, NormalDistribution, Range, UniformDistribution,
    Vector as SerVec,
};

/// Tuning parameters for the world generator/mutator.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GeneratorConfig {
    /// The probability of generating a new scenario. Parent scenarios are chosen with an
    /// exponential distribution over all current scenarios. The lambda for the exponential
    /// distribution is chosen so that there is `create_new_scenario_probability` of getting an
    /// index outside of the existing scenario range, which triggers generating a new scenario.
    #[serde(deserialize_with = "deserialize_percent")]
    pub create_new_scenario_probability: f64,

    /// The parameters affecting world mutation.
    pub mutation_parameters: MutationParameters,

    /// The parameters affecting new world generation.
    pub new_world_parameters: NewWorldParameters,
}

/// Deserializes the a float, erroring if it isn't in range [0,1].
fn deserialize_percent<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let val = f64::deserialize(deserializer)?;
    if val < 0.0 || val > 1.0 {
        Err(D::Error::invalid_value(
            Unexpected::Float(val),
            &"a float between 0 and 1 inclusive",
        ))
    } else {
        Ok(val)
    }
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        GeneratorConfig {
            create_new_scenario_probability: 0.05,
            mutation_parameters: Default::default(),
            new_world_parameters: Default::default(),
        }
    }
}

/// Parameters that control initial world generation.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct MutationParameters {
    /// The min and max number of planets to add. Used as a clamp on the add_planets_distribution.
    /// Defaults to [0, 20]. Max is inclusive.
    #[serde(deserialize_with = "Range::deserialize_reorder")]
    pub add_planets_limits: Range<usize>,

    /// Distribution over the number of new planets to add. If using a uniform distribution, the
    /// range is inclusive. Exponential distribution rounds down, normal distribution rounds to
    /// nearest.
    /// The default value is an exponential distribution with lambda chosen to have a 99.9% chance
    /// of having fewer than 10 new planets.
    pub add_planets_dist: Distribution,

    /// The parameters affecting new planets that get added in this mutation.
    pub new_planet_parameters: NewPlanetParameters,

    /// The min and max number of planets to remove. Used as a clamp on the
    /// remove_planets_distribution.  Defaults to [0, 20]. Max is inclusive.
    #[serde(deserialize_with = "Range::deserialize_reorder")]
    pub remove_planets_limits: Range<usize>,

    /// Distribution over the number of new planets to remove. If using a uniform distribution, the
    /// range is inclusive. Exponential distribution rounds down, normal distribution rounds to
    /// nearest.
    /// The default value is an exponential distribution with lambda chosen to have a 99.9% chance
    /// of removing fewer than 10 planets.
    pub remove_planets_dist: Distribution,

    /// Percentage of planets to change, on average.
    #[serde(deserialize_with = "deserialize_percent")]
    pub fraction_of_planets_to_change: f64,

    /// Parameters for how to mutate individual planets.
    pub planet_mutation_parameters: PlanetMutationParameters,
}

impl Default for MutationParameters {
    fn default() -> Self {
        const DEFAULT_ADD_REMOVE_PLANETS_LIMITS: Range<usize> = Range { min: 0, max: 20 };
        // -ln(1 - .999) / 10 = 99.9% chance of adding or removing fewer than 10 planets.
        const DEFAULT_ADD_REMOVE_PLANETS_DIST: Distribution =
            Distribution::Exponential(ExponentialDistribution(0.6907755278982136));
        MutationParameters {
            add_planets_limits: DEFAULT_ADD_REMOVE_PLANETS_LIMITS,
            add_planets_dist: DEFAULT_ADD_REMOVE_PLANETS_DIST,
            new_planet_parameters: Default::default(),
            remove_planets_limits: DEFAULT_ADD_REMOVE_PLANETS_LIMITS,
            remove_planets_dist: DEFAULT_ADD_REMOVE_PLANETS_DIST,
            fraction_of_planets_to_change: 0.10,
            planet_mutation_parameters: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct NewWorldParameters {
    /// Inclusive range over the number of planets that should be generated. Used to cap
    /// distributions with long tails. Defaults to [1, 1000].
    #[serde(deserialize_with = "Range::deserialize_reorder")]
    pub num_planets_range: Range<usize>,
    /// Distribution used for selecting the number of planets to distribute over. If using a
    /// uniform distribution, the range is inclusive. Exponential distribution rounds down, normal
    /// distribution rounds to nearest.
    /// The default value is an exponential distribution with lambda chosen to have a 99.999%
    /// chance of picking fewer than 1000 planets.
    pub num_planets_dist: Distribution,
    /// Parameters for how new planets are generated.
    pub planet_parameters: NewPlanetParameters,
}

impl Default for NewWorldParameters {
    fn default() -> Self {
        NewWorldParameters {
            num_planets_range: Range { min: 1, max: 1000 },
            num_planets_dist:
                // -ln(1 - .99999) / 1000 = 99.999% chance of choosing fewer than 1000 planets.
                Distribution::Exponential(ExponentialDistribution(0.01151292546497023)),
            planet_parameters: Default::default(),
        }
    }
}

/// Parameters to control how new planets are generated.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct NewPlanetParameters {
    /// The distribution the generated planet's start position. Defaults to [-2000, 2000] in each
    /// axis.
    pub start_position: SerVec<UniformDistribution>,
    /// Controls the distribution of starting velocities for planets. Defaults to mean: 0,
    /// stddev:
    /// 20 in both x and y.
    pub start_velocity: SerVec<NormalDistribution>,
    /// A minimum limit on the starting mass of planets. Should be positve (i.e. greater than
    /// zero). defaults to 1.
    #[serde(deserialize_with = "deserialize_min_mass")]
    pub min_start_mass: f32,
    /// Controls the distribution of starting masses for planets. Defaults to mean: 500.
    /// stddev: 400.
    pub start_mass: NormalDistribution,
}

impl Default for NewPlanetParameters {
    fn default() -> Self {
        NewPlanetParameters {
            start_position: SerVec {
                x: UniformDistribution {
                    min: -500.0,
                    max: 500.0,
                },
                y: UniformDistribution {
                    min: -500.0,
                    max: 500.0,
                },
                z: UniformDistribution {
                    min: -500.0,
                    max: 500.0,
                },
            },
            start_velocity: SerVec {
                x: NormalDistribution {
                    mean: 0.,
                    standard_deviation: 20.,
                },
                y: NormalDistribution {
                    mean: 0.,
                    standard_deviation: 20.,
                },
                z: NormalDistribution {
                    mean: 0.,
                    standard_deviation: 20.,
                },
            },
            min_start_mass: 1.,
            start_mass: NormalDistribution {
                mean: 500.,
                standard_deviation: 400.,
            },
        }
    }
}

/// Deserializes the min mass, erroring if not positive.
fn deserialize_min_mass<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>,
{
    let val = f32::deserialize(deserializer)?;
    if val <= 0.0 {
        Err(D::Error::invalid_value(
            Unexpected::Float(val as f64),
            &"a positive float",
        ))
    } else {
        Ok(val)
    }
}

/// Parameters to control how planets are mutated.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct PlanetMutationParameters {
    /// Distribution for how much to change position when modifying the planet. Defaults to a mean
    /// of 0 and a standard deviation of 10 in both x and y.
    pub position_change: SerVec<NormalDistribution>,

    /// Distribution for how much to change velocity when modifying the planet. Defaults to a mean
    /// of 0 and a standard deviation of 10 in both x and y.
    pub velocity_change: SerVec<NormalDistribution>,

    /// Distribution for how much to change mass when modifying the planet. Defaults to a normal
    /// distribution with a mean of 0 and a standard deviation of 100. Cannot be an exponential
    /// distribution because those only go up.
    #[serde(deserialize_with = "deserialize_mass_change")]
    pub mass_change: Distribution,

    /// Min mass that the planet must have, used to clamp the results of the mass change must be
    /// positive. Default is 1.
    #[serde(deserialize_with = "deserialize_min_mass")]
    pub min_mass: f32,
}

impl Default for PlanetMutationParameters {
    fn default() -> Self {
        const DEFAULT_VEC_CHANGE: SerVec<NormalDistribution> = SerVec {
            x: NormalDistribution {
                mean: 0.,
                standard_deviation: 10.,
            },
            y: NormalDistribution {
                mean: 0.,
                standard_deviation: 10.,
            },
            z: NormalDistribution {
                mean: 0.,
                standard_deviation: 10.,
            },
        };
        PlanetMutationParameters {
            position_change: DEFAULT_VEC_CHANGE,
            velocity_change: DEFAULT_VEC_CHANGE,
            mass_change: Distribution::Normal(NormalDistribution {
                mean: 0.,
                standard_deviation: 100.,
            }),
            min_mass: 1.,
        }
    }
}

/// Deserializes the min mass, erroring if not positive.
fn deserialize_mass_change<'de, D>(deserializer: D) -> Result<Distribution, D::Error>
where
    D: Deserializer<'de>,
{
    let val = Distribution::deserialize(deserializer)?;
    if let Distribution::Exponential(_) = val {
        Err(D::Error::invalid_value(
            Unexpected::TupleVariant,
            &"a non-exponential distribution",
        ))
    } else {
        Ok(val)
    }
}
