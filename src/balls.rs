use bevy::math::Vec2;
use bevy::prelude::*;
use bevy_rapier2d::dynamics::{Ccd, RigidBody};
use bevy_rapier2d::geometry::Collider;
use rand::random;

use crate::Modifying;

pub fn despawn_outside_world(
    mut commands: Commands,
    query: Query<(Entity, &Transform), Without<Modifying>>,
    window_query: Query<&Window>,
) {
    if let Ok(window) = window_query.get_single() {
        for (entity, transform) in &mut query.iter() {
            if transform.translation.y < -window.resolution.height()
                || transform.translation.x < -window.resolution.width()
                || transform.translation.x > window.resolution.width()
                || transform.translation.y > window.resolution.height() {
                commands.get_entity(entity).map(|mut entity|  {
                    entity.despawn();
                });
            }
        }
    }
}

#[derive(Component)]
pub struct Ball;

pub fn spawn_balls(mut commands: Commands, window_query: Query<&Window>) {
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
        Ball,
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
