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

use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use storage::Storage;
use timer;

/// Tick action received on the timer channel.
#[derive(Clone)]
enum Action {
    /// The timer tick has expired, so prune now.
    Tick,
    /// The main screensaver has shutdown, so prune now and exit the loop.
    Shutdown,
}

/// Struct used to shutdown pruning.
pub struct ShutdownPrune {
    join_handle: JoinHandle<()>,
    sender: Sender<Action>,
}

impl ShutdownPrune {
    /// Consumes the shutdown handle, instructing the scenario pruner to run one final prune and
    /// then shut down.. Blocks until the pruner has shut down.
    pub fn shutdown(self) {
        self.sender.send(Action::Shutdown)
            .expect("Scenario pruner has already closed the shutdown channel!");
        self.join_handle.join().expect("Secnario Pruner shut down with a panic.");
        info!("Scenario pruner shutdown successfully.");
    }
}

/// Starts automatic background pruning of scenarios. Returns a handle that can be used to shutdown
/// background pruning.
pub fn prune_scenarios<S>(interval: Duration, number_to_keep: u64, storage: S) -> ShutdownPrune 
where S: Storage + Send + 'static
{
    let (send, recv) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut storage = storage;
        loop {
            match recv.recv() {
                Ok(Action::Tick) => {
                    info!("Pruning scenarios");
                    match storage.keep_top_scenarios_by_score(number_to_keep) {
                        Ok(num_pruned) => info!("Pruned {} scenarios", num_pruned),
                        Err(err) => error!("Falied to prune scenarios: {}", err),
                    }
                },
                Ok(Action::Shutdown) => {
                    info!("Sending final prune and shutting down.");
                    match storage.keep_top_scenarios_by_score(number_to_keep) {
                        Ok(num_pruned) => info!("Pruned {} scenarios", num_pruned),
                        Err(err) => error!("Falied to prune scenarios: {}", err),
                    }
                    break;
                },
                Err(_) => {
                    error!("Cannot get pruner action; channel already shut down!");
                    break;
                },
            }
        }
    });
    timer::interval(interval, send.clone(), Action::Tick);
    ShutdownPrune {
        join_handle: handle,
        sender: send,
    }
}
