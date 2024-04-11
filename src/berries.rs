use bevy::{prelude::*, utils::HashSet};
use bevy_rapier2d::prelude::*;

use crate::{
    player::{Player, Wings},
    WINDOW_HEIGHT, WINDOW_WIDTH,
};

const BERRY_RENDER_RADIUS: f32 = 10.0;
pub const BERRY_COLOR: Color = Color::BLUE;

pub struct BerriesPlugin;

impl Plugin for BerriesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (grab_berries, put_berries_in_cells, color_cells_with_berry),
        );
    }
}

#[derive(Component)]
pub struct Berry;

#[derive(Bundle)]
pub struct BerryBundle {
    berry: Berry,
    sprite_bundle: SpriteBundle,
    body: RigidBody,
    collider: Collider,
    restitution: Restitution,
}

impl BerryBundle {
    pub fn new(x: f32, y: f32, asset_server: &Res<AssetServer>) -> Self {
        let texture = asset_server.load("berry.png");
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

#[derive(Component)]
pub struct BerryCell;

#[derive(Bundle)]
struct BerryCellBundle {
    berry_cell: BerryCell,
    sprite_bundle: SpriteBundle,
    collider: Collider,
    sensor: Sensor,
}

impl BerryCellBundle {
    fn new(x: f32, y: f32, asset_server: &Res<AssetServer>) -> Self {
        let texture = asset_server.load("berry-cell.png");
        Self {
            berry_cell: BerryCell,
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
            collider: Collider::ball(BERRY_RENDER_RADIUS),
            sensor: Sensor,
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let num_berries = 10;
    for i in 0..num_berries {
        let x =
            (-WINDOW_WIDTH / 2.0) + (WINDOW_WIDTH / (num_berries as f32 + 1.0)) * (i + 1) as f32;
        commands.spawn((BerryBundle::new(x, 50.0, &asset_server),));
        commands.spawn(BerryCellBundle::new(x, WINDOW_HEIGHT / 4.0, &asset_server));
    }
}

fn grab_berries(
    mut collision_events: EventReader<CollisionEvent>,
    berries: Query<Entity, (With<Berry>, Without<Player>, Without<BerryCell>)>,
    players_without_berries: Query<Entity, (With<Player>, Without<Berry>, Without<Wings>)>,
    mut commands: Commands,
) {
    let mut grabbed_berries_this_frame = HashSet::new();
    for collision_event in collision_events.read() {
        if let CollisionEvent::Started(entity1, entity2, _flags) = collision_event {
            for (berry_entity, player_entity) in [(entity1, entity2), (entity2, entity1)] {
                if let Ok(berry) = berries.get(*berry_entity) {
                    if let Ok(player) = players_without_berries.get(*player_entity) {
                        if grabbed_berries_this_frame.contains(&player) {
                            continue;
                        }
                        commands.entity(berry).despawn();
                        commands.entity(player).insert(Berry);
                        grabbed_berries_this_frame.insert(player);
                    }
                }
            }
        };
    }
}

fn put_berries_in_cells(
    mut collision_events: EventReader<CollisionEvent>,
    empty_berry_cells: Query<Entity, (With<BerryCell>, Without<Berry>)>,
    players_with_berries: Query<Entity, (With<Player>, With<Berry>, Without<Wings>)>,
    mut commands: Commands,
) {
    let mut placed_berries_this_frame = HashSet::new();
    for collision_event in collision_events.read() {
        if let CollisionEvent::Started(entity1, entity2, _flags) = collision_event {
            for (berry_cell_entity, player_entity) in [(entity1, entity2), (entity2, entity1)] {
                if let Ok(berry_cell) = empty_berry_cells.get(*berry_cell_entity) {
                    if let Ok(player) = players_with_berries.get(*player_entity) {
                        if placed_berries_this_frame.contains(&player) {
                            continue;
                        }
                        commands.entity(player).remove::<Berry>();
                        commands.entity(berry_cell).insert(Berry);
                        placed_berries_this_frame.insert(player);
                    }
                }
            }
        };
    }
}

fn color_cells_with_berry(mut berry_cells: Query<(Has<Berry>, &mut Sprite), With<BerryCell>>) {
    for (has_berry, mut sprite) in berry_cells.iter_mut() {
        sprite.color = if has_berry { BERRY_COLOR } else { Color::WHITE };
    }
}
