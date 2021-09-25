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

//! Screensavers for XSecurelock using SFML or Bevy. Enable one of the features, either `simple` for
//! SFML or `engine` for Bevy, and see the corresponding module for usage.

#[cfg(any(feature = "engine", doc))]
pub mod engine;
#[cfg(any(feature = "simple", doc))]
pub mod simple;
