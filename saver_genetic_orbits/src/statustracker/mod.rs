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

use std::mem;
use std::str::FromStr;

use bevy::ecs::component::Component;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use serde::{Deserialize, Serialize};

use crate::config::scoring::ScoringConfig;
use crate::model::{Scenario, World};
use crate::storage::sqlite::SqliteStorage;
use crate::storage::Storage;
use crate::world::Planet;
use crate::SaverState;

use self::scoring_function::Expression;

mod scoring_function;

pub struct ScoringPlugin;

impl Plugin for ScoringPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<ActiveWorld>()
            .add_startup_system(setup.system())
            .add_system_set(
                SystemSet::on_enter(SaverState::Run)
                    .with_system(parent_text.system())
                    .with_system(parent_score_text.system())
                    .with_system(generation_text.system())
                    .with_system(family_text.system())
                    .with_system(high_score_text::<SqliteStorage>.system()),
            )
            .add_system_set(
                SystemSet::on_update(SaverState::Run)
                    .with_system(score.system().label("compute-score"))
                    .with_system(score_text.system().after("compute-score"))
                    .with_system(time_left_text.system().after("compute-score")),
            )
            .add_system_set(
                SystemSet::on_exit(SaverState::Run)
                    .with_system(store_result::<SqliteStorage>.system()),
            );
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct ScoringFunction(Expression);

impl ScoringFunction {
    /// Evaluate the expression given the scoring function inputs.
    pub fn eval(&self, elapsed_fract: f64, total_mass: f64, mass_count: f64) -> f64 {
        self.0.eval(elapsed_fract, total_mass, mass_count)
    }
}

impl FromStr for ScoringFunction {
    type Err = String;

    fn from_str(source: &str) -> Result<ScoringFunction, String> {
        source.parse().map(ScoringFunction)
    }
}

/// Resource for tracking the status of the currently active scene.
pub struct ActiveWorld {
    /// The world being scored.
    pub world: World,
    /// The parent of the world being scored.
    pub parent: Option<Scenario>,
    /// The score the world has received so far.
    pub cumulative_score: f64,
    /// The number of physics ticks that the world has been scored on so far.
    pub timer: Timer,
}

impl ActiveWorld {
    /// Reset the active world for a new scenario.
    pub fn start(&mut self, world: World, parent: Option<Scenario>) {
        self.world = world;
        self.parent = parent;
        self.cumulative_score = 0.0;
        self.timer.reset();
    }
}

impl FromWorld for ActiveWorld {
    fn from_world(world: &mut bevy::ecs::world::World) -> Self {
        let config = world.get_resource::<ScoringConfig>().unwrap();
        ActiveWorld {
            world: World { planets: vec![] },
            parent: None,
            cumulative_score: 0.,
            timer: Timer::new(config.scored_time, false),
        }
    }
}

/// Marker component for the score text entity.
struct ScoreText;

struct ParentText;

struct FamilyText;

struct GenerationText;

struct ParentScoreText;

struct HighScoreText;

struct TimeLeftText;

/// Adds a ui camera and score keeper text.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    const FONT_SIZE: f32 = 18.0;

    commands.spawn_bundle(UiCameraBundle::default());

    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::ColumnReverse,
                position_type: PositionType::Absolute,
                position: Rect::all(Val::Percent(0.0)),
                ..Default::default()
            },
            visible: Visible {
                is_transparent: false,
                is_visible: false,
            },
            ..Default::default()
        })
        .with_children(|root| {
            root.spawn_bundle(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    margin: Rect::all(Val::Px(10.0)),
                    ..Default::default()
                },
                visible: Visible {
                    is_transparent: false,
                    is_visible: false,
                },
                ..Default::default()
            })
            .with_children(|row| {
                row.spawn_bundle(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::ColumnReverse,
                        ..Default::default()
                    },
                    visible: Visible {
                        is_transparent: false,
                        is_visible: false,
                    },
                    ..Default::default()
                })
                .with_children(|left_col| {
                    // Score text
                    left_col
                        .spawn_bundle(TextBundle {
                            style: Style {
                                align_self: AlignSelf::FlexStart,
                                ..Default::default()
                            },
                            text: Text {
                                sections: vec![
                                    TextSection {
                                        value: "Score: ".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraSans-Book.ttf"),
                                            font_size: FONT_SIZE,
                                            color: Color::WHITE,
                                        },
                                    },
                                    TextSection {
                                        value: "".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraMono-Regular.ttf"),
                                            font_size: FONT_SIZE,
                                            color: Color::GOLD,
                                        },
                                    },
                                ],
                                alignment: TextAlignment {
                                    horizontal: HorizontalAlign::Left,
                                    vertical: VerticalAlign::Top,
                                },
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .insert(ScoreText);

                    left_col
                        .spawn_bundle(TextBundle {
                            style: Style {
                                align_self: AlignSelf::FlexStart,
                                ..Default::default()
                            },
                            text: Text {
                                sections: vec![
                                    TextSection {
                                        value: "Time Left: ".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraSans-Book.ttf"),
                                            font_size: FONT_SIZE,
                                            color: Color::WHITE,
                                        },
                                    },
                                    TextSection {
                                        value: "N/A".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraMono-Regular.ttf"),
                                            font_size: FONT_SIZE,
                                            color: Color::GOLD,
                                        },
                                    },
                                ],
                                alignment: TextAlignment {
                                    horizontal: HorizontalAlign::Left,
                                    vertical: VerticalAlign::Top,
                                },
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .insert(TimeLeftText);
                });

                row.spawn_bundle(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::ColumnReverse,
                        ..Default::default()
                    },
                    visible: Visible {
                        is_transparent: false,
                        is_visible: false,
                    },
                    ..Default::default()
                })
                .with_children(|right_col| {
                    right_col
                        .spawn_bundle(TextBundle {
                            style: Style {
                                align_self: AlignSelf::FlexEnd,
                                ..Default::default()
                            },
                            text: Text {
                                sections: vec![
                                    TextSection {
                                        value: "Parent: ".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraSans-Book.ttf"),
                                            font_size: FONT_SIZE,
                                            color: Color::WHITE,
                                        },
                                    },
                                    TextSection {
                                        value: "None".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraMono-Regular.ttf"),
                                            font_size: FONT_SIZE,
                                            color: Color::GOLD,
                                        },
                                    },
                                ],
                                alignment: TextAlignment {
                                    horizontal: HorizontalAlign::Right,
                                    vertical: VerticalAlign::Top,
                                },
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .insert(ParentText);

                    right_col
                        .spawn_bundle(TextBundle {
                            style: Style {
                                align_self: AlignSelf::FlexEnd,
                                ..Default::default()
                            },
                            text: Text {
                                sections: vec![
                                    TextSection {
                                        value: "Family: ".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraSans-Book.ttf"),
                                            font_size: FONT_SIZE,
                                            color: Color::WHITE,
                                        },
                                    },
                                    TextSection {
                                        value: "None".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraMono-Regular.ttf"),
                                            font_size: FONT_SIZE,
                                            color: Color::GOLD,
                                        },
                                    },
                                ],
                                alignment: TextAlignment {
                                    horizontal: HorizontalAlign::Right,
                                    vertical: VerticalAlign::Top,
                                },
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .insert(FamilyText);

                    right_col
                        .spawn_bundle(TextBundle {
                            style: Style {
                                align_self: AlignSelf::FlexEnd,
                                ..Default::default()
                            },
                            text: Text {
                                sections: vec![
                                    TextSection {
                                        value: "Generation: ".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraSans-Book.ttf"),
                                            font_size: FONT_SIZE,
                                            color: Color::WHITE,
                                        },
                                    },
                                    TextSection {
                                        value: "0".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraMono-Regular.ttf"),
                                            font_size: FONT_SIZE,
                                            color: Color::GOLD,
                                        },
                                    },
                                ],
                                alignment: TextAlignment {
                                    horizontal: HorizontalAlign::Right,
                                    vertical: VerticalAlign::Top,
                                },
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .insert(GenerationText);

                    right_col
                        .spawn_bundle(TextBundle {
                            style: Style {
                                align_self: AlignSelf::FlexEnd,
                                ..Default::default()
                            },
                            text: Text {
                                sections: vec![
                                    TextSection {
                                        value: "Parent Score: ".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraSans-Book.ttf"),
                                            font_size: FONT_SIZE,
                                            color: Color::WHITE,
                                        },
                                    },
                                    TextSection {
                                        value: "N/A".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraMono-Regular.ttf"),
                                            font_size: FONT_SIZE,
                                            color: Color::GOLD,
                                        },
                                    },
                                ],
                                alignment: TextAlignment {
                                    horizontal: HorizontalAlign::Right,
                                    vertical: VerticalAlign::Top,
                                },
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .insert(ParentScoreText);

                    right_col
                        .spawn_bundle(TextBundle {
                            style: Style {
                                align_self: AlignSelf::FlexEnd,
                                ..Default::default()
                            },
                            text: Text {
                                sections: vec![
                                    TextSection {
                                        value: "High Score: ".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraSans-Book.ttf"),
                                            font_size: FONT_SIZE,
                                            color: Color::WHITE,
                                        },
                                    },
                                    TextSection {
                                        value: "N/A".to_string(),
                                        style: TextStyle {
                                            font: asset_server.load("fonts/FiraMono-Regular.ttf"),
                                            font_size: FONT_SIZE,
                                            color: Color::GOLD,
                                        },
                                    },
                                ],
                                alignment: TextAlignment {
                                    horizontal: HorizontalAlign::Right,
                                    vertical: VerticalAlign::Top,
                                },
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .insert(HighScoreText);
                });
            });
        });
}

/// Compute the scenario score for each frame.
fn score(
    time: Res<Time>,
    mut world: ResMut<ActiveWorld>,
    config: Res<ScoringConfig>,
    query: Query<&RigidBodyMassProps, With<Planet>>,
    mut state: ResMut<State<SaverState>>,
) {
    world.timer.tick(time.delta());

    let scenario_time = world.timer.percent() as f64;
    let mut mass_count = 0.0;
    let mut total_mass = 0.0;

    let maxx = config.scored_area.width / 2.0;
    let maxy = config.scored_area.height / 2.0;
    let maxz = config.scored_area.depth / 2.0;

    for rb in query.iter() {
        if rb.world_com.x.abs() > maxx || rb.world_com.y.abs() > maxy || rb.world_com.z.abs() > maxz
        {
            continue;
        }
        mass_count += 1.0;
        total_mass += rb.mass() as f64;
    }

    world.cumulative_score += config
        .score_per_second
        .eval(scenario_time, total_mass, mass_count)
        * time.delta_seconds_f64();

    if world.timer.just_finished() {
        state
            .set(SaverState::Generate)
            .expect("Unable to switch to scenario generation");
    }
}

/// Put the score in the score text.
fn score_text(world: Res<ActiveWorld>, mut query: Query<&mut Text, With<ScoreText>>) {
    for mut text in query.iter_mut() {
        text.sections[1].value = format!("{:.2}", world.cumulative_score);
    }
}

/// Add the parent id.
fn parent_text(world: Res<ActiveWorld>, mut query: Query<&mut Text, With<ParentText>>) {
    for mut text in query.iter_mut() {
        match world.parent {
            None => text.sections[1].value = "None".to_string(),
            Some(ref parent) => text.sections[1].value = format!("{}", parent.id),
        }
    }
}

/// Add the parent score.
fn parent_score_text(world: Res<ActiveWorld>, mut query: Query<&mut Text, With<ParentScoreText>>) {
    for mut text in query.iter_mut() {
        match world.parent {
            None => text.sections[1].value = "N/A".to_string(),
            Some(ref parent) => text.sections[1].value = format!("{:.2}", parent.score),
        }
    }
}

/// Add the generation number.
fn generation_text(world: Res<ActiveWorld>, mut query: Query<&mut Text, With<GenerationText>>) {
    for mut text in query.iter_mut() {
        match world.parent {
            None => text.sections[1].value = "0".to_string(),
            Some(ref parent) => text.sections[1].value = format!("{}", parent.generation + 1),
        }
    }
}

/// Add the family id.
fn family_text(world: Res<ActiveWorld>, mut query: Query<&mut Text, With<FamilyText>>) {
    for mut text in query.iter_mut() {
        match world.parent {
            None => text.sections[1].value = "None".to_string(),
            Some(ref parent) => text.sections[1].value = format!("{}", parent.family),
        }
    }
}

/// Add the high score
fn high_score_text<S: Storage + Component>(
    mut storage: ResMut<S>,
    mut query: Query<&mut Text, With<HighScoreText>>,
) {
    let highest = storage.get_nth_scenario_by_score(0).unwrap();
    for mut text in query.iter_mut() {
        match highest {
            None => text.sections[1].value = "None".to_string(),
            Some(ref highest) => text.sections[1].value = format!("{:.2}", highest.score),
        }
    }
}

/// Show the time remaining
fn time_left_text(world: Res<ActiveWorld>, mut query: Query<&mut Text, With<TimeLeftText>>) {
    let duration = world.timer.duration();
    let remaining = duration.mul_f32(world.timer.percent_left());
    let secs = remaining.as_secs();
    let mins = secs / 60;
    let secs = secs % 60;
    let ms = remaining.subsec_millis();
    for mut text in query.iter_mut() {
        text.sections[1].value = format!("{}:{:02}.{:03}", mins, secs, ms);
    }
}

/// Store scenario results.
fn store_result<S: Storage + Component>(mut tracker: ResMut<ActiveWorld>, mut storage: ResMut<S>) {
    info!("Storing scored world");
    let world = mem::replace(&mut tracker.world, World::default());
    let parent = mem::replace(&mut tracker.parent, None);
    let score = if tracker.cumulative_score.is_nan() {
        warn!("Score was NaN, replacing with -inf");
        f64::NEG_INFINITY
    } else {
        tracker.cumulative_score
    };
    let store_result = match parent {
        Some(parent) => storage.add_child_scenario(world, score, &parent),
        None => storage.add_root_scenario(world, score),
    };
    match store_result {
        Err(error) => error!("Error while storing finished scenario: {}", error),
        Ok(scenario) => info!(
            "Saved scenario {} (parent: {:?}, family: {}, generation: {}) with score {}",
            scenario.id, scenario.parent, scenario.family, scenario.generation, scenario.score,
        ),
    }
}
