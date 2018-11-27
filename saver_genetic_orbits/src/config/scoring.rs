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

use config::{Validation, fix_invalid_helper};

/// Tuning parameters for world scoring.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScoringConfig {
    /// The number of physics ticks to count the score for. Physics ticks are defined to be 16
    /// milliseconds long. Defaults to 3750, which is approximately 60 seconds.
    #[serde(default = "ScoringConfig::default_scored_ticks")]
    pub scored_ticks: u32,

    /// The region where planets actually count towards the scenario score.
    #[serde(default)]
    pub scored_area: ScoredArea,
}

impl ScoringConfig {
    /// 1 minute (60,000 milliseconds) / 16 milliseconds per tick
    fn default_scored_ticks() -> u32 { 3750 }
}

impl Default for ScoringConfig {
    fn default() -> Self {
        ScoringConfig {
            scored_ticks: Self::default_scored_ticks(),
            scored_area: Default::default(),
        }
    }
}

impl Validation for ScoringConfig {
    fn fix_invalid(&mut self, path: &str) {
        fix_invalid_helper(
            path, "scored_ticks", "must be > 0",
            &mut self.scored_ticks,
            |&v| v > 0,
            Self::default_scored_ticks,
        );
        self.scored_area.fix_invalid(&[path, "scored_area"].join("."));
    }
}

/// Defines the area where planets are actually scored. Area is centered on the origin, and planets
/// outside of it don't get any score. Note that the screen is scaled on startup, so the units are
/// *not* pixels. In general the screen is set up so that the height is 2000 units and the width is
/// height * aspect-ratio.
///
/// The default size is 4000x4000. On a 16:9 monitor with 2000 high, the width will be ~3555, so
/// 4000x4000 gives a nice rectangular scoring area with a bit of margin on most standard ratio
/// monitors. Some users may want to modify this to match their monitors.
///
/// If you want to match individual monitor sizes, you can use `scale_width_by_aspect` to scale the
/// width according to the aspect ratio of the monitor. With this on, if you want to exactly match
/// the screen, you should set both height and width to 2000.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScoredArea {
    // TODO(zstewar1): use a Range<Vector> for the scored area.
    /// The width of the scored region. Defaults to 4000.
    #[serde(default = "ScoredArea::default_scored_width")]
    pub width: f32,
    /// The height of the scored region. Defaults to 4000.
    #[serde(default = "ScoredArea::default_scored_height")]
    pub height: f32,
    /// Whether to scale the width based on the aspect ratio. Defaults to false.
    #[serde(default)]
    pub scale_width_by_aspect: bool,
}

impl ScoredArea {
    fn default_scored_width() -> f32 { 4000. }
    fn default_scored_height() -> f32 { 4000. }
}

impl Default for ScoredArea {
    fn default() -> Self {
        ScoredArea {
            width: Self::default_scored_width(),
            height: Self::default_scored_height(),
            scale_width_by_aspect: Default::default(),
        }
    }
}

impl Validation for ScoredArea {
    fn fix_invalid(&mut self, path: &str) {
        fix_invalid_helper(
            path, "width", "must be >= 0",
            &mut self.width,
            |&v| v >= 0.,
            Self::default_scored_width,
        );
        fix_invalid_helper(
            path, "height", "must be >= 0",
            &mut self.height,
            |&v| v >= 0.,
            Self::default_scored_height,
        );
        // no validation for scale_width_by_aspect
    }
}
