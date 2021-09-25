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

//! A module providing an engine for game-like screensavers, using [Bevy](https://bevyengine.org).
//! Provides [`XSecurelockSaverPlugins`] which replaces the Bevy [`DefaultPlugins`], and hacks the
//! engine to use the window provided by XSecurelock instead of `winit` when running inside of
//! XSecurelock. Outside of XSecurelock, functions like `DefaultPlugins`. You can plug this into an
//! [`App`] like pretty much any other plugin.
use std::env;

use bevy::app::{Events, ManualEventReader, PluginGroupBuilder};
use bevy::prelude::*;
use bevy::wgpu::WgpuPlugin;
use bevy::window::{CreateWindow, WindowCreated, WindowPlugin};
use bevy::winit::WinitPlugin;
use bevy_wgpu_xsecurelock::ExternalXWindow;

/// A Bevy plugin for making the bevy app work as an X-Securelock screenaver using SFML rendering.
#[derive(Debug)]
pub struct XSecurelockSaverPlugins;

impl PluginGroup for XSecurelockSaverPlugins {
    fn build(&mut self, plugins: &mut PluginGroupBuilder) {
        DefaultPlugins.build(plugins);
        plugins
            .disable::<WinitPlugin>()
            .disable::<WgpuPlugin>()
            .add_before::<WindowPlugin, _>(ConfigWindowPlugin)
            .add(bevy_wgpu_xsecurelock::WgpuPlugin)
            .add(CreateWindowPlugin)
            .add(RunnerPlugin);
    }
}

#[derive(Debug)]
struct ConfigWindowPlugin;

impl Plugin for ConfigWindowPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // Get the ID of the window from the $XSCREENSAVER_WINDOW environment variable, and attach a ExternalXWindow if so.
        if let Ok(window_id_str) = env::var("XSCREENSAVER_WINDOW") {
            info!("Opening existing window");
            let handle = window_id_str.parse().expect("window id was not an integer");
            let external_window = ExternalXWindow::new(handle);

            app.insert_resource(external_window.bevy_window_descriptor());
            app.insert_resource(external_window);
        } else {
            info!("Using winit");
            app.add_plugin(WinitPlugin::default());
        }
    }
}

#[derive(Debug)]
struct CreateWindowPlugin;

impl Plugin for CreateWindowPlugin {
    fn build(&self, app: &mut AppBuilder) {
        if let Some(id) = app
            .world()
            .get_resource::<ExternalXWindow>()
            .map(|ew| ew.window_id)
        {
            info!("Checking for create window events to add ExternalXWindow");
            let world = app.world_mut().cell();
            let mut windows = world.get_resource_mut::<Windows>().unwrap();
            let create_window_events = world.get_resource::<Events<CreateWindow>>().unwrap();
            let mut window_created_events =
                world.get_resource_mut::<Events<WindowCreated>>().unwrap();
            let mut added = false;
            for create_window_event in ManualEventReader::default().iter(&create_window_events) {
                if create_window_event.id == id {
                    info!("Found matching event");
                    let descriptor = world
                        .get_resource::<WindowDescriptor>()
                        .as_deref()
                        .cloned()
                        .unwrap();
                    windows.add(Window::new(
                        id,
                        &descriptor,
                        descriptor.width as u32,
                        descriptor.height as u32,
                        1.0,
                        None,
                    ));
                    window_created_events.send(WindowCreated {
                        id: create_window_event.id,
                    });
                    added = true;
                } else {
                    warn!(
                        "Skipping non-xsecurlock window {:?}",
                        create_window_event.id
                    );
                }
            }
            if !added {
                warn!("Didn't find event for ExternalXWindow");
                let descriptor = world
                    .get_resource::<WindowDescriptor>()
                    .as_deref()
                    .cloned()
                    .unwrap();
                windows.add(Window::new(
                    id,
                    &descriptor,
                    descriptor.width as u32,
                    descriptor.height as u32,
                    1.0,
                    None,
                ));
                window_created_events.send(WindowCreated { id });
            }
        } else {
            info!("No ExternalXWindow, skipping");
        }
    }
}

struct RunnerPlugin;

impl Plugin for RunnerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        if app.world().get_resource::<ExternalXWindow>().is_some() {
            info!("Configuring XSecurelockRunner");

            app.set_runner(runner);
        } else {
            info!("Should use wgpu runner instead.");
        }
    }
}

fn runner(mut app: App) {
    let span = info_span!("XSecurelock Engine Runner");
    let _ = span.enter();

    info!("starting runner");
    sigint::init();
    while !sigint::received_sigint() {
        trace!("Doing one loop");
        app.update();
    }
    info!("Runner done (SIGINT)");
}
