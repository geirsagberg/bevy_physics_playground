#![allow(unused_parens)]

use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy::render::render_resource::{ShaderType, TextureDimension, TextureFormat};
use bevy::sprite::{MaterialMesh2dBundle, Mesh2dHandle};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_rapier2d::prelude::*;
use rand::random;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::Command::Place;

mod perlin;

struct MainPlugin;

impl Plugin for MainPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ClearColor(Color::BLACK))
            .insert_resource(State::Default);
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.))
        .add_plugin(RapierDebugRenderPlugin::default().disabled())
        .add_plugin(EguiPlugin)
        .add_plugin(MainPlugin)
        .add_startup_system(setup_camera)
        .add_startup_system(generate_textures)
        .add_event::<ToolEvent>()
        .add_event::<CommandEvent>()
        .add_system(update_ui)
        .add_system(handle_mouse_controls.before(update_ui))
        .add_system(spawn_balls.run_if(on_timer(Duration::from_secs_f32(0.01))))
        .add_system(despawn_outside_world)
        .add_system(toggle_debug_rendering)
        .add_system(handle_tool_events)
        .add_system(handle_command_events)
        .add_system(handle_input)
        .add_system(highlight_sprites)
        .run();
}

#[derive(Resource, Debug)]
struct Meshes {
    meshes: Vec<Mesh2dHandle>,
}

impl Meshes {
    fn get_random(&self) -> Mesh2dHandle {
        let index = random::<usize>() % self.meshes.len();
        self.meshes[index].clone()
    }
}

fn generate_textures(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let meshes = VERTEX_COLORS.map(|colors| {
        let vertex_colors: Vec<[f32; 4]> = colors.map(|color| color.as_rgba_f32()).to_vec();
        let mut mesh = Mesh::from(shape::Quad::default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors);
        let handle: Mesh2dHandle = meshes.add(mesh).into();
        handle
    }).to_vec();
    commands.insert_resource(Meshes { meshes });
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
            if transform.translation.y < -window.resolution.height()
                || transform.translation.x < -window.resolution.width()
                || transform.translation.x > window.resolution.width()
                || transform.translation.y > window.resolution.height() {
                commands.entity(entity).despawn();
            }
        }
    }
}

#[derive(Resource, Debug, PartialEq)]
enum State {
    Default,
    Placing,
    Moving,
    Scaling { start: Vec3 },
    Rotating,
    Selecting(SelectAction),
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum SelectAction {
    Delete,
    Move,
    Rotate,
    Scale,
}

#[derive(Component)]
struct Placing;

#[derive(Component)]
struct ForceField;

fn spawn_balls(mut commands: Commands, window_query: Query<&Window>) {
    let resolution = match window_query.get_single() {
        Ok(window) => &window.resolution,
        Err(_) => return,
    };
    let width = resolution.width();
    let height = resolution.height();

    let rand_position = Vec2::new(width * (random::<f32>() - 0.5), height * 0.5 + 100.);
    let half = 1.;
    let random_color = Color::rgb(random(), random(), random());

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
                color: random_color,
                custom_size: Some(Vec2::new(half * 2., half * 2.)),
                ..default()
            },
            ..default()
        },
    ));
}

#[derive(Debug, Clone, Copy, Component)]
struct Highlighted;

fn handle_mouse_controls(
    mut placing_query: Query<(&mut Transform), (With<Placing>, Without<Collider>)>,
    mut moving_query: Query<(&mut Velocity, &Transform), (With<Placing>, With<Collider>)>,
    camera_query: Query<(&GlobalTransform, &Camera)>,
    window_query: Query<&Window>,
    mouse: Res<Input<MouseButton>>,
    state: Res<State>,
    mut commands: Commands,
    mut command_event_writer: EventWriter<CommandEvent>,
    rapier_context: Res<RapierContext>,
    highlighted_query: Query<Entity, With<Highlighted>>,
) {
    let (camera_transform, camera) = camera_query.single();
    let window = window_query.single();

    let position = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor))
        .unwrap_or_default();

    let mut move_to_mouse = || {
        for (mut transform) in &mut placing_query {
            transform.translation.x = position.x;
            transform.translation.y = position.y;
        }
    };

    let mut move_towards_mouse = || {
        for (mut velocity, transform) in &mut moving_query {
            velocity.linvel.x = (position.x - transform.translation.x) * 10.;
            velocity.linvel.y = (position.y - transform.translation.y) * 10.;
        }
    };

    match *state {
        State::Default => {}
        State::Placing => {
            move_to_mouse();
            if mouse.just_pressed(MouseButton::Left) {
                commands.insert_resource(State::Scaling {
                    start: position.extend(0.),
                });
            }
        }
        State::Scaling { start } => {
            for (mut transform) in &mut placing_query {
                transform.translation.x = (position.x + start.x) / 2.;
                transform.translation.y = (position.y + start.y) / 2.;

                transform.scale.x = ((position.x - transform.translation.x) * 2.).abs();
                transform.scale.y = ((position.y - transform.translation.y) * 2.).abs();
            }
            if mouse.just_pressed(MouseButton::Left) {
                command_event_writer.send(CommandEvent(Place));
            }
        }
        State::Rotating => {
            for (mut transform) in &mut placing_query {
                transform.rotation = Quat::from_rotation_z(
                    -(position - transform.translation.truncate())
                        .angle_between(Vec2::new(1.0, 0.0)),
                );
            }

            if mouse.just_pressed(MouseButton::Left) {
                command_event_writer.send(CommandEvent(Place));
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
                match action {
                    SelectAction::Delete => {
                        for entity in entities {
                            commands.entity(entity).despawn();
                        }
                        commands.insert_resource(State::Default);
                    }
                    SelectAction::Move => {
                        let state = if entities.is_empty() { State::Default } else {
                            State::Moving
                        };
                        commands.insert_resource(state);
                        for entity in entities {
                            commands
                                .entity(entity)
                                // .remove::<(RigidBody, Collider)>()
                                .insert(Placing);
                        }
                    }
                    SelectAction::Rotate => {
                        let state = if entities.is_empty() { State::Default } else { State::Rotating };
                        commands.insert_resource(state);
                        for entity in entities {
                            commands
                                .entity(entity)
                                // .remove::<(RigidBody, Collider)>()
                                .insert(Placing);
                        }
                    }
                    SelectAction::Scale => {
                        let state = if entities.is_empty() { State::Default } else {
                            State::Scaling {
                                start: position.extend(0.)
                            }
                        };
                        commands.insert_resource(state);
                        for entity in entities {
                            commands
                                .entity(entity)
                                .remove::<(RigidBody, Collider)>()
                                .insert(Placing);
                        }
                    }
                }
            }
        }
        State::Moving => {
            move_towards_mouse();
            if mouse.just_pressed(MouseButton::Left) {
                command_event_writer.send(CommandEvent(Place));
            }
        }
    }
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

struct ToolEvent(Tool);

enum Command {
    Place,
}

struct CommandEvent(Command);

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


        for tool in Tool::iter() {
            add_button(tool.label(), tool);
        }
    });
}

fn handle_command_events(
    mut event_reader: EventReader<CommandEvent>,
    mut commands: Commands,
    mut query: Query<Entity, With<Placing>>,
) {
    for event in event_reader.iter() {
        match event.0 {
            Place => {
                for entity in &mut query {
                    commands
                        .entity(entity)
                        .remove::<Placing>()
                        .insert(RigidBody::Dynamic)
                        .insert(GravityScale(0.0))
                        .insert(Velocity::default())
                        // .insert(ActiveCollisionTypes::KINEMATIC_KINEMATIC)
                        .insert(Collider::cuboid(0.5, 0.5));
                }
                commands.insert_resource(State::Default);
            }
        }
    }
}

// const VERTEX_COLORS: [[Color; 4]; 3] = [
//     [Color::CYAN, Color::WHITE, Color::FUCHSIA, Color::BLUE],
//     [Color::GREEN, Color::YELLOW, Color::WHITE, Color::CYAN],
//     [Color::WHITE, Color::YELLOW, Color::RED, Color::FUCHSIA],
// ];

const VERTEX_COLORS: [[Color; 4]; 4] = [
    [Color::RED, Color::WHITE, Color::GREEN, Color::BLUE],
    [Color::YELLOW, Color::WHITE, Color::PURPLE, Color::RED],
    [Color::ORANGE, Color::BLUE, Color::WHITE, Color::YELLOW],
    [Color::PURPLE, Color::YELLOW, Color::WHITE, Color::BLUE],
];

fn handle_tool_events(
    mut event_reader: EventReader<ToolEvent>,
    mut commands: Commands,
    state: Res<State>,
    meshes: Res<Meshes>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for event in event_reader.iter() {
        match *state {
            State::Default => match event.0 {
                Tool::Box => {
                    let material = materials.add(ColorMaterial::default());

                    commands.spawn((
                        Placing,
                        MaterialMesh2dBundle {
                            mesh: meshes.get_random(),
                            material,
                            ..default()
                        },
                    ));
                    commands.insert_resource(State::Placing);
                }
                Tool::Delete => {
                    commands.insert_resource(State::Selecting(SelectAction::Delete));
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
                    commands.insert_resource(State::Placing);
                }
                Tool::Move => {
                    commands.insert_resource(State::Selecting(SelectAction::Move));
                }
                Tool::Rotate => {
                    commands.insert_resource(State::Selecting(SelectAction::Rotate));
                }
                Tool::Scale => {
                    commands.insert_resource(State::Selecting(SelectAction::Scale));
                }
            },
            _ => {}
        }
    }
}

fn handle_input(
    mut commands: Commands,
    state: Res<State>,
    keyboard_input: Res<Input<KeyCode>>,
    mut event_sender: EventWriter<ToolEvent>,
    placing_query: Query<Entity, With<Placing>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        match *state {
            State::Default => {}
            _ => {
                commands.insert_resource(State::Default);

                if let Ok(entity) = placing_query.get_single() {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
    for tool in Tool::iter() {
        if keyboard_input.just_pressed(tool.key()) {
            event_sender.send(ToolEvent(tool));
        }
    }
}
