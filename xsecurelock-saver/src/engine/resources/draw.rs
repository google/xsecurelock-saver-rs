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

use sfml::graphics::View as SfView;
use sfml::system::Vector2f;
use specs::BitSet;

pub(crate) fn add_default_resources(world: &mut ::specs::World) {
    world.add_resource(DrawLayers::default());
    world.add_resource(CurrentDrawLayer::default());
    world.add_resource(View::default());
}

/// Internal tracking of bitsets for which entities are on which layers.
#[derive(Default)]
pub(crate) struct DrawLayers {
    /// Masks for each layer.
    pub(crate) masks: [BitSet; DrawLayers::NUM_LAYERS],
    /// Mask of values set in any layer.
    pub(crate) any_mask: BitSet,
}

impl DrawLayers {
    /// Number of total layer masks.
    pub(crate) const NUM_LAYERS: usize = 10;
}

/// Which draw layer to read from for the current draw.
#[derive(Default)]
pub(crate) struct CurrentDrawLayer(usize);

impl CurrentDrawLayer {
    /// Get the current draw layer. Guaranteed to be <= DrawLayers::NUM_LAYERS.
    #[inline]
    pub(crate) fn layer(&self) -> usize { self.0 }

    /// Set the current draw layer. Must be <= DrawLayers::NUM_LAYERS.
    #[inline]
    pub(crate) fn set_layer(&mut self, layer: usize) {
        assert!(layer <= DrawLayers::NUM_LAYERS, "layer out of range");
        self.0 = layer;
    }
}

pub struct View {
    pub center: Vector2f,
    pub size: Vector2f,
    pub rotation: f32,
}

impl Default for View {
    fn default() -> Self { 
        Self {
            center: Vector2f::new(0., 0.),
            size: Vector2f::new(1000., 1000.),
            rotation: 0.,
        }
    }
}

impl View {
    pub(crate) fn copy_to(&self, view: &mut SfView) {
        view.set_center(self.center);
        view.set_size(self.size);
        view.set_rotation(self.rotation);
    }
}
