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
        // .add_startup_system(setup_initial_blocks)
        .add_system(update_ui)
        .add_system(update_placing.before(update_ui))
        .add_system(spawn_balls.run_if(on_timer(Duration::from_secs_f32(0.1))))
        .add_system(despawn_outside_world)
        .add_system(toggle_debug_rendering)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(State::Default)
        .run();
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

#[derive(Resource, Debug)]
enum State {
    Default,
    Placing,
    Scaling,
    Rotating,
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

    let rand_position = Vec2::new(width * (random::<f32>() - 0.5), height * 0.5);
    let half = 5.;
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

fn update_placing(
    mut placing_query: Query<(Entity, &mut Transform, &mut Sprite), With<Placing>>,
    camera_query: Query<(&GlobalTransform, &Camera)>,
    window_query: Query<&Window>,
    mouse: Res<Input<MouseButton>>,
    mut state: ResMut<State>,
    mut commands: Commands,
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
                *state = State::Scaling;
            }
        }
        State::Scaling => {
            for (_, mut transform, _) in &mut placing_query {
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
    }
}

fn update_ui(mut egui_contexts: EguiContexts, mut commands: Commands, mut state: ResMut<State>) {
    let ctx = egui_contexts.ctx_mut();

    egui::Window::new("Physics").show(ctx, |ui| {
        ui.label(format!("State: {:?}", *state));
        if ui.button("Box").clicked() {
            match *state {
                State::Default => {
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
                _ => {}
            }
        }
    });
}
