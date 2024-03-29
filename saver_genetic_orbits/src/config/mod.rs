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

//! Contains structs used for configuring the screensaver.

use bevy::prelude::*;
use figment::providers::{Format, Serialized, Yaml};
use figment::Figment;

use self::camera::CameraConfig;
use self::database::DatabaseConfig;
use self::generator::GeneratorConfig;
use self::scoring::ScoringConfig;

pub mod camera;
pub mod database;
pub mod generator;
pub mod scoring;
pub mod util;

/// The screensaver folder name, used both for saving the database in the user data directory and
/// for looking for configs in the
const SAVER_DIR: &'static str = "xsecurelock-saver-genetic-orbits";

/// Adds figment-based configs.
pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let mut figment = Figment::new();

        if let Some(mut data_dir) = dirs::data_dir() {
            data_dir.push(SAVER_DIR);
            data_dir.push("scenario-db.sqlite3");
            figment = figment.merge(Serialized::defaults(DatabaseConfig {
                database_path: Some(data_dir),
                ..Default::default()
            }));
        }

        if let Some(mut config_dir) = dirs::config_dir() {
            config_dir.push(SAVER_DIR);
            config_dir.push("config.yaml");
            figment = figment.merge(Yaml::file(config_dir));
        }

        if let Some(mut home_dir) = dirs::home_dir() {
            home_dir.push(".xsecurelock-saver-genetic-orbits.yaml");
            figment = figment.merge(Yaml::file(home_dir));
        }

        let camconf = figment.extract::<CameraConfig>().unwrap();
        let dbconf = figment.extract::<DatabaseConfig>().unwrap();
        let scoreconf = figment.extract::<ScoringConfig>().unwrap();
        let genconf = figment.extract::<GeneratorConfig>().unwrap();

        info!("Loaded camera config: {:?}", camconf);
        info!("Loaded database config: {:?}", dbconf);
        info!("Loaded score config: {:?}", scoreconf);
        info!("Loaded generator config: {:?}", genconf);

        app.insert_resource(camconf)
            .insert_resource(dbconf)
            .insert_resource(scoreconf)
            .insert_resource(genconf);
    }
}
