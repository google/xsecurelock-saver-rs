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

//! Model of the start-state of the world. Identifies a unique world.
use std::f32;

use xsecurelock_saver::engine::components::physics::Vector;

#[derive(Debug)]
pub struct Scenario {
    /// The name of this scenario.
    pub id: i64,
    /// The family of this scenario. This is the ID of the root of its family tree.
    pub family: i64,
    /// Optional parent of this scenario. The parent scenario may have been pruned. This is None if
    /// the scenario is a root, but is retained even if the parent is pruned.
    pub parent: Option<i64>,
    /// The generation number of this scenario. Useful in case any of the parents have been pruned.
    pub generation: i64,
    /// The state of the world at the start of the scenario.
    pub world: World,
    /// The score that this world earned when tested.
    pub score: f64,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct World {
    pub planets: Vec<Planet>,
}

impl World {
    /// Combines overlapping planets into a single, larger planet.
    pub fn merge_overlapping_planets(&mut self) {
        loop {
            // Stop looping when we haven't merged any more planets.
            let mut clean = true;

            let mut left = 0;
            while left < self.planets.len() - 1 {
                let mut right = left + 1;
                while right < self.planets.len() {
                    let total_radius = self.planets[left].radius() + self.planets[right].radius();
                    let total_radius_sqr = total_radius * total_radius;
                    let dist_sqr = (self.planets[left].position - self.planets[right].position)
                        .norm_squared();
                    if dist_sqr < total_radius_sqr {
                        clean = false;
                        self.merge_planets(left, right);
                    } else {
                        right += 1;
                    }
                }
                left += 1;
            }

            if clean {
                break;
            }
        }
    }

    /// Helper function to merge two planets with specified indexes. Combines the right planet into
    /// the left, then removes the right planet.
    fn merge_planets(&mut self, left: usize, right: usize) {
        assert!(left < right);
        assert!(right < self.planets.len());
        {
            let (left_sub, right_sub) = self.planets.split_at_mut(right);
            left_sub[left].merge(&right_sub[0]);
        }
        self.planets.remove(right);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Planet {
    pub position: Vector,
    pub velocity: Vector,
    pub mass: f32,
}

const PLANET_DENSITY: f32 = 0.1;

impl Planet {
    /// Calculates the radius for a planet of the given mass.
    pub fn radius_from_mass(mass: f32) -> f32 {
        // Calculate radius as if this planet were a sphere with the given mass and density:
        // V = 4/3 * pi * r^3
        // M = V * D
        // M = 4/3 * pi * r^3 * D
        // 3M / (4 * pi * D) = r^3
        (3. * mass / (4. * f32::consts::PI * PLANET_DENSITY)).cbrt()
    }

    /// Calculates the radius of this planet.
    pub fn radius(&self) -> f32 {
        Self::radius_from_mass(self.mass)
    }

    /// Updates the mass so the planet has the given radius.
    #[allow(dead_code)]
    pub fn set_radius(&mut self, radius: f32) {
        // V = 4/3 * pi * r^3
        // M = V * D
        // M = 4/3 * pi * r^3 * D
        self.mass = 4./3. * f32::consts::PI * radius.powi(3) * PLANET_DENSITY;
    }

    /// Merges the given other planet into this one.
    fn merge(&mut self, other: &Planet) {
        let total_mass = self.mass + other.mass;
        // multiplying by mass may give less precision, maybe? So pre-calculate multiplication
        // factors.
        let self_factor = self.mass / total_mass;
        let other_factor = other.mass / total_mass;
        // The center of mass.
        let net_position = self.position * self_factor + other.position * other_factor;
        // Equivalent to calculating total momentum and dividing by mass.
        let net_velocity = self.velocity * self_factor + other.velocity * other_factor;
        self.position = net_position;
        self.velocity = net_velocity;
        self.mass = total_mass;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod planet_tests {
        use super::*;
        #[test]
        fn test_merge_simple() {
            let mut left = Planet {
                position: Vector::new(0., 0.),
                velocity: Vector::new(0., 0.),
                mass: 1.,
            };
            let right = Planet {
                position: Vector::new(1., 0.),
                velocity: Vector::new(0., 0.),
                mass: 1.,
            };
            let expected = Planet {
                position: Vector::new(0.5, 0.),
                velocity: Vector::new(0., 0.),
                mass: 2.,
            };
            left.merge(&right);
            assert_eq!(left, expected);
        }

        #[test]
        fn test_merge_moving() {
            let mut left = Planet {
                position: Vector::new(1., -5.),
                velocity: Vector::new(3., 6.),
                mass: 8.,
            };
            let right = Planet {
                position: Vector::new(-9., 2.),
                velocity: Vector::new(-7.,-2.),
                mass: 24.,
            };
            let expected = Planet {
                position: Vector::new(-6.5, 0.25),
                velocity: Vector::new(-4.5, 0.),
                mass: 32.,
            };
            left.merge(&right);
            assert_eq!(left, expected);
        }

        #[test]
        fn test_merge_moving_order_independent() {
            let mut left = Planet {
                position: Vector::new(-9., 2.),
                velocity: Vector::new(-7.,-2.),
                mass: 24.,
            };
            let right = Planet {
                position: Vector::new(1., -5.),
                velocity: Vector::new(3., 6.),
                mass: 8.,
            };
            let expected = Planet {
                position: Vector::new(-6.5, 0.25),
                velocity: Vector::new(-4.5, 0.),
                mass: 32.,
            };
            left.merge(&right);
            assert_eq!(left, expected);
        }

    }

    mod world_tests {
        use super::*;

        #[test]
        fn test_merge_planets_simple() {
            let mut world = World { planets: vec![
                Planet {
                    position: Vector::new(0., 0.),
                    velocity: Vector::new(0., 0.),
                    mass: 1.,
                },
                Planet {
                    position: Vector::new(1., -5.),
                    velocity: Vector::new(3., 6.),
                    mass: 8.,
                },
                Planet {
                    position: Vector::new(1., 0.),
                    velocity: Vector::new(0., 0.),
                    mass: 1.,
                },
                Planet {
                    position: Vector::new(-9., 2.),
                    velocity: Vector::new(-7.,-2.),
                    mass: 24.,
                },
            ]};
            let expected = World { planets: vec![
                Planet {
                    position: Vector::new(0., 0.),
                    velocity: Vector::new(0., 0.),
                    mass: 1.,
                },
                Planet {
                    position: Vector::new(-6.5, 0.25),
                    velocity: Vector::new(-4.5, 0.),
                    mass: 32.,
                },
                Planet {
                    position: Vector::new(1., 0.),
                    velocity: Vector::new(0., 0.),
                    mass: 1.,
                },
            ]};
            world.merge_planets(1, 3);
            assert_eq!(world, expected);
        }


        #[test]
        fn test_merge_overlapping_simple() {
            let mut world = World { planets: vec![
                Planet {
                    position: Vector::new(0., 0.),
                    velocity: Vector::new(0., 0.),
                    mass: 1.,
                },
                Planet {
                    position: Vector::new(2., -10.),
                    velocity: Vector::new(3., 6.),
                    mass: 8.,
                },
                Planet {
                    position: Vector::new(5., 5.),
                    velocity: Vector::new(0., 0.),
                    mass: 1.,
                },
                Planet {
                    position: Vector::new(-2., -12.),
                    velocity: Vector::new(-7.,-2.),
                    mass: 24.,
                },
            ]};
            let expected = World { planets: vec![
                Planet {
                    position: Vector::new(0., 0.),
                    velocity: Vector::new(0., 0.),
                    mass: 1.,
                },
                Planet {
                    position: Vector::new(-1., -11.5),
                    velocity: Vector::new(-4.5, 0.),
                    mass: 32.,
                },
                Planet {
                    position: Vector::new(5., 5.),
                    velocity: Vector::new(0., 0.),
                    mass: 1.,
                },
            ]};
            world.merge_overlapping_planets();
            assert_eq!(world, expected);
        }
    }
}
