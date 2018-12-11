pub mod components;
pub mod systems;
pub mod resources;
pub mod time;

/// Register all components and default resources.
pub fn register(world: &mut ::specs::World) {
    components::register_all(world);
    resources::add_default_resources(world);
}
