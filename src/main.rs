#![allow(unused_parens)]

use std::time::Duration;

use bevy::sprite::MaterialMesh2dBundle;
use bevy::utils::HashSet;
use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_rapier2d::prelude::*;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use textures::Meshes;
use Command::Created;
use Command::Scaled;

use crate::balls::Ball;
use crate::Command::{Move, Rotate};

mod balls;
mod perlin;
mod textures;
mod ui;

struct MainPlugin;

#[derive(Resource, Default)]
struct ZCounter(f32);

impl Plugin for MainPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::BLACK))
            .insert_resource(Mode::Default)
            .insert_resource(ZCounter::default())
            .insert_resource(Mouse::default());
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.))
        .add_plugins(RapierDebugRenderPlugin::default().disabled())
        .add_plugins(EguiPlugin)
        .add_plugins(MainPlugin)
        .add_systems(Startup, setup_camera)
        .add_systems(Startup, textures::generate_textures)
        .add_event::<ToolEvent>()
        .add_event::<CommandEvent>()
        .add_systems(Update, ui::update_ui)
        .add_systems(Update, calculate_mouse_position)
        .add_systems(Update, handle_left_click.after(calculate_mouse_position))
        .add_systems(Update, set_hover.after(calculate_mouse_position))
        .add_systems(Update, highlight_hover.after(set_hover))
        .add_systems(
            Update,
            balls::spawn_balls.run_if(on_timer(Duration::from_secs_f32(0.01))),
        )
        .add_systems(PostUpdate, balls::despawn_outside_world)
        .add_systems(Update, toggle_debug_rendering)
        .add_systems(Update, handle_tool_events)
        .add_systems(Update, handle_command_events)
        .add_systems(PostUpdate, handle_input)
        .add_systems(Update, scale)
        .add_systems(Update, rotate)
        .add_systems(Update, move_towards_mouse.after(calculate_mouse_position))
        .add_systems(Update, move_to_mouse.after(calculate_mouse_position))
        .add_systems(Update, apply_force_field)
        .run();
}

#[derive(Resource, Debug, Default)]
struct Mouse {
    position: Vec2,
}

#[derive(Debug, Clone, Copy)]
enum HoverPosition {
    Center,
    Edge,
    Corner,
}

#[derive(Component, Debug, Default)]
struct Hoverable {
    position: Option<HoverPosition>,
}

#[derive(Component)]
struct OriginalColor(Color);

fn set_hover(
    mut query: Query<(&mut Hoverable, Entity, &GlobalTransform), With<Collider>>,
    rapier_context: Res<RapierContext>,
    mouse: Res<Mouse>,
) {
    let mut entities = HashSet::new();
    let position = mouse.position;

    rapier_context.intersections_with_point(position, QueryFilter::default(), |entity| {
        entities.insert(entity);
        true
    });

    // Find entity with highest z value
    let mut highest_entity: Option<Entity> = None;
    let mut highest_z = f32::NEG_INFINITY;
    for (_, entity, transform) in &mut query {
        if entities.contains(&entity) {
            let z = transform.translation().z;
            if z > highest_z {
                highest_z = z;
                highest_entity = Some(entity);
            }
        }
    }
    for (mut hoverable, entity, transform) in &mut query {
        if highest_entity == Some(entity) {
            let inverse = transform.compute_matrix().inverse();
            let transformed = inverse.transform_point3(position.extend(0.));
            let x = transformed.x.abs();
            let y = transformed.y.abs();
            let x_is_center = x < 0.45;
            let y_is_center = y < 0.45;
            hoverable.position = if x_is_center && y_is_center {
                Some(HoverPosition::Center)
            } else if x_is_center || y_is_center {
                Some(HoverPosition::Edge)
            } else {
                Some(HoverPosition::Corner)
            };
        } else {
            hoverable.position = None;
        }
    }
}

fn highlight_hover(
    mut query: Query<(
        &Hoverable,
        Option<&Handle<ColorMaterial>>,
        Option<&mut Sprite>,
        Option<&Modifying>,
        Option<&OriginalColor>,
    )>,
    mut color_mterials: ResMut<Assets<ColorMaterial>>,
    mode: Res<Mode>,
    mut egui_contexts: EguiContexts,
) {
    let ctx = egui_contexts.ctx_mut();
    for (hoverable, material, sprite, modifying, original_color) in &mut query {
        let fallback_color = original_color.map(|c| c.0).unwrap_or(Color::WHITE);
        let color: Option<Color> = if *mode == Mode::Default {
            match hoverable.position {
                Some(HoverPosition::Center) => {
                    ctx.set_cursor_icon(egui::CursorIcon::Grab);
                    Some(fallback_color.with_a(0.9))
                }
                Some(HoverPosition::Edge) => {
                    ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    Some(fallback_color.with_a(0.9))
                }
                Some(HoverPosition::Corner) => {
                    ctx.set_cursor_icon(egui::CursorIcon::ResizeSouthEast);
                    Some(fallback_color.with_a(0.9))
                }
                None => None,
            }
        } else if *mode == Mode::Modify {
            match modifying {
                Some(Modifying::Moving { .. }) => {
                    ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                    Some(fallback_color.with_a(0.9))
                }
                Some(Modifying::Rotating { .. }) => {
                    ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    Some(fallback_color.with_a(0.9))
                }
                _ => None,
            }
        } else {
            None
        };
        let color = color.unwrap_or(original_color.map(|c| c.0).unwrap_or(Color::WHITE));
        if let Some(material) = material {
            color_mterials.get_mut(material).unwrap().color = color;
        }
        if let Some(mut sprite) = sprite {
            sprite.color = color;
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

#[derive(Resource, Debug, Clone, Copy, PartialEq)]
enum Mode {
    Default,
    Create,
    Modify,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub enum Modifying {
    Placing,
    Scaling { start: Vec2 },
    Moving { start: Vec2 },
    Rotating { start: Vec2 },
}

fn calculate_mouse_position(
    camera_query: Query<(&GlobalTransform, &Camera)>,
    window_query: Query<&Window>,
    mut mouse: ResMut<Mouse>,
) {
    let (camera_transform, camera) = camera_query.single();
    let window = window_query.single();

    let position = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor))
        .unwrap_or_default();

    mouse.position = position;
}

fn move_to_mouse(mut query: Query<(&mut Transform, &Modifying)>, mouse: Res<Mouse>) {
    for (mut transform, modifying) in &mut query {
        if let Modifying::Placing = *modifying {
            transform.translation.x = mouse.position.x;
            transform.translation.y = mouse.position.y;
        }
    }
}

fn move_towards_mouse(
    mut query: Query<(&mut Velocity, &GlobalTransform, &Modifying)>,
    mouse: Res<Mouse>,
) {
    for (mut velocity, transform, modifying) in &mut query {
        if let Modifying::Moving { start } = *modifying {
            let translation = transform.translation().truncate();
            velocity.linvel = (mouse.position - translation) * 10.;
        }
    }
}

fn handle_left_click(
    mouse_input: Res<Input<MouseButton>>,
    mode: Res<Mode>,
    mouse: Res<Mouse>,
    mut event_writer: EventWriter<CommandEvent>,
    query: Query<(Entity, &Hoverable)>,
) {
    if mouse_input.just_pressed(MouseButton::Left) {
        match *mode {
            Mode::Default => {
                for (entity, hoverable) in &query {
                    match hoverable.position {
                        Some(HoverPosition::Center) => {
                            event_writer.send(CommandEvent {
                                command: Move {
                                    entity,
                                    start: mouse.position,
                                },
                            });
                        }
                        Some(HoverPosition::Edge) => {
                            event_writer.send(CommandEvent {
                                command: Rotate {
                                    entity,
                                    start: mouse.position,
                                },
                            });
                        }
                        _ => {}
                    }
                }
            }
            Mode::Create => {
                event_writer.send(CommandEvent {
                    command: Created {
                        position: mouse.position,
                    },
                });
            }
            Mode::Modify => {
                event_writer.send(CommandEvent { command: Scaled });
            }
        }
    }
}

fn scale(mut query: Query<(&mut Transform, &Modifying)>, mouse: Res<Mouse>) {
    let position = mouse.position;
    for (mut transform, modifying) in &mut query {
        if let Modifying::Scaling { start } = modifying {
            transform.translation.x = (position.x + start.x) / 2.;
            transform.translation.y = (position.y + start.y) / 2.;

            transform.scale.x = ((position.x - transform.translation.x) * 2.).abs();
            transform.scale.y = ((position.y - transform.translation.y) * 2.).abs();
        }
    }
}

fn rotate(mouse: Res<Mouse>, mut query: Query<(&mut Transform, &Modifying)>) {
    let position = mouse.position;
    for (mut transform, modifying) in &mut query {
        if let Modifying::Rotating { start } = modifying {
            transform.rotation = Quat::from_rotation_z(
                -(position - transform.translation.truncate()).angle_between(Vec2::new(1.0, 0.0)),
            );
        }
    }
}

#[derive(EnumIter, Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum Tool {
    Box,
    ForceField,
}

impl Tool {
    fn key(&self) -> KeyCode {
        match self {
            Tool::Box => KeyCode::B,
            Tool::ForceField => KeyCode::F,
        }
    }

    fn label(&self) -> &str {
        match self {
            Tool::Box => "Box",
            Tool::ForceField => "Force Field",
        }
    }
}

#[derive(Event)]
struct ToolEvent {
    tool: Tool,
}

enum Command {
    Created { position: Vec2 },
    Scaled,
    Move { entity: Entity, start: Vec2 },
    Rotate { entity: Entity, start: Vec2 },
}

#[derive(Event)]
struct CommandEvent {
    command: Command,
}

fn apply_force_field(
    rapier_context: Res<RapierContext>,
    query: Query<(&GlobalTransform, &Solid, &Collider)>,
    balls_query: Query<(Entity), With<Ball>>,
    mut commands: Commands,
) {
    for (entity) in &balls_query {
        commands.entity(entity).insert(ExternalForce {
            force: Vec2::new(0.0, 0.0),
            ..default()
        });
    }

    for (transform, solid, collider) in &query {
        if let Solid::ForceField { force } = solid {
            let (_, rotation, translation) = transform.to_scale_rotation_translation();
            let z_rotation = rotation.z;
            let rotated_force = Vec2::new(
                force.x * z_rotation.cos() - force.y * z_rotation.sin(),
                force.x * z_rotation.sin() + force.y * z_rotation.cos(),
            );
            rapier_context.intersections_with_shape(
                translation.truncate(),
                rotation.z * 2.,
                collider,
                QueryFilter::default(),
                |entity| {
                    commands.get_entity(entity).map(|mut commands| {
                        commands.insert(ExternalForce {
                            force: rotated_force,
                            ..default()
                        });
                    });
                    true
                },
            );
        }
    }
}

fn handle_command_events(
    mut event_reader: EventReader<CommandEvent>,
    mut commands: Commands,
    query: Query<(Entity, &Solid), With<Modifying>>,
) {
    for event in event_reader.iter() {
        match event.command {
            Created { position } => {
                for (entity, _) in &query {
                    commands
                        .entity(entity)
                        .insert(Modifying::Scaling { start: position });
                }
                commands.insert_resource(Mode::Modify);
            }
            Scaled => {
                for (entity, solid) in &query {
                    commands
                        .entity(entity)
                        .remove::<Modifying>()
                        .insert(Velocity::default())
                        .insert(Collider::cuboid(0.5, 0.5));

                    match solid {
                        Solid::Box => {
                            commands
                                .entity(entity)
                                .insert(RigidBody::KinematicVelocityBased);
                        }
                        Solid::ForceField { .. } => {
                            commands
                                .entity(entity)
                                .insert(RigidBody::KinematicVelocityBased);
                            commands.entity(entity).insert(Sensor);
                        }
                    }
                }
                commands.insert_resource(Mode::Default);
            }
            Move { start, entity } => {
                commands.entity(entity).insert(Modifying::Moving { start });
                commands.insert_resource(Mode::Modify);
            }
            Rotate { start, entity } => {
                commands
                    .entity(entity)
                    .insert(Modifying::Rotating { start });
                commands.insert_resource(Mode::Modify);
            }
        }
    }
}

#[derive(Component)]
enum Solid {
    Box,
    ForceField { force: Vec2 },
}

fn handle_tool_events(
    mode: Res<Mode>,
    meshes: Res<Meshes>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut event_reader: EventReader<ToolEvent>,
    mut commands: Commands,
    mut z_counter: ResMut<ZCounter>,
) {
    for event in event_reader.iter() {
        match *mode {
            Mode::Default => match event.tool {
                Tool::Box => {
                    let material = materials.add(ColorMaterial::default());

                    commands.spawn((
                        Solid::Box,
                        Hoverable::default(),
                        Modifying::Placing,
                        MaterialMesh2dBundle {
                            mesh: meshes.get_random(),
                            material,
                            transform: Transform::from_xyz(0.0, 0.0, z_counter.0)
                                .with_scale(Vec3::splat(10.)),
                            ..default()
                        },
                    ));
                    z_counter.0 += 0.01;
                    commands.insert_resource(Mode::Create);
                }
                Tool::ForceField => {
                    let color = Color::rgba(0.0, 0.0, 1.0, 0.1);
                    commands.spawn((
                        Solid::ForceField {
                            force: Vec2::new(0.0, 0.5),
                        },
                        OriginalColor(color),
                        Hoverable::default(),
                        Modifying::Placing,
                        SpriteBundle {
                            sprite: Sprite { color, ..default() },
                            transform: Transform::from_xyz(0.0, 0.0, z_counter.0)
                                .with_scale(Vec3::splat(10.)),
                            ..default()
                        },
                    ));
                    z_counter.0 += 0.01;
                    commands.insert_resource(Mode::Create);
                }
            },
            _ => {}
        }
    }
}

fn handle_input(
    mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    mut event_sender: EventWriter<ToolEvent>,
    query: Query<Entity, With<Modifying>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        for entity in &query {
            commands.entity(entity).despawn();
        }
        commands.insert_resource(Mode::Default);
    }
    for tool in Tool::iter() {
        if keyboard_input.just_pressed(tool.key()) {
            event_sender.send(ToolEvent { tool });
        }
    }
}
