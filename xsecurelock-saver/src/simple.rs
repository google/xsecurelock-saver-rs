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

//! Simple screensavers using SFML graphics. To use, implement [`Screensaver`]. You screensaver type
//! should store whatever state it needs, and update state during `update`, then draw that state
//! onto the render target during `draw`. The `Screensaver` instance can hold any data from [`sfml`]
//! which you nee dto use during drawing or update, as well as any custom state.
//!
//! Once you have a screensaver type, run it with [`run_saver`]. This will handle connecting to the
//! xsecurelock screensaver window and looping until sigint is received. If run outside of
//! XSecurelock, this will create a small window for testing purposes.
//!
//! See `saver_sfmlrect` for basic example usage.

use std::env;

use log::info;

use sfml::graphics::{Color, RenderTarget, RenderWindow};
use sfml::system::Vector2u;
use sfml::window::{ContextSettings, Style};

/// A screensaver which can be run on an SFML RenderTarget.
pub trait Screensaver {
    /// Update the internal state of this screensaver. Will be run as fast as possible by
    /// [`run_saver`].
    fn update(&mut self);

    /// Draw the screensaver on the specified target.
    fn draw<T>(&self, target: &mut T)
    where
        T: RenderTarget;
}

/// Run a screensaver created by the given function. The argument to create will be the size of the
/// render target.
pub fn run_saver<F, S>(create_saver: F)
where
    F: FnOnce(Vector2u) -> S,
    S: Screensaver,
{
    sigint::init();

    let mut window = open_window();
    let mut saver = create_saver(window.size());

    while !sigint::received_sigint() {
        while let Some(_) = window.poll_event() {}

        saver.update();

        window.clear(Color::GREEN);
        saver.draw(&mut window);
        window.display();
    }
    info!("Shutting Down");
}

pub(crate) fn open_window() -> RenderWindow {
    let mut settings = ContextSettings::default();
    settings.set_antialiasing_level(4);
    let window = match env::var("XSCREENSAVER_WINDOW") {
        // Get the ID of the window from the $XSCREENSAVER_WINDOW environment variable, if
        // available, otherwise create a window for testing.
        Ok(window_id_str) => {
            info!("Opening existing window");
            let window_handle = window_id_str.parse().expect("window id was not an integer");
            unsafe { RenderWindow::from_handle(window_handle, &settings) }
        }
        Err(_) => {
            info!("Creating new window");
            RenderWindow::new(
                (1200, 900),
                "Screensaver Test Window",
                Style::NONE,
                &settings,
            )
        }
    };
    info!("Opened SFML Window");
    window
}
