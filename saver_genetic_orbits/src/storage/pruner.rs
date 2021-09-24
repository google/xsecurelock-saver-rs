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

use log::{error, info};

use super::Storage;

/// Struct used to shutdown pruning.
pub struct Pruner {
    join_handle: Option<JoinHandle<()>>,
    sender: Option<Sender<()>>,
}

// This is safe because we require &mut Self for all methods that access sender, so sharing &self is
// safe though not useful.
unsafe impl Sync for Pruner {}

impl Pruner {
    /// Creates a pruner running on a remote thread which can be triggered to asynchronously prune scenarios.
    pub fn new<S>(number_to_keep: u64, storage: S) -> Pruner
    where
        S: Storage + Send + 'static,
    {
        let (sender, recv) = mpsc::channel();
        let join_handle = thread::spawn(move || {
            let mut storage = storage;
            loop {
                match recv.recv() {
                    Ok(()) => {
                        info!("Pruning scenarios");
                        match storage.keep_top_scenarios_by_score(number_to_keep) {
                            Ok(num_pruned) => info!("Pruned {} scenarios", num_pruned),
                            Err(err) => error!("Falied to prune scenarios: {}", err),
                        }
                    }
                    Err(_) => {
                        info!("Sending final prune and shutting down.");
                        match storage.keep_top_scenarios_by_score(number_to_keep) {
                            Ok(num_pruned) => info!("Pruned {} scenarios", num_pruned),
                            Err(err) => error!("Falied to prune scenarios: {}", err),
                        }
                        break;
                    }
                }
            }
        });

        Pruner {
            join_handle: Some(join_handle),
            sender: Some(sender),
        }
    }

    /// Trigger pruning.
    // this has to be mut so that Sender isn't accidentally shared across threads.
    pub fn prune(&mut self) {
        self.sender
            .as_ref()
            .unwrap()
            .send(())
            .expect("Pruner shut down unexpectedly");
    }
}

impl Drop for Pruner {
    fn drop(&mut self) {
        self.sender.take().unwrap();
        self.join_handle
            .take()
            .unwrap()
            .join()
            .expect("Remote thread paniced");
        info!("Scenario pruner shutdown successfully.");
    }
}
