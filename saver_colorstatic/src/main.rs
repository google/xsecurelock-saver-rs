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

extern crate rand;
extern crate sfml;
extern crate xsecurelock_saver;

use sfml::graphics::{
    Color,
    Image,
    RenderTarget,
    Texture,
    Sprite,
};
use sfml::system::Vector2u;

use xsecurelock_saver::Screensaver;

struct StaticScreensaver {
    img: Image,
}

impl Screensaver for StaticScreensaver {
    fn update(&mut self) {
        for Vector2u{x, y} in row_major_iterator(self.img.size()) {
            self.img.set_pixel(x, y, Color::rgb(rand::random(), rand::random(), rand::random()));
        }
    }

    fn draw<T: RenderTarget>(&self, target: &mut T) {
        let tex = Texture::from_image(&self.img).unwrap();
        let sprite = Sprite::with_texture(&tex);

        target.draw(&sprite);
    }
}

fn main() {
    xsecurelock_saver::run_saver(
        |screen_size| StaticScreensaver {
            img: Image::new(screen_size.x, screen_size.y),
        });
}

fn row_major_iterator<V>(size: V) -> impl Iterator<Item=Vector2u> where V: Into<Vector2u> {
    let Vector2u { x: width, y: height } = size.into();
    (0..height).flat_map(move |y| (0..width).map(move |x| Vector2u { x: x, y: y }))
}
