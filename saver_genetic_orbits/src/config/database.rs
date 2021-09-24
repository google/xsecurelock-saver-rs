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

//! Contains configuration structs for the database.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Configuration parameters for the Sqlite Database.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct DatabaseConfig {
    /// The path to the SqliteDatabase to use. If set, the parent directory must exist and the
    /// location must be writable. Saver will never fall back to an in-memory database if this is
    /// set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_path: Option<PathBuf>,

    /// Sets the cap for the number of scenarios to keep in the database. Set to None for
    /// unlimited. Defaults to 1,000,000.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_scenarios_to_keep: Option<u64>,

    /// How often (in seconds) to prune excess scenarios while running normally. Defaults to every
    /// 20 minutes (1200 seconds). Regardless of what this is set to, it will always prune on
    /// shutdown unless max_scenarios_to_keep is unset.
    pub prune_interval_seconds: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig {
            database_path: None,
            max_scenarios_to_keep: Some(1000000),
            prune_interval_seconds: 1200,
        }
    }
}
