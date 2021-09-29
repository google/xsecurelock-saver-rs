// Copyright 2021 Google LLC
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

use serde::{Deserialize, Serialize};

/// Configuration for the scenario camera.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct CameraConfig {
    /// Relative rotation speed.
    pub rotation_speed: f32,

    /// How far from the origin the camera should be.
    pub view_dist: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            rotation_speed: 0.1,
            view_dist: 1000.0,
        }
    }
}
