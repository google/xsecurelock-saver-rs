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

use self::database::DatabaseConfig;
use self::generator::GeneratorConfig;
use self::scoring::ScoringConfig;

pub mod database;
pub mod generator;
pub mod scoring;

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
