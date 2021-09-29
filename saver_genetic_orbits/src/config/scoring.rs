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

//! Contains configuration structs for the scoring system.

use std::time::Duration;

use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer, Serialize};

use crate::statustracker::ScoringFunction;

/// Tuning parameters for world scoring.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ScoringConfig {
    /// The number of physics ticks to count the score for. Physics ticks are defined to be 16
    /// milliseconds long. Defaults to 3750, which is approximately 60 seconds.
    #[serde(with = "humantime_serde")]
    pub scored_time: Duration,

    /// The region where planets actually count towards the scenario score.
    pub scored_area: ScoredArea,

    /// Expression that is evaluated each frame to determine the score for that frame, to be added
    /// to the cumulative score. This is a simple math expression and can use three variables:
    ///
    /// - `elapsed` is the percentage of scenario time that has completed, from 0 to 1.
    /// - `total_mass` is the total mass of all planets in the `scored_area`.
    /// - `mass_count` is the number of masses in the `scored_area`.
    ///
    /// The score is "per second" because the output is multiplied by delta time before adding it to
    /// the total score.
    pub score_per_second: ScoringFunction,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        ScoringConfig {
            scored_time: Duration::from_secs(60),
            scored_area: Default::default(),
            score_per_second: "total_mass * mass_count".parse().unwrap(),
        }
    }
}

/// Defines the area where planets are actually scored. Area is centered on the origin, and planets
/// outside of it don't get any score.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ScoredArea {
    // TODO(zstewar1): use a Range<Vector> for the scored area.
    /// The width (x) of the scored region. Defaults to 4000.
    #[serde(deserialize_with = "scored_area_whd_deserialize")]
    pub width: f32,
    /// The height (y) of the scored region. Defaults to 4000.
    #[serde(deserialize_with = "scored_area_whd_deserialize")]
    pub height: f32,
    /// The depth (z) of the scored region. Defaults to 4000.
    #[serde(deserialize_with = "scored_area_whd_deserialize")]
    pub depth: f32,
}

impl Default for ScoredArea {
    fn default() -> Self {
        ScoredArea {
            width: 4000.0,
            height: 4000.0,
            depth: 4000.0,
        }
    }
}

/// Deserializes the width or height of ScoredArea, flipping negatives and changing 0 to 4000.
fn scored_area_whd_deserialize<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>,
{
    let val = f32::deserialize(deserializer)?;
    if val <= 0.0 {
        Err(D::Error::invalid_value(
            Unexpected::Float(val as f64),
            &"a float > 0",
        ))
    } else {
        Ok(val)
    }
}
