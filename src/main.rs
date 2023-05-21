#![allow(unused_parens)]

use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy::sprite::MaterialMesh2dBundle;
use bevy::utils::HashSet;
use bevy_egui::EguiPlugin;
use bevy_prototype_debug_lines::{DebugLines, DebugLinesPlugin, DebugShapes};
use bevy_rapier2d::prelude::*;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use Command::Created;
use Command::Scaled;
use textures::Meshes;

mod perlin;
mod balls;
mod ui;
mod textures;

struct MainPlugin;

impl Plugin for MainPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ClearColor(Color::BLACK))
            .insert_resource(Mode::Default)
            .insert_resource(Mouse::default());
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.))
        .add_plugin(RapierDebugRenderPlugin::default().disabled())
        .add_plugin(EguiPlugin)
        .add_plugin(DebugLinesPlugin::default())
        .add_plugin(MainPlugin)
        .add_startup_system(setup_camera)
        .add_startup_system(textures::generate_textures)
        .add_event::<ToolEvent>()
        .add_event::<CommandEvent>()
        .add_system(ui::update_ui)
        .add_system(calculate_mouse_position)
        .add_system(handle_left_click.after(calculate_mouse_position))
        .add_system(set_hover.after(calculate_mouse_position))
        .add_system(highlight_hover.after(set_hover))
        .add_system(balls::spawn_balls.run_if(on_timer(Duration::from_secs_f32(0.01))))
        .add_system(balls::despawn_outside_world)
        .add_system(toggle_debug_rendering)
        .add_system(handle_tool_events)
        .add_system(handle_command_events)
        .add_system(handle_input)
        .add_system(scale)
        .add_system(move_towards_mouse.after(calculate_mouse_position))
        .add_system(move_to_mouse.after(calculate_mouse_position))
        .run();
}

#[derive(Resource, Debug, Default)]
struct Mouse {
    position: Vec2,
}

#[derive(Component, Debug, Default)]
struct Hoverable {
    is_hovered: bool,
}

fn set_hover(
    mut query: Query<(&mut Hoverable, Entity)>,
    rapier_context: Res<RapierContext>,
    mouse: Res<Mouse>,
) {
    let mut entities = HashSet::new();
    let position = mouse.position;

    rapier_context.intersections_with_point(position, QueryFilter::default(), |entity| {
        entities.insert(entity);
        true
    });

    for (mut hoverable, entity) in query.iter_mut() {
        hoverable.is_hovered = entities.contains(&entity);
    }
}

fn highlight_hover(
    mut query: Query<(&Hoverable, &Handle<ColorMaterial>)>,
    mut color_mterials: ResMut<Assets<ColorMaterial>>,
) {
    for (hoverable, material) in query.iter_mut() {
        let material = color_mterials.get_mut(material).unwrap();
        if hoverable.is_hovered {
            material.color = Color::rgb(0.5, 0.5, 0.5);
        } else {
            material.color = Color::WHITE;
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
    Scaling,
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum SelectAction {
    Delete,
    Move,
    Rotate,
    Scale,
}
#[derive(Component, Debug, Clone, Copy, PartialEq)]
enum Modifying {
    Placing,
    Scaling {start: Vec3}
}

#[derive(Component)]
pub struct Placing;

#[derive(Component)]
struct Moving {
    start: Vec3,
}

#[derive(Component)]
struct Scaling {
    start: Vec3,
}

#[derive(Component)]
struct ForceField;

#[derive(Debug, Clone, Copy, Component)]
struct Highlighted;

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

fn move_to_mouse(
    mut query: Query<(&mut Transform), With<Placing>>,
    mouse: Res<Mouse>,
) {
    for (mut transform) in &mut query {
        transform.translation.x = mouse.position.x;
        transform.translation.y = mouse.position.y;
    }
}

fn move_towards_mouse(
    mut query: Query<(&mut Velocity, &Transform), (With<Placing>, With<Collider>)>,
    mouse: Res<Mouse>,
) {
    for (mut velocity, transform) in &mut query {
        velocity.linvel = (mouse.position - transform.translation.truncate()).normalize() * 100.;
    }
}

fn handle_left_click(
    mouse_input: Res<Input<MouseButton>>,
    mode: Res<Mode>,
    mouse: Res<Mouse>,
    mut event_writer: EventWriter<CommandEvent>,
) {
    if mouse_input.just_pressed(MouseButton::Left) {
        match *mode {
            Mode::Default => {}
            Mode::Create => {
                event_writer.send(CommandEvent { command: Created { position: mouse.position } });
            }
            Mode::Scaling => {
                event_writer.send(CommandEvent { command: Scaled });
            }
        }
    }
}

fn scale(
    mut query: Query<(&mut Transform, &Scaling)>,
    mouse: Res<Mouse>,
) {
    let position = mouse.position;
    for (mut transform, scaling) in &mut query {
        let start = scaling.start;
        transform.translation.x = (position.x + start.x) / 2.;
        transform.translation.y = (position.y + start.y) / 2.;

        transform.scale.x = ((position.x - transform.translation.x) * 2.).abs();
        transform.scale.y = ((position.y - transform.translation.y) * 2.).abs();
    }
}

fn handle_mouse_controls(
    mut placing_query: Query<(&mut Transform), (With<Placing>, Without<Collider>)>,
    mut moving_query: Query<(&mut Velocity, &Transform), (With<Placing>, With<Collider>)>,
    mouse: Res<Mouse>,
    mode: Res<Mode>,
    mouse_input: Res<Input<MouseButton>>,
    mut commands: Commands,
    mut command_event_writer: EventWriter<CommandEvent>,
    rapier_context: Res<RapierContext>,
    highlighted_query: Query<Entity, With<Highlighted>>,
) {
    // match *mode {
    //     Mode::Default => {}
    //     State::Placing => {
    //         move_to_mouse();
    //         if mouse_input.just_pressed(MouseButton::Left) {
    //             commands.insert_resource(State::Scaling {
    //                 start: position.extend(0.),
    //             });
    //         }
    //     }
    //     State::Scaling { start } => {
    //         for (mut transform) in &mut placing_query {
    //             transform.translation.x = (position.x + start.x) / 2.;
    //             transform.translation.y = (position.y + start.y) / 2.;
    //
    //             transform.scale.x = ((position.x - transform.translation.x) * 2.).abs();
    //             transform.scale.y = ((position.y - transform.translation.y) * 2.).abs();
    //         }
    //         if mouse_input.just_pressed(MouseButton::Left) {
    //             command_event_writer.send(CommandEvent(Place));
    //         }
    //     }
    //     State::Rotating => {
    //         for (mut transform) in &mut placing_query {
    //             transform.rotation = Quat::from_rotation_z(
    //                 -(position - transform.translation.truncate())
    //                     .angle_between(Vec2::new(1.0, 0.0)),
    //             );
    //         }
    //
    //         if mouse_input.just_pressed(MouseButton::Left) {
    //             command_event_writer.send(CommandEvent(Place));
    //         }
    //     }
    //     State::Selecting(action) => {
    //         let mut entities = Vec::new();
    //
    //         rapier_context.intersections_with_point(position, QueryFilter::default(), |entity| {
    //             entities.push(entity);
    //             true
    //         });
    //
    //         for entity in &entities {
    //             commands.entity(*entity).insert(Highlighted);
    //         }
    //
    //         for entity in &highlighted_query {
    //             if !entities.contains(&entity) {
    //                 commands.entity(entity).remove::<Highlighted>();
    //             }
    //         }
    //
    //         if mouse_input.just_pressed(MouseButton::Left) {
    //             match action {
    //                 SelectAction::Delete => {
    //                     for entity in entities {
    //                         commands.entity(entity).despawn();
    //                     }
    //                     commands.insert_resource(Mode::Default);
    //                 }
    //                 SelectAction::Move => {
    //                     let state = if entities.is_empty() { Mode::Default } else {
    //                         State::Moving
    //                     };
    //                     commands.insert_resource(state);
    //                     for entity in entities {
    //                         commands
    //                             .entity(entity)
    //                             // .remove::<(RigidBody, Collider)>()
    //                             .insert(Placing);
    //                     }
    //                 }
    //                 SelectAction::Rotate => {
    //                     let state = if entities.is_empty() { Mode::Default } else { State::Rotating };
    //                     commands.insert_resource(state);
    //                     for entity in entities {
    //                         commands
    //                             .entity(entity)
    //                             // .remove::<(RigidBody, Collider)>()
    //                             .insert(Placing);
    //                     }
    //                 }
    //                 SelectAction::Scale => {
    //                     let state = if entities.is_empty() { Mode::Default } else {
    //                         State::Scaling {
    //                             start: position.extend(0.)
    //                         }
    //                     };
    //                     commands.insert_resource(state);
    //                     for entity in entities {
    //                         commands
    //                             .entity(entity)
    //                             .remove::<(RigidBody, Collider)>()
    //                             .insert(Placing);
    //                     }
    //                 }
    //             }
    //         }
    //     }
    //     State::Moving => {
    //         move_towards_mouse();
    //         if mouse_input.just_pressed(MouseButton::Left) {
    //             command_event_writer.send(CommandEvent(Place));
    //         }
    //     }
    // }
}


#[derive(EnumIter, Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum Tool {
    Box,
    Delete,
    Move,
    Rotate,
    Scale,
    ForceField,
}

impl Tool {
    fn key(&self) -> KeyCode {
        match self {
            Tool::Box => KeyCode::B,
            Tool::Delete => KeyCode::D,
            Tool::Move => KeyCode::M,
            Tool::Rotate => KeyCode::R,
            Tool::ForceField => KeyCode::F,
            Tool::Scale => KeyCode::S,
        }
    }

    fn label(&self) -> &str {
        match self {
            Tool::Box => "Box",
            Tool::Delete => "Delete",
            Tool::Move => "Move",
            Tool::Rotate => "Rotate",
            Tool::ForceField => "Force Field",
            Tool::Scale => "Scale",
        }
    }
}

struct ToolEvent {
    tool: Tool,
}

enum Command {
    Created { position: Vec2 },
    Scaled,
}

struct CommandEvent {
    command: Command,
}

fn handle_command_events(
    mut event_reader: EventReader<CommandEvent>,
    mut commands: Commands,
    mut placing_query: Query<Entity, With<Placing>>,
    mut scaling_query: Query<Entity, With<Scaling>>,
) {
    for event in event_reader.iter() {
        match event.command {
            Created { position } => {
                for entity in &mut placing_query {
                    commands
                        .entity(entity)
                        .remove::<Placing>()
                        .insert(Scaling {
                            start: position.extend(0.),
                        });
                    // .insert(RigidBody::Dynamic)
                    // .insert(GravityScale(0.0))
                    // .insert(Velocity::default())
                    // // .insert(ActiveCollisionTypes::KINEMATIC_KINEMATIC)
                    // .insert(Collider::cuboid(0.5, 0.5));
                }
                commands.insert_resource(Mode::Scaling);
            }
            Scaled => {
                for entity in &mut scaling_query {
                    commands
                        .entity(entity)
                        .remove::<Scaling>()
                        .insert(RigidBody::KinematicVelocityBased)
                        .insert(GravityScale(0.0))
                        .insert(Velocity::default())
                        .insert(Collider::cuboid(0.5, 0.5));
                }
                commands.insert_resource(Mode::Default);
            }
            _ => {}
        }
    }
}

fn handle_tool_events(
    mode: Res<Mode>,
    meshes: Res<Meshes>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut event_reader: EventReader<ToolEvent>,
    mut commands: Commands,
) {
    for event in event_reader.iter() {
        match *mode {
            Mode::Default => match event.tool {
                Tool::Box => {
                    let material = materials.add(ColorMaterial::default());

                    commands.spawn((
                        Placing,
                        MaterialMesh2dBundle {
                            mesh: meshes.get_random(),
                            material,
                            transform: Transform::default().with_scale(Vec3::splat(10.)),
                            ..default()
                        },
                    ));
                    commands.insert_resource(Mode::Create);
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
                    commands.insert_resource(Mode::Create);
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn handle_input(
    mut commands: Commands,
    mode: Res<Mode>,
    keyboard_input: Res<Input<KeyCode>>,
    mut event_sender: EventWriter<ToolEvent>,
    placing_query: Query<Entity, With<Placing>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        match *mode {
            Mode::Default => {}
            _ => {
                if let Ok(entity) = placing_query.get_single() {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
    for tool in Tool::iter() {
        if keyboard_input.just_pressed(tool.key()) {
            event_sender.send(ToolEvent { tool });
        }
    }
}
