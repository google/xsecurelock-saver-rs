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

use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};

use libc;

static RECEIVED_SIGINT: AtomicBool = ATOMIC_BOOL_INIT;

extern "C" fn sigint_handler(_arg: libc::c_int) {
    RECEIVED_SIGINT.store(true, Ordering::Release);
}

#[allow(non_camel_case_types)]
type sighandler_t = extern "C" fn(libc::c_int);

extern "C" {
    fn signal(signum: libc::c_int, handler: sighandler_t) -> sighandler_t;
}

pub(crate) fn received_sigint() -> bool {
    RECEIVED_SIGINT.load(Ordering::Acquire)
}

pub(crate) fn init() {
    unsafe { signal(libc::SIGINT, sigint_handler) };
}
