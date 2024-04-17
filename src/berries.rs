use bevy::{prelude::*, utils::HashSet};
use bevy_rapier2d::prelude::*;

use crate::{
    platforms::PLATFORM_HEIGHT,
    player::{Player, Team, Wings, WORKER_RENDER_WIDTH},
    GameState, WinCondition, WinEvent, WINDOW_BOTTOM_Y, WINDOW_HEIGHT, WINDOW_RIGHT_X,
    WINDOW_TOP_Y, WINDOW_WIDTH,
};

const BERRY_RENDER_RADIUS: f32 = 10.0;

pub struct BerriesPlugin;

impl Plugin for BerriesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BerriesCollected>()
            .add_systems(OnEnter(GameState::Join), setup)
            .add_systems(
                Update,
                (grab_berries, put_berries_in_cells, check_for_berry_win),
            )
            .add_systems(
                OnExit(GameState::GameOver),
                (remove_berries_and_berry_cells, reset_berries_collected),
            );
    }
}

#[derive(Default, Resource)]
pub struct BerriesCollected {
    red_berries: i32,
    blue_berries: i32,
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
    pub fn new(x: f32, y: f32, body: RigidBody, asset_server: &Res<AssetServer>) -> Self {
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
                    translation: Vec3::new(x, y, -1.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            body,
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
    team: Team,
}

impl BerryCellBundle {
    fn new(x: f32, y: f32, team: Team, asset_server: &Res<AssetServer>) -> Self {
        let texture = asset_server.load("berry-cell.png");
        Self {
            berry_cell: BerryCell,
            sprite_bundle: SpriteBundle {
                texture,
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(BERRY_RENDER_RADIUS * 2.0)),
                    color: team.color(),
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
            team,
        }
    }
}

fn spawn_berry_bunch(x: f32, y: f32, commands: &mut Commands, asset_server: &Res<AssetServer>) {
    for i in [-1.0, 0.0, 1.0] {
        commands
            .spawn(BerryBundle::new(
                x + i * BERRY_RENDER_RADIUS * 2.0,
                y,
                RigidBody::Fixed,
                asset_server,
            ))
            .insert(Sensor);
    }
    for i in [-0.5, 0.5] {
        commands
            .spawn(BerryBundle::new(
                x + i * BERRY_RENDER_RADIUS * 2.0,
                y + BERRY_RENDER_RADIUS * 3.0 / 2.0,
                RigidBody::Fixed,
                asset_server,
            ))
            .insert(Sensor);
    }
    commands
        .spawn(BerryBundle::new(
            x,
            y + BERRY_RENDER_RADIUS * 3.0,
            RigidBody::Fixed,
            asset_server,
        ))
        .insert(Sensor);
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    for (x, y) in [
        // layer 0
        (
            (WINDOW_RIGHT_X - WINDOW_WIDTH / 5.0),
            WINDOW_BOTTOM_Y + PLATFORM_HEIGHT,
        ),
        (
            -(WINDOW_RIGHT_X - WINDOW_WIDTH / 5.0),
            WINDOW_BOTTOM_Y + PLATFORM_HEIGHT,
        ),
        // layer 1
        (0.0, WINDOW_BOTTOM_Y + WINDOW_HEIGHT / 9.0 + PLATFORM_HEIGHT),
        // layer 2
        (
            0.0,
            WINDOW_BOTTOM_Y + 2.0 * WINDOW_HEIGHT / 9.0 + PLATFORM_HEIGHT,
        ),
        (
            (WINDOW_RIGHT_X - WINDOW_WIDTH / 7.0),
            WINDOW_BOTTOM_Y + 2.0 * WINDOW_HEIGHT / 9.0 + PLATFORM_HEIGHT,
        ),
        (
            -(WINDOW_RIGHT_X - WINDOW_WIDTH / 7.0),
            WINDOW_BOTTOM_Y + 2.0 * WINDOW_HEIGHT / 9.0 + PLATFORM_HEIGHT,
        ),
        // layer 3
        (
            WINDOW_WIDTH / 10.0,
            WINDOW_BOTTOM_Y + 3.0 * WINDOW_HEIGHT / 9.0 + PLATFORM_HEIGHT,
        ),
        (
            -WINDOW_WIDTH / 10.0,
            WINDOW_BOTTOM_Y + 3.0 * WINDOW_HEIGHT / 9.0 + PLATFORM_HEIGHT,
        ),
    ] {
        spawn_berry_bunch(x, y, &mut commands, &asset_server)
    }

    for team in [Team::Red, Team::Blue] {
        for x in -2..2 {
            for y in 0..3 {
                let sign = match team {
                    Team::Red => -1.0,
                    Team::Blue => 1.0,
                };
                commands.spawn(BerryCellBundle::new(
                    (WINDOW_WIDTH / 20.0 + x as f32 * BERRY_RENDER_RADIUS * 2.1) * sign,
                    WINDOW_TOP_Y - (WINDOW_HEIGHT / 9.0) + y as f32 * BERRY_RENDER_RADIUS * 2.1,
                    team,
                    &asset_server,
                ));
            }
        }
    }
}

fn grab_berries(
    mut collision_events: EventReader<CollisionEvent>,
    berries: Query<Entity, (With<Berry>, Without<Player>, Without<BerryCell>)>,
    players_without_berries: Query<Entity, (With<Player>, Without<Berry>, Without<Wings>)>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
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
                        commands
                            .entity(player)
                            .insert(Berry)
                            .with_children(|children| {
                                children
                                    .spawn(BerryBundle::new(
                                        -WORKER_RENDER_WIDTH / 4.0,
                                        0.0,
                                        RigidBody::Dynamic,
                                        &asset_server,
                                    ))
                                    .remove::<RigidBody>()
                                    .remove::<Collider>();
                            });
                        grabbed_berries_this_frame.insert(player);
                    }
                }
            }
        };
    }
}

fn put_berries_in_cells(
    mut collision_events: EventReader<CollisionEvent>,
    mut empty_berry_cells: Query<(Entity, &Team, &mut Sprite), (With<BerryCell>, Without<Berry>)>,
    players_with_berries: Query<(Entity, &Team), (With<Player>, With<Berry>, Without<Wings>)>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut berries_collected: ResMut<BerriesCollected>,
) {
    let mut placed_berries_this_frame = HashSet::new();
    for collision_event in collision_events.read() {
        if let CollisionEvent::Started(entity1, entity2, _flags) = collision_event {
            for (berry_cell_entity, player_entity) in [(entity1, entity2), (entity2, entity1)] {
                if let Ok((berry_cell, berry_cell_team, mut berry_cell_sprite)) =
                    empty_berry_cells.get_mut(*berry_cell_entity)
                {
                    if let Ok((player, player_team)) = players_with_berries.get(*player_entity) {
                        if placed_berries_this_frame.contains(&player) {
                            continue;
                        }
                        if berry_cell_team == player_team {
                            match player_team {
                                Team::Red => berries_collected.red_berries += 1,
                                Team::Blue => berries_collected.blue_berries += 1,
                            };
                            commands
                                .entity(player)
                                .remove::<Berry>()
                                .despawn_descendants();
                            let berry_texture: Handle<Image> = asset_server.load("berry.png");
                            berry_cell_sprite.color = Color::WHITE;
                            commands
                                .entity(berry_cell)
                                .insert(Berry)
                                .insert(berry_texture);
                            placed_berries_this_frame.insert(player);
                        }
                    }
                }
            }
        };
    }
}

fn check_for_berry_win(
    mut ev_win: EventWriter<WinEvent>,
    berries_collected: Res<BerriesCollected>,
) {
    let win_condition = WinCondition::Economic;
    if berries_collected.red_berries >= 12 {
        ev_win.send(WinEvent {
            team: Team::Red,
            win_condition,
        });
    }
    if berries_collected.blue_berries >= 12 {
        ev_win.send(WinEvent {
            team: Team::Blue,
            win_condition,
        });
    }
}

fn reset_berries_collected(mut berries_collected: ResMut<BerriesCollected>) {
    berries_collected.red_berries = 0;
    berries_collected.blue_berries = 0;
}

fn remove_berries_and_berry_cells(
    berries: Query<Entity, With<Berry>>,
    berry_cells: Query<Entity, With<BerryCell>>,
    mut commands: Commands,
) {
    for berry in &berries {
        commands.entity(berry).despawn();
    }
    for berry_cell in &berry_cells {
        commands.entity(berry_cell).despawn();
    }
}
