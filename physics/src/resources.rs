use crate::time::Time;

pub(crate) fn add_default_resources(world: &mut ::specs::World) {
    world.add_resource(PhysicsDeltaTime::default());
    world.add_resource(PhysicsElapsed::default());
}

/// The amount of time elapsed in the physics simulation.
pub struct PhysicsDeltaTime(pub Time);

impl Default for PhysicsDeltaTime {
    fn default() -> Self { PhysicsDeltaTime(Time::milliseconds(16)) }
}

/// The total elapsed time in the physics simulation.
pub struct PhysicsElapsed {
    pub previous: Time,
    pub current: Time,
}

impl Default for PhysicsElapsed {
    fn default() -> Self { 
        PhysicsElapsed {
            current: Time::ZERO,
            previous: Time::ZERO,
        }
    }
}
