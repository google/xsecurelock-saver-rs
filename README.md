# XSecureLock Saver

A library for creating 2D screensavers for XSecureLock in Rust, using
[SFML][sfml], and optionally [specs][specs] ECS.

[smfl]: https://www.sfml-dev.org/
[specs]: https://github.com/slide-rs/specs

This library has two modes, a DIY SFML mode (the default), and an
pseud-gameengine based on specs.

## Installing

### Dependencies

You'll need to install CSFML and SFML separately. They should be available from
your package manager on most linux distributions. Rust SFML requires version
2.4 of these dependencies.

Rust 1.27 is required.

Other dependencies should all be available from crates.io/fetched automatically
by cargo.

## Running

Once the SFML depencencies are installed, you should be able to run either of
the two example screensavers with cargo run.

XSecureLock provides the window handle for the screensaver to draw in in the
environment variable `XSCREENSAVER_WINDOW`, so if that environment variable is
set, the screensaver will try to draw into that window. Otherwise, it will open
a 1200x900 borderless window to run the screensaver in.

Note that the created window does not accept any input events, since it can't
accept input as a screensaver. This means that the window cannot be moved or
closed through the gui. Instead, just send sigterm (ctrl+c in the running
terminal) to close the screensaver (this is in line with the xsecurelock
specification).

## Use as a Screensaver

To use one of these or your own screensaver as the XSecureLock screensaver,
you'll need to copy the built binary to wherever XSecureLock looks for
screensavers (most likely somewhere in /usr/lib), and name it with the `saver_`
prefix. You can then specify the name of the screensaver by passing the
`XSECURELOCK_SAVER=saver_savername` environment variable to xsecurelock when
starting.

## Library Usage

### Plain SFML

To use the default mode, you just implement this trait:

```rust
/// A screensaver which can be run on an SFML RenderTarget.
pub trait Screensaver {
    /// Runs one "tick" in the screensaver, with the update happening at the specified time.
    fn update(&mut self);

    /// Draw the screensaver on the specified target.
    fn draw<T>(&self, target: &mut T) where T: RenderTarget;
}
```

And run:

```rust
fn main() {
    xsecurelock_saver::run_saver(|screen_size| MyScreenSaver::new(screen_size));
}
```
### Game-Engine-Like

To use the game-engine like features, you'll need to enable the feature
`engine`. You should also familiarize yourself with ECS in general and specs in
particular; the [specs book][specs-book] has an overview.

[specs-book]: https://slide-rs.github.io/specs/

The engine runs two distinct sets of systems on a common set of components.
There are general update systems, which are run once per frame just before
drawing, and physics update systems which are run on fixed intervals. The main
difference is that the time delta between updates can vary, while fixed updates
keep a consistent time interval, so they are more suitable for physics
calculations. (The engine provides only a very limited set of builtin physics
components and systems -- currently only a sympletic euler integrator over
force, velocity, and position).

To run the engine, use the engine builder to set up components, resources, and
update and physics update systems, then build some entities and run.

```rust
fn main() {
    // Configure Components and Systems:
    let mut engine = EngineBuilder::new()
        .with_resource(MyResource::default())
        .with_component::<MyAdditionalComponent>()
        .with_update_sys(MyAdditionalSystem::new(), "", &[])
        .with_physics_update_sys(MyAdditionalPhysicsSystem::new(), "", &[])
        .build();

    // Add some entities.
    engine.create_entity()
        .with(MyAdditionalComponent::new())
        .build();
    engine.create_entity()
        .with(MyAdditionalComponent::new())
        .build();

    // Run the main loop.
    engine.run();
}
```

#### Builtin Component Types

* Drawing:
  * `DrawPosition` -- The screen position and rotation where the object will be
    drawn. Copied to the corresponding SFML object each frame automatically.
    Since this is the SFML position in screen coordinates, positive y values are
    down. If the object also contains a Phsyics Position, this will
    automatically be set each frame by interpolating the position between two
    physics ticks. This is done to allow smooth motion even though the physics
    tick rate may be slower than the framerate.
  * `DrawLayer` -- Controls the depth at which an entity will be drawn. Entities
    are drawn in passes, with entities at the same depth drawn in an arbitrary
    order, so setting draw layers is the only way to positively control the
    depth at which entities are drawn. Entities with no draw layer are drawn on
    top of all others.
  * `DrawColor` -- The `Color` for a drawable's fill and outline colors, as well
    as outline thickness.
  * `DrawShape` -- A `ShapeType`, describing one of the supported SFML shapes
    (automatically synchronized by the builtin draw system), and an origin,
    which controls the offset that identifies the center of the shape.
* Physics:
  * `Position` -- The position and rotation of the object for physics purposes.
    Tracks the current and previous position to allow interpolation.
  * `Velocity` -- Linear and agular velocity of the object. Integrator adds this
    to the physics position each physics update.
  * `Mass` -- Linear inertia and moment of inertia of this object. Optional for
    the integrator, defaults to `1.0` for each if not present.
  * `ForceAccumulator` -- Linear and angular forces accumulated during a physics
    update. Added to velocity each physics update, then cleared to zero before
    the next update.

#### Builtin Resources

* Drawing:
  * `View` -- The view rectangle corresponding to the screen. By default this
    has the aspect ratio of the screen. Can be updated to change what part of
    the world appears on the screen. Automatically copied to the SFML render
    window's view when drawing.
* Time:
  * `DeltaTime` -- Only meaningful during normal updates (not physics); the
    amount of Time since the last update.
  * `Elapsed` -- The elapsed time since the engine was started, both of the
    current and previous frame.
  * `PhysicsDeltaTime` -- Only meaningful during physics updates; the amount of
    time since the last physics update (this value is constant).
  * `PhysicsElapsed` -- The elapsed time since the engine was started, both of
    the current and previous physics update (the difference between these two
    should always be the same as the constant fixed delta time).

#### Builtin Systems

Most builtin systems (e.g. drawing related) are enabled by default. Some of the
physics systems are not. Systems to clear force accumulators and update the
position interpolators are, but the integrator is not. The integrator consists
of two systems:

* `SympleticEulerForceStep` -- integrates the force, changing the velocity.
* `SympleticEulerVelocityStep` -- integrates the velocity, changing the
  position.

These systems are made public and not installed by default so that clients can
mix other code with them, e.g. collision calculation. If collision detection
and handling is added to the engine core these will likely be privatized again.

**NOTE: in order for integration to *actually* be sympletic, the force step
*must* come before the velocity step**


#### Limitations/Limitations that I'm Interested in Fixing Eventually

There is currently no support for using textures on drawables, due to
lifetimes/ownership used by Rust-SFML. Likely solution is to have a pool of
textures on the Engine and use non-lifetime-restricted handles to them from a
new DrawTexture component. This isn't required for `saver_spacesim`, so it's not
a priority.

# License

The code is released unser the Apache 2.0 license. See the LICENSE file for more
details.

This project is not an official Google project. It is not supported by Google
and Google specifically disclaims all warranties as to its quality,
merchantability, or fitness for a particular purpose.
