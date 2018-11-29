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

//! Contains structs used for configuring the screensaver.

use self::database::DatabaseConfig;
use self::generator::GeneratorConfig;
use self::scoring::ScoringConfig;

pub mod database;
pub mod generator;
pub mod scoring;
pub mod util;

/// Internal trait for config validation.
trait Validation {
    /// Log a warning about invalid configs and replace them with default values. path is used to
    /// provide an exact path to each invalid item.
    fn fix_invalid(&mut self, path: &str);
}

fn fix_invalid_helper<T, F>(
    path: &str, name: &str, requirement_desc: &str, 
    source: &mut T, check_valid: F, default: fn() -> T,
) where 
    T: ::std::fmt::Debug,
    F: FnOnce(&T) -> bool
{
    if !check_valid(source) {
        let old = ::std::mem::replace(source, default());
        warn!(
            "{}.{} {}, but was {:?}; reset to default of {:?}.",
            path, name, requirement_desc, old, source,
        );
    }
}

/// Global configuration for the Genetic Orbits screensaver.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct GeneticOrbitsConfig {
    /// Parameters for the database.
    #[serde(default)]
    pub database: DatabaseConfig,

    /// Parameters for the generator/mutator.
    #[serde(default)]
    pub generator: GeneratorConfig,

    /// Parameters affecting scoring.
    #[serde(default)]
    pub scoring: ScoringConfig,
}

impl GeneticOrbitsConfig {
    /// Log a warning about invalid configs and replace them with default values. 
    pub fn fix_invalid(&mut self) {
        // database has no validation.
        self.generator.fix_invalid("generator");
        self.scoring.fix_invalid("scoring");
    }
}
