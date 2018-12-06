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

use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

/// Every `interval` send `value` into the given channel. Stops sending only when the channel is
/// closed.
pub fn interval<T: Clone + Send + 'static>(interval: Duration, chan: Sender<T>, value: T) {
    thread::spawn(move || {
        loop {
            thread::sleep(interval);
            match chan.send(value.clone()) {
                Ok(()) => {},
                Err(_) => break,
            };
        }
    });
}
