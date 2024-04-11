use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::player::{Player, Wings};

const BERRY_RENDER_RADIUS: f32 = 10.0;

pub struct BerriesPlugin;

#[derive(Component)]
pub struct Berry;

#[derive(Bundle)]
struct BerryBundle {
    berry: Berry,
    sprite_bundle: SpriteBundle,
    body: RigidBody,
    collider: Collider,
    restitution: Restitution,
}

impl BerryBundle {
    fn new(x: f32, y: f32, texture: Handle<Image>) -> Self {
        Self {
            berry: Berry,
            sprite_bundle: SpriteBundle {
                texture,
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(BERRY_RENDER_RADIUS * 2.0)),
                    ..Default::default()
                },
                transform: Transform {
                    translation: Vec3::new(x, y, 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            body: RigidBody::Dynamic,
            collider: Collider::ball(BERRY_RENDER_RADIUS),
            restitution: Restitution::coefficient(0.7),
        }
    }
}

impl Plugin for BerriesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, grab_berries);
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let berry_texture = asset_server.load("berry.png");
    for i in -25..25 {
        commands.spawn((BerryBundle::new(
            i as f32 * BERRY_RENDER_RADIUS * 2.1,
            50.0,
            berry_texture.clone(),
        ),));
    }
}

fn grab_berries(
    mut collision_events: EventReader<CollisionEvent>,
    berries: Query<Entity, With<Berry>>,
    players_without_berries: Query<Entity, (With<Player>, Without<Berry>, Without<Wings>)>,
    mut commands: Commands,
) {
    for collision_event in collision_events.read() {
        if let CollisionEvent::Started(entity, player, _flags) = collision_event {
            if let Ok(berry) = berries.get(*entity) {
                if let Ok(player) = players_without_berries.get(*player) {
                    commands.entity(berry).despawn();
                    commands.entity(player).insert(Berry);
                }
            }
        };
    }
}
