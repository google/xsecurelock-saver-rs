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
    Color,
    Shape,
};
use sfml::system::Vector2f;
use specs::{
    Component,
    FlaggedStorage,
    VecStorage,
    World,
};

use crate::engine::resources::draw::DrawLayers;

pub(crate) fn register_all(world: &mut World) {
    world.register::<DrawLayer>();
    world.register::<DrawColor>();
    world.register::<DrawShape>();
}

/// What layer to draw an object on.
#[derive(Debug)]
pub struct DrawLayer(u8);

impl Component for DrawLayer {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

impl DrawLayer {
    /// Set the layer. Must be less than `NUM_LAYERS`.
    #[inline]
    pub fn set_layer(&mut self, layer: u8) {
        assert!((layer as usize) < DrawLayers::NUM_LAYERS, "layer out of range");
        self.0 = layer;
    }

    /// Get the current draw layer.
    #[inline]
    pub fn layer(&self) -> u8 { self.0 }
}

/// Properties for basic shape renderers.
#[derive(Debug)]
pub struct DrawColor {
    pub fill_color: Color,
    pub outline_color: Color,
    pub outline_thickness: f32,
}

impl Component for DrawColor {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

impl DrawColor {
    pub(crate) fn copy_to<'a, T>(&self, shape: &mut T) where T: Shape<'a> {
        shape.set_fill_color(self.fill_color);
        shape.set_outline_color(self.outline_color);
        shape.set_outline_thickness(self.outline_thickness);
    }
}

/// A drawable shape.
#[derive(Clone)]
pub struct DrawShape {
    pub shape_type: ShapeType,
    pub origin: Vector2f,
}

/// Properties specific to particular types of shapes.
#[derive(Clone)]
pub enum ShapeType {
    Rect {
        size: Vector2f
    },
    Circle {
        radius: f32,
        point_count: u32,
    },
    Convex(Vec<Vector2f>),
}

impl Component for DrawShape {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}
