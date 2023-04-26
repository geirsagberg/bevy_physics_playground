#![allow(unused_parens)]
use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_rapier2d::prelude::*;
use rand::random;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.))
        .add_plugin(RapierDebugRenderPlugin::default().disabled())
        .add_plugin(EguiPlugin)
        .add_startup_system(setup_camera)
        .add_event::<ToolEvent>()
        .add_system(update_ui)
        .add_system(update_placing.before(update_ui))
        .add_system(spawn_balls.run_if(on_timer(Duration::from_secs_f32(0.01))))
        .add_system(despawn_outside_world)
        .add_system(toggle_debug_rendering)
        .add_system(handle_creation_events)
        .add_system(handle_input)
        .add_system(highlight_sprites)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(State::Default)
        .run();
}

fn highlight_sprites(
    mut added_query: Query<(&mut Sprite), Added<Highlighted>>,
    mut removed_query: Query<&mut Sprite, Without<Highlighted>>,
    mut removals: RemovedComponents<Highlighted>,
) {
    for (mut sprite) in &mut added_query {
        sprite.color = Color::RED;
    }

    for entity in removals.iter() {
        if let Ok(mut sprite) = removed_query.get_mut(entity) {
            sprite.color = Color::WHITE;
        }
    }
}

fn toggle_debug_rendering(
    mut debug_render_context: ResMut<DebugRenderContext>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::F1) {
        debug_render_context.enabled = !debug_render_context.enabled;
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn despawn_outside_world(
    mut commands: Commands,
    query: Query<(Entity, &Transform), Without<Placing>>,
    window_query: Query<&Window>,
) {
    if let Ok(window) = window_query.get_single() {
        for (entity, transform) in &mut query.iter() {
            if transform.translation.y < -window.resolution.height() / 2. {
                commands.entity(entity).despawn();
            }
        }
    }
}

#[derive(Resource, Debug, PartialEq)]
enum State {
    Default,
    Placing,
    Scaling { start: Vec3 },
    Rotating,
    Selecting(SelectAction),
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum SelectAction {
    Delete,
}

#[derive(Component)]
struct Placing;

fn spawn_balls(mut commands: Commands, window_query: Query<&Window>) {
    let resolution = match window_query.get_single() {
        Ok(window) => &window.resolution,
        Err(_) => return,
    };
    let width = resolution.width();
    let height = resolution.height();

    let rand_position = Vec2::new(width * (random::<f32>() - 0.5), height * 0.5 + 100.);
    let half = 1.;
    commands.spawn((
        RigidBody::Dynamic,
        Collider::ball(half),
        Ccd::enabled(),
        SpriteBundle {
            transform: Transform {
                translation: rand_position.extend(0.),
                ..default()
            },
            sprite: Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::new(half * 2., half * 2.)),
                ..default()
            },
            ..default()
        },
    ));
}

#[derive(Debug, Clone, Copy, Component)]
struct Highlighted;

fn update_placing(
    mut placing_query: Query<(Entity, &mut Transform, &mut Sprite), With<Placing>>,
    camera_query: Query<(&GlobalTransform, &Camera)>,
    window_query: Query<&Window>,
    mouse: Res<Input<MouseButton>>,
    mut state: ResMut<State>,
    mut commands: Commands,
    rapier_context: Res<RapierContext>,
    highlighted_query: Query<Entity, With<Highlighted>>,
) {
    let (camera_transform, camera) = camera_query.single();
    let window = window_query.single();

    let position = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor))
        .unwrap_or_default();

    match *state {
        State::Default => {}
        State::Placing => {
            for (_, mut transform, _) in &mut placing_query {
                transform.translation.x = position.x;
                transform.translation.y = position.y;
            }
            if mouse.just_pressed(MouseButton::Left) {
                *state = State::Scaling {
                    start: position.extend(0.),
                };
            }
        }
        State::Scaling { start } => {
            for (_, mut transform, _) in &mut placing_query {
                transform.translation.x = (position.x + start.x) / 2.;
                transform.translation.y = (position.y + start.y) / 2.;

                transform.scale.x = ((position.x - transform.translation.x) * 2.).abs();
                transform.scale.y = ((position.y - transform.translation.y) * 2.).abs();

                if mouse.just_pressed(MouseButton::Left) {
                    *state = State::Rotating;
                }
            }
        }
        State::Rotating => {
            for (entity, mut transform, mut sprite) in &mut placing_query {
                transform.rotation = Quat::from_rotation_z(
                    -(position - transform.translation.truncate())
                        .angle_between(Vec2::new(1.0, 0.0)),
                );

                if mouse.just_pressed(MouseButton::Left) {
                    sprite.color = Color::WHITE;
                    *state = State::Default;

                    commands
                        .entity(entity)
                        .remove::<Placing>()
                        .insert(RigidBody::Fixed)
                        .insert(Collider::cuboid(0.5, 0.5));
                }
            }
        }
        State::Selecting(action) => {
            let mut entities = Vec::new();

            rapier_context.intersections_with_point(position, QueryFilter::default(), |entity| {
                entities.push(entity);
                true
            });

            for entity in &entities {
                commands.entity(*entity).insert(Highlighted);
            }

            for entity in &highlighted_query {
                if !entities.contains(&entity) {
                    commands.entity(entity).remove::<Highlighted>();
                }
            }

            if mouse.just_pressed(MouseButton::Left) {
                for entity in entities {
                    match action {
                        SelectAction::Delete => commands.entity(entity).despawn(),
                    }
                }

                *state = State::Default;
            }
        }
    }
}

enum Tool {
    Box,
    Delete,
    ForceField,
}

struct ToolEvent(Tool);

fn update_ui(
    mut egui_contexts: EguiContexts,
    state: Res<State>,
    mut event_sender: EventWriter<ToolEvent>,
) {
    let ctx = egui_contexts.ctx_mut();

    egui::Window::new("Physics").show(ctx, |ui| {
        ui.label(format!("State: {:?}", *state));

        let mut add_button = |label: &str, tool: Tool| {
            ui.add_enabled_ui(*state == State::Default, |ui| {
                if ui.button(label).clicked() {
                    event_sender.send(ToolEvent(tool));
                }
            });
        };

        add_button("Box", Tool::Box);
        add_button("Delete", Tool::Delete);
    });
}

fn handle_creation_events(
    mut event_reader: EventReader<ToolEvent>,
    mut commands: Commands,
    mut state: ResMut<State>,
) {
    for event in event_reader.iter() {
        match *state {
            State::Default => match event.0 {
                Tool::Box => {
                    commands.spawn((
                        Placing,
                        SpriteBundle {
                            sprite: Sprite {
                                color: Color::GRAY,
                                ..default()
                            },
                            ..default()
                        },
                    ));
                    *state = State::Placing;
                }
                Tool::Delete => {
                    *state = State::Selecting(SelectAction::Delete);
                }
                Tool::ForceField => {
                    commands.spawn((
                        Placing,
                        SpriteBundle {
                            sprite: Sprite {
                                color: Color::GRAY,
                                ..default()
                            },
                            ..default()
                        },
                    ));
                    *state = State::Placing;
                }
            },
            _ => {}
        }
    }
}

fn handle_input(
    mut commands: Commands,
    mut state: ResMut<State>,
    keyboard_input: Res<Input<KeyCode>>,
    mut event_sender: EventWriter<ToolEvent>,
    placing_query: Query<Entity, With<Placing>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        match *state {
            State::Default => {}
            _ => {
                *state = State::Default;

                if let Ok(entity) = placing_query.get_single() {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
    if keyboard_input.just_pressed(KeyCode::B) {
        event_sender.send(ToolEvent(Tool::Box));
    }

    if keyboard_input.just_pressed(KeyCode::D) {
        event_sender.send(ToolEvent(Tool::Delete));
    }
}
