// Copyright 2021 Google LLC
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

use sfml::graphics::{Color, Image, RectangleShape, RenderTarget, Shape, Texture, Transformable};
use sfml::system::{Clock, Time, Vector2f};

use xsecurelock_saver::simple::Screensaver;

/// Simple screensaver that shows a rotating rectangle over a rotating textured square.
struct RotatingRectScreensaver<'t> {
    /// Clock used to track time.
    clock: Clock,
    /// Last update time, used to compute dt using clock.
    tick: Time,
    /// Rectangle shape to draw. Since it has no textures, uses `'static`.
    rect: RectangleShape<'static>,
    /// Second rectangle, demonstrating use of texture with lifetime. The texture is allocated
    /// before `run_saver` so it will outlive the screensaver instance.
    tex_rect: RectangleShape<'t>,
}

impl<'t> RotatingRectScreensaver<'t> {
    /// Updates the previous tick time and computes delta time.
    fn update_dt(&mut self) -> Time {
        let prev = std::mem::replace(&mut self.tick, self.clock.elapsed_time());
        self.tick - prev
    }
}

impl<'t> Screensaver for RotatingRectScreensaver<'t> {
    fn update(&mut self) {
        let dt = self.update_dt().as_seconds();
        self.rect.rotate(50.0 * dt);
        self.tex_rect.rotate(-20.0 * dt);
    }

    fn draw<T: RenderTarget>(&self, target: &mut T) {
        target.clear(Color::BLACK);
        target.draw(&self.rect);
        target.draw(&self.tex_rect);
    }
}

fn main() {
    let mut img = Image::new(256, 256);
    for x in 0..256 {
        for y in 0..256 {
            img.set_pixel(x, y, Color::rgb(x as u8, 0, y as u8));
        }
    }
    let tex = Texture::from_image(&img).expect("Failed to create texture");

    // Closure can capture references that outlive the screensaver, allowing you to load textures in
    // `main` before starting the screensaver, and then reference them from the screensaver
    // instance.
    xsecurelock_saver::simple::run_saver(|screen_size| {
        let center = Vector2f::new(screen_size.x as f32 * 0.5, screen_size.y as f32 * 0.5);

        let mut rect = RectangleShape::with_size(Vector2f::new(500.0, 250.0));
        rect.set_fill_color(Color::GREEN);
        rect.set_outline_thickness(5.0);
        rect.set_outline_color(Color::rgb(0, 128, 0));
        rect.set_position(center);
        rect.set_origin(rect.size() * 0.5);

        let mut tex_rect = RectangleShape::with_texture(&tex);
        tex_rect.set_size((256.0, 256.0));
        tex_rect.set_position(center);
        tex_rect.set_origin(tex_rect.size() * 0.5);
        tex_rect.set_outline_thickness(2.0);
        tex_rect.set_outline_color(Color::MAGENTA);

        let clock = Clock::start();
        let tick = clock.elapsed_time();
        RotatingRectScreensaver {
            clock,
            tick,
            rect,
            tex_rect,
        }
    });
}
