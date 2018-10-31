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

use sfml::graphics::{
    CircleShape,
    ConvexShape,
    Drawable,
    RectangleShape,
    RenderWindow,
    RenderTarget,
    Transformable,
};
use sfml::system::Vector2f;
use specs::{
    BitSet,
    Entities,
    Join,
    Read,
    ReaderId,
    ReadStorage,
    Resources,
    System,
    SystemData,
    Write,
    WriteStorage,
};
use specs::storage::{
    InsertedFlag,
    ModifiedFlag,
    RemovedFlag,
};

use engine::{
    components::{
        draw::{
            DrawColor,
            DrawLayer,
            DrawShape,
            ShapeType,
        },
        physics::Position,
    },
    resources::{
        draw::{
            CurrentDrawLayer,
            DrawLayers,
        },
        time,
        time::{
            Elapsed,
            PhysicsElapsed,
        },
    },
    systems::specialized::SpecializedSystem,
};

/// Updates draw layers.
#[derive(Default)]
pub(crate) struct DrawLayersUpdater {
    dirty: BitSet,
    removed_reader_id: Option<ReaderId<RemovedFlag>>,
    modified_reader_id: Option<ReaderId<ModifiedFlag>>,
    inserted_reader_id: Option<ReaderId<InsertedFlag>>,
}

impl<'a> System<'a> for DrawLayersUpdater {
    type SystemData = (
        Write<'a, DrawLayers>,
        ReadStorage<'a, DrawLayer>,
    );

    fn run(&mut self, (mut layer_masks, layers): Self::SystemData) {
        // If a remove flag is set, remove the id from all numbered masks and add it to the "none
        // set" mask.
        self.dirty.clear();
        layers.populate_removed(&mut self.removed_reader_id.as_mut().unwrap(), &mut self.dirty);
        for id in (&self.dirty).join() {
            for mask in &mut layer_masks.masks[..] {
                mask.remove(id);
            }
            layer_masks.any_mask.remove(id);
        }

        // If inserted or modified, remove from all masks and add to the current mask.
        self.dirty.clear();
        layers.populate_inserted(&mut self.inserted_reader_id.as_mut().unwrap(), &mut self.dirty);
        layers.populate_modified(&mut self.modified_reader_id.as_mut().unwrap(), &mut self.dirty);

        for (layer, id) in (&layers, &self.dirty).join() {
            for mask in &mut layer_masks.masks[..] {
                mask.remove(id);
            }
            layer_masks.masks[layer.layer() as usize].add(id);
            layer_masks.any_mask.add(id);
        }
    }

    fn setup(&mut self, res: &mut Resources) {
        Self::SystemData::setup(res);
        let mut storage: WriteStorage<DrawLayer> = SystemData::fetch(&res);
        self.removed_reader_id = Some(storage.track_removed());
        self.modified_reader_id = Some(storage.track_modified());
        self.inserted_reader_id = Some(storage.track_inserted());
    }
}

pub(crate) enum SfShape<'tex> {
    Rect(RectangleShape<'tex>),
    Circle(CircleShape<'tex>),
    Convex(ConvexShape<'tex>),
}

impl<'tex> SfShape<'tex> {
    /// Create an SfShape for a given DrawShape.
    fn create_for(shape: &DrawShape) -> Self {
        match shape.shape_type {
            ShapeType::Rect{size} => SfShape::Rect({
                let mut rect = RectangleShape::with_size(size);
                rect.set_origin(shape.origin);
                rect
            }),
            ShapeType::Circle{
                radius,
                point_count,
            } => SfShape::Circle({
                let mut circ = CircleShape::new(radius, point_count);
                circ.set_origin(shape.origin);
                circ
            }),
            ShapeType::Convex(ref points) => SfShape::Convex({
                let mut convex = ConvexShape::new(points.len() as u32);
                for (i, point) in points.iter().cloned().enumerate() {
                    convex.set_point(i as u32, point);
                }
                convex.set_origin(shape.origin);
                convex
            }),
        }
    }

    /// Update this shape if it's of the same type, or replace it with the correct type.
    fn update_for(&mut self, shape: &DrawShape) {
        match (self, &shape.shape_type) {
            (SfShape::Rect(rect), &ShapeType::Rect{size}) => {
               rect.set_size(size);
               rect.set_origin(shape.origin);
            },
            (SfShape::Circle(circ), &ShapeType::Circle{radius, point_count}) => {
                circ.set_radius(radius);
                circ.set_point_count(point_count);
                circ.set_origin(shape.origin);
            },
            (SfShape::Convex(convex), &ShapeType::Convex(ref points)) => {
                convex.set_point_count(points.len() as u32);
                for (i, point) in points.iter().cloned().enumerate() {
                    convex.set_point(i as u32, point);
                }
                convex.set_origin(shape.origin);
            },
            (this, _) => *this = SfShape::create_for(shape),
        }
    }

    /// Copy the given color properties to this shape.
    fn apply_color(&mut self, color: &DrawColor) {
        match self {
            SfShape::Rect(shape) => color.copy_to(shape),
            SfShape::Circle(shape) => color.copy_to(shape),
            SfShape::Convex(shape) => color.copy_to(shape),
        }
    }

    /// Sets the position of this drawable.
    fn apply_position(&mut self, pos: Vector2f, rot: f32) {
        match self {
            SfShape::Rect(shape) => SfShape::apply_position_to(shape, pos, rot),
            SfShape::Circle(shape) => SfShape::apply_position_to(shape, pos, rot),
            SfShape::Convex(shape) => SfShape::apply_position_to(shape, pos, rot),
        }
    }

    fn apply_position_to<T: Transformable>(trans: &mut T, pos: Vector2f, rot: f32) {
        trans.set_position(pos);
        trans.set_rotation(rot);
    }

    fn draw(&self, target: &mut RenderWindow) {
        match self {
            SfShape::Rect(shape) => SfShape::draw_to(shape, target),
            SfShape::Circle(shape) => SfShape::draw_to(shape, target),
            SfShape::Convex(shape) => SfShape::draw_to(shape, target),
        }
    }

    fn draw_to<D: Drawable>(drawable: &D, target: &mut RenderWindow) {
        target.draw(drawable);
    }
}

/// System to sync all SFML shapes from their corresponding components.
#[derive(Default)]
pub(crate) struct SyncDrawShapesSystem {
    dirty: BitSet,
    removed_shape_reader_id: Option<ReaderId<RemovedFlag>>,
    modified_shape_reader_id: Option<ReaderId<ModifiedFlag>>,
    inserted_shape_reader_id: Option<ReaderId<InsertedFlag>>,

    inserted_color_reader_id: Option<ReaderId<InsertedFlag>>,
    modified_color_reader_id: Option<ReaderId<ModifiedFlag>>,
}

impl<'a, 'b, 'tex> SpecializedSystem<'a, &'b mut Vec<Option<SfShape<'tex>>>> 
    for SyncDrawShapesSystem 
{
    type SystemData = (
        Entities<'a>,
        Read<'a, Elapsed>,
        Read<'a, PhysicsElapsed>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, DrawShape>,
        ReadStorage<'a, DrawColor>,
    );

    fn run_special(
        &mut self, 
        sf_shapes: &'b mut Vec<Option<SfShape<'tex>>>,
        (entities, elapsed, physics_elapsed, positions, shapes, colors): Self::SystemData,
    ) {
        self.dirty.clear();
        shapes.populate_removed(
            &mut self.removed_shape_reader_id.as_mut().unwrap(), 
            &mut self.dirty,
        );
        for id in (&self.dirty).join() {
            if (id as usize) < sf_shapes.len() {
                sf_shapes[id as usize] = None;
            }
        }

        self.dirty.clear();
        shapes.populate_inserted(
            &mut self.inserted_shape_reader_id.as_mut().unwrap(), 
            &mut self.dirty,
        );
        for (entity, shape, id) in (&*entities, &shapes, &self.dirty).join() {
            if (id as usize) >= sf_shapes.len() {
                let needed = ((id as usize) - sf_shapes.len()) + 1;
                sf_shapes.reserve(needed);
                sf_shapes.extend((0..needed).map(|_| None));
            }
            if sf_shapes[id as usize].is_none() {
                sf_shapes[id as usize] = Some(SfShape::create_for(shape));
            } else {
                sf_shapes[id as usize].as_mut().unwrap().update_for(shape);
            }
            if let Some(color) = colors.get(entity) {
                sf_shapes[id as usize].as_mut().unwrap().apply_color(color);
            }
        }

        self.dirty.clear();
        shapes.populate_modified(
            &mut self.modified_shape_reader_id.as_mut().unwrap(), 
            &mut self.dirty,
        );
        for (shape, id) in (&shapes, &self.dirty).join() {
            sf_shapes[id as usize].as_mut().unwrap().update_for(shape);
        }

        self.dirty.clear();
        colors.populate_inserted(
            &mut self.inserted_color_reader_id.as_mut().unwrap(),
            &mut self.dirty,
        );
        colors.populate_modified(
            &mut self.modified_color_reader_id.as_mut().unwrap(),
            &mut self.dirty,
        );
        for (_, color, id) in (&shapes, &colors, &self.dirty).join() {
            sf_shapes[id as usize].as_mut().unwrap().apply_color(color);
        }

        let factor = time::physics_interpolation_factor(&elapsed, &physics_elapsed);
        for (entity, _, position) in (&*entities, &shapes, &positions).join() {
            let (pos, rot) = position.interpolate(factor);
            let draw_pos = Vector2f::new(pos.x, -pos.y);
            let draw_rot = -rot.angle().to_degrees();
            sf_shapes[entity.id() as usize].as_mut().unwrap().apply_position(draw_pos, draw_rot);
        }
    }

    fn setup_special(&mut self, _: &'b mut Vec<Option<SfShape<'tex>>>, res: &mut Resources) {
        Self::SystemData::setup(res);
        let mut shape_storage: WriteStorage<DrawShape> = SystemData::fetch(&res);
        self.removed_shape_reader_id = Some(shape_storage.track_removed());
        self.modified_shape_reader_id = Some(shape_storage.track_modified());
        self.inserted_shape_reader_id = Some(shape_storage.track_inserted());

        let mut color_storage: WriteStorage<DrawColor> = SystemData::fetch(&res);
        self.modified_color_reader_id = Some(color_storage.track_modified());
        self.inserted_color_reader_id = Some(color_storage.track_inserted());
    }
}

#[derive(Default)]
pub(crate) struct DrawDrawShapesSystem;

impl<'a, 'b, 'tex> 
    SpecializedSystem<'a, (&'b mut RenderWindow, &'b mut Vec<Option<SfShape<'tex>>>)> 
    for DrawDrawShapesSystem
{
    type SystemData = (
        Read<'a, DrawLayers>,
        Read<'a, CurrentDrawLayer>,
        ReadStorage<'a, DrawShape>,
    );

    fn run_special(
        &mut self, 
        (window, sf_shapes): (&'b mut RenderWindow, &'b mut Vec<Option<SfShape>>),
        (layer_masks, current_draw_layer, shapes): Self::SystemData,
    ) {
        if current_draw_layer.layer() < DrawLayers::NUM_LAYERS {
            for (_, id) in (&shapes, &layer_masks.masks[current_draw_layer.layer()]).join() {
                sf_shapes[id as usize].as_mut().unwrap().draw(window);
            }
        } else {
            for (_, id) in (&shapes, !&layer_masks.any_mask).join() {
                sf_shapes[id as usize].as_mut().unwrap().draw(window);
            }
        }
    }
}
