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

//! A module providing an engine for game-like screensavers.
use std::env;

use bevy::app::{Events, ManualEventReader, PluginGroupBuilder};
use bevy::prelude::*;
use bevy::wgpu::WgpuPlugin;
use bevy::window::{CreateWindow, WindowCreated, WindowPlugin};
use bevy::winit::WinitPlugin;
use bevy_wgpu_xsecurelock::ExternalXWindow;
use log::trace;

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
    info!("starting runner");
    sigint::init();
    while !sigint::received_sigint() {
        trace!("Doing one loop");
        app.update();
    }
    info!("Runner done (SIGINT)");
}

// use std::sync::Arc;
//
// use rayon::ThreadPoolBuilder;
// use sfml::graphics::{
//     Color,
//     RenderTarget,
//     RenderWindow,
//     View as SfView,
// };
// use sfml::system::{Clock, SfBox, Time, Vector2f};
// use specs::{Component, System};
// use shred::Resource;
//
// use physics::{
//     self,
//     resources::{PhysicsDeltaTime, PhysicsElapsed},
//     systems::{ClearForceAccumulators, SetupNextPhysicsPosition},
// };
// use scene_management::{
//     self,
//     resources::{SceneChange, SceneLoader},
//     SceneChangeHandler,
//     SceneChangeHandlerBuilder,
//     systems::DeleteSystem,
// };
//
// use self::{
//     resources::{
//         draw::{
//             CurrentDrawLayer,
//             DrawLayers,
//             View,
//         },
//         time::{
//             DeltaTime,
//             Elapsed,
//         },
//     },
//     systems::{
//         draw::{
//             DrawLayersUpdater,
//             SyncDrawShapesSystem,
//             DrawDrawShapesSystem,
//             SfShape,
//         },
//         specialized::{
//             SpecializedSystem,
//             SpecializedSystemObject,
//         },
//     },
// };
//
// pub mod components;
// pub mod resources;
// pub mod systems;
//
// /// Builds an engine.
// pub struct EngineBuilder<'a, 'b> {
//     world: ::specs::World,
//     update_dispatcher: ::specs::DispatcherBuilder<'a, 'b>,
//     scene_change_handler: SceneChangeHandlerBuilder<'a, 'b>,
//     physics_update_dispatcher: ::specs::DispatcherBuilder<'a, 'b>,
//     max_physics_updates: usize
// }
//
// impl<'a, 'b> EngineBuilder<'a, 'b> {
//     pub fn new() -> Self {
//         let thread_pool = Arc::new(ThreadPoolBuilder::new().build().unwrap());
//         Self {
//             world: {
//                 let mut world = ::specs::World::new();
//                 physics::register(&mut world);
//                 scene_management::register(&mut world);
//                 components::register_all(&mut world);
//                 resources::add_default_resources(&mut world);
//                 world
//             },
//             update_dispatcher: ::specs::DispatcherBuilder::new()
//                 .with_pool(Arc::clone(&thread_pool)),
//             scene_change_handler: SceneChangeHandlerBuilder::new()
//                 .with_threadpool(Arc::clone(&thread_pool)),
//             physics_update_dispatcher: ::specs::DispatcherBuilder::new()
//                 .with_pool(thread_pool)
//                 .with(SetupNextPhysicsPosition, "", &[])
//                 .with(ClearForceAccumulators, "", &[])
//                 .with_barrier(),
//             max_physics_updates: 5,
//         }
//     }
//
//     /// Add a system to run on updates. Arguments are the same as for `DispatcherBuilder.with`.
//     /// Dependencies can only refer to other update systems added previously.
//     pub fn with_update_sys<S>(mut self, sys: S, name: &str, dep: &[&str]) -> Self
//         where S: for<'c> System<'c> + Send + 'a
//     {
//         self.add_update_sys(sys, name, dep);
//         self
//     }
//
//     /// Add a system to run on updates. Arguments are the same as for `DispatcherBuilder.with`.
//     /// Dependencies can only refer to other update systems added previously.
//     pub fn add_update_sys<S>(&mut self, sys: S, name: &str, dep: &[&str])
//         where S: for<'c> System<'c> + Send + 'a
//     {
//         self.update_dispatcher.add(sys, name, dep);
//     }
//
//     /// Add a barrier to the update system.
//     pub fn with_update_barrier(mut self) -> Self { self.add_update_barrier(); self }
//
//     /// Add a barrier to the update system.
//     pub fn add_update_barrier(&mut self) { self.update_dispatcher.add_barrier(); }
//
//     /// Add a system to run on scene_changes. Arguments are the same as for
//     /// `DispatcherBuilder.with`.  Dependencies can only refer to other scene_change systems added
//     /// previously. This system will be run only when a scene change has been scheduled and can be
//     /// used to clean up and reset resources or entities for the new scene.
//     pub fn with_scene_change_sys<S>(mut self, sys: S, name: &str, dep: &[&str]) -> Self
//         where S: for<'c> System<'c> + Send + 'a
//     {
//         self.add_scene_change_sys(sys, name, dep);
//         self
//     }
//
//     /// Add a system to run on scene_changes. Arguments are the same as for
//     /// `DispatcherBuilder.with`.  Dependencies can only refer to other scene_change systems added
//     /// previously. This system will be run only when a scene change has been scheduled and can be
//     /// used to clean up and reset resources or entities for the new scene.
//     pub fn add_scene_change_sys<S>(&mut self, sys: S, name: &str, dep: &[&str])
//         where S: for<'c> System<'c> + Send + 'a
//     {
//         self.scene_change_handler.add_pre_load_sys(sys, name, dep);
//     }
//
//     /// Add a barrier to the scene_change system.
//     pub fn with_scene_change_barrier(mut self) -> Self { self.add_scene_change_barrier(); self }
//
//     /// Add a barrier to the scene_change system.
//     pub fn add_scene_change_barrier(&mut self) {
//         self.scene_change_handler.add_pre_load_barrier();
//     }
//
//     /// Add a system to run on physics updates. Arguments are the same as for
//     /// `DispatcherBuilder.with`. Dependencies can only refer to other physics update systems
//     /// added previously.
//     pub fn with_physics_update_sys<S>(mut self, sys: S, name: &str, dep: &[&str]) -> Self
//         where S: for<'c> System<'c> + Send + 'a
//     {
//         self.add_physics_update_sys(sys, name, dep);
//         self
//     }
//
//     /// Add a system to run on physics updates. Arguments are the same as for
//     /// `DispatcherBuilder.with`. Dependencies can only refer to other physics update systems
//     /// added previously.
//     pub fn add_physics_update_sys<S>(&mut self, sys: S, name: &str, dep: &[&str])
//         where S: for<'c> System<'c> + Send + 'a
//     {
//         self.physics_update_dispatcher.add(sys, name, dep);
//     }
//
//     /// Add a barrier to the physics update system.
//     pub fn with_physics_update_barrier(mut self) -> Self {
//         self.add_physics_update_barrier();
//         self
//     }
//
//     /// Add a barrier to the physics update system.
//     pub fn add_physics_update_barrier(&mut self) { self.physics_update_dispatcher.add_barrier(); }
//
//     /// Register a component for the world.
//     pub fn with_component<C: Component>(mut self) -> Self
//         where <C as Component>::Storage: Default
//     {
//         self.add_component::<C>();
//         self
//     }
//
//     /// Register a component for the world.
//     pub fn add_component<C: Component>(&mut self) where <C as Component>::Storage: Default {
//         self.world.register::<C>();
//     }
//
//     /// Add a resource to the world.
//     pub fn with_resource<T: Resource>(mut self, res: T) -> Self { self.add_resource(res); self }
//
//     /// Add a resource to the world.
//     pub fn add_resource<T: Resource>(&mut self, res: T) { self.world.add_resource(res); }
//
//     /// Set the initial scene-loader, used on the first frame.
//     pub fn with_initial_sceneloader<T>(mut self, loader: T) -> Self
//         where T: for<'l> SceneLoader<'l> + Send + Sync + 'static
//     {
//         self.set_initial_sceneloader(loader);
//         self
//     }
//
//     /// Set the initial scene-loader, used on the first frame.
//     pub fn set_initial_sceneloader<T>(&mut self, loader: T)
//         where T: for<'l> SceneLoader<'l> + Send + Sync + 'static
//     {
//         self.world.write_resource::<SceneChange>().change_scene(loader);
//     }
//
//     /// Builds the engine and creates the render window.
//     pub fn build<'tex>(self) -> Engine<'a, 'b, 'tex> {
//         let mut engine = Engine {
//             world: self.world,
//             update_dispatcher: self.update_dispatcher
//                 .with_barrier()
//                 .with(DrawLayersUpdater::default(), "", &[])
//                 .with(DeleteSystem, "", &[])
//                 .build(),
//             scene_change_handler: self.scene_change_handler.build(),
//             physics_update_dispatcher: self.physics_update_dispatcher
//                 .with_barrier()
//                 .with(DeleteSystem, "", &[])
//                 .build(),
//
//             window: super::open_window(),
//             view: SfView::new(Vector2f::new(0., 0.), Vector2f::new(1., 1.)),
//
//             clock: Clock::start(),
//
//             max_physics_updates: self.max_physics_updates,
//
//             draw_shapes: Vec::new(),
//             sync_draw_shapes: Default::default(),
//             draw_draw_shapes: Default::default(),
//         };
//
//         {
//             let mut view = engine.world.write_resource::<View>();
//             let win_sz = engine.window.size();
//             let ratio = win_sz.x as f32 / win_sz.y as f32;
//             view.size.y = 2000.;
//             view.size.x = ratio * view.size.y;
//             view.copy_to(&mut engine.view);
//         }
//
//         engine.update_dispatcher.setup(&mut engine.world.res);
//         engine.scene_change_handler.setup(&mut engine.world);
//         engine.physics_update_dispatcher.setup(&mut engine.world.res);
//
//         engine.sync_draw_shapes.setup_special(&mut engine.draw_shapes, &mut engine.world.res);
//         engine.draw_draw_shapes.setup_special(
//             (&mut engine.window, &mut engine.draw_shapes), &mut engine.world.res);
//
//         engine
//     }
// }
//
// /// A Game-Engine like Screensaver implementation.
// pub struct Engine<'a, 'b, 'tex> {
//     world: ::specs::World,
//     update_dispatcher: ::specs::Dispatcher<'a, 'b>,
//     scene_change_handler: SceneChangeHandler<'a, 'b>,
//     physics_update_dispatcher: ::specs::Dispatcher<'a, 'b>,
//
//     window: RenderWindow,
//     view: SfBox<SfView>,
//
//     clock: Clock,
//
//     max_physics_updates: usize,
//
//     draw_shapes: Vec<Option<SfShape<'tex>>>,
//     sync_draw_shapes: SyncDrawShapesSystem,
//     draw_draw_shapes: DrawDrawShapesSystem,
// }
//
// impl<'a, 'b, 'tex> Engine<'a, 'b, 'tex> {
//     /// Create an entity in the world.
//     pub fn create_entity(&mut self) -> ::specs::world::EntityBuilder {
//         self.world.create_entity()
//     }
//
//     /// Runs the game loop.
//     pub fn run(mut self) {
//         sigint::init();
//
//         self.clock.restart();
//         {
//             let start = self.clock.elapsed_time();
//             let mut physt = self.world.write_resource::<PhysicsElapsed>();
//             physt.current = start;
//             physt.previous = start - self.world.read_resource::<PhysicsDeltaTime>().0;
//             let mut dt = self.world.write_resource::<DeltaTime>();
//             dt.0 = Time::milliseconds(5);
//             let mut t = self.world.write_resource::<Elapsed>();
//             t.current = start;
//             t.previous = start - dt.0;
//         }
//         while !sigint::received_sigint() {
//             let now = self.clock.elapsed_time();
//             self.maybe_physics_update(now);
//             self.update(now);
//             self.draw();
//         }
//     }
//
//     /// Run a fixed update if enough time has elapsed.
//     fn maybe_physics_update(&mut self, now: Time) {
//         for _ in 0..self.max_physics_updates {
//             {
//                 let mut physt = self.world.write_resource::<PhysicsElapsed>();
//                 if physt.current >= now {
//                     return;
//                 }
//                 let physdt = self.world.read_resource::<PhysicsDeltaTime>();
//                 physt.previous = physt.current;
//                 physt.current += physdt.0;
//             }
//             self.physics_update_dispatcher.dispatch(&self.world.res);
//             self.world.maintain();
//
//             self.scene_change_handler.handle_scene_change(&mut self.world);
//         }
//         // if we run out of iterations trying to catch up physics, we're pretty far behind -- just
//         // run up the clock and let physics stutter.
//         let mut phys = self.world.write_resource::<PhysicsElapsed>();
//         let dphys = self.world.read_resource::<PhysicsDeltaTime>();
//         while phys.current < now {
//             phys.previous = phys.current;
//             phys.current += dphys.0;
//         }
//
//     }
//
//     /// Run a normal update.
//     fn update(&mut self, now: Time) {
//         {
//             let mut elapsed = self.world.write_resource::<Elapsed>();
//             elapsed.previous = elapsed.current;
//             elapsed.current = now;
//             let mut delta = self.world.write_resource::<DeltaTime>();
//             delta.0 = elapsed.current - elapsed.previous;
//         }
//         self.update_dispatcher.dispatch(&self.world.res);
//         self.world.maintain();
//
//         self.scene_change_handler.handle_scene_change(&mut self.world);
//     }
//
//     /// Redraw.
//     fn draw(&mut self) {
//         self.sync_draw_shapes.run(&mut self.draw_shapes, &self.world.res);
//
//
//         self.world.write_resource::<View>().copy_to(&mut self.view);
//
//         self.window.clear(Color::BLACK);
//         self.window.set_view(&self.view);
//         for layer in 0..=DrawLayers::NUM_LAYERS {
//             self.world.write_resource::<CurrentDrawLayer>().set_layer(layer);
//             self.draw_draw_shapes.run((&mut self.window, &mut self.draw_shapes), &self.world.res);
//         }
//         self.window.display();
//     }
// }
