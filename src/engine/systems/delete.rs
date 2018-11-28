use specs::{
    Entities,
    Join,
    ReadStorage,
    System,
};

use engine::components::delete::Deleted;

pub(crate) struct DeleteSystem;
impl<'a> System<'a> for DeleteSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Deleted>,
    );

    fn run(&mut self, (entities, deleted): Self::SystemData) {
        for (ent, _) in (&*entities, &deleted).join() {
            entities.delete(ent).unwrap();
        }
    }
}
