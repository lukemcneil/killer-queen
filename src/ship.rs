use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use leafwing_input_manager::action_state::ActionState;

use crate::{
    player::{
        Action, Direction, KnockBackEvent, Player, Team, Wings, PLAYER_JUMP_IMPULSE,
        WORKER_RENDER_HEIGHT,
    },
    WinCondition, WinEvent, WINDOW_BOTTOM_Y, WINDOW_HEIGHT, WINDOW_WIDTH,
};

pub struct ShipPlugin;

const SHIP_WIDTH: f32 = 124.0 / 2.0;
const SHIP_HEIGHT: f32 = 67.0 / 2.0;
const SHIP_SPEED: f32 = 20.0;
const SHIP_WIN_SPOT: f32 = WINDOW_WIDTH / 2.0 - WINDOW_WIDTH / 18.0;
const SHIP_WIN_SPOT_WIDTH: f32 = 50.0;

impl Plugin for ShipPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (
                get_on_ship,
                move_ship,
                jump_off_ship,
                color_ships_with_drivers,
                check_for_ship_win,
            ),
        );
    }
}

#[derive(Component)]
pub struct Ship;

#[derive(Component)]
pub struct RidingOnShip {
    pub ship: Entity,
}

#[derive(Bundle)]
struct ShipBundle {
    ship: Ship,
    sprite_bundle: SpriteBundle,
    collider: Collider,
}

impl ShipBundle {
    fn new(x: f32, y: f32, asset_server: &Res<AssetServer>) -> Self {
        let texture = asset_server.load("ship.png");
        Self {
            ship: Ship,
            sprite_bundle: SpriteBundle {
                texture,
                sprite: Sprite {
                    custom_size: Some(Vec2::new(SHIP_WIDTH, SHIP_HEIGHT)),
                    ..Default::default()
                },
                transform: Transform {
                    translation: Vec3::new(x, y, -1.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            collider: Collider::ball(SHIP_HEIGHT / 4.0),
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let y = WINDOW_BOTTOM_Y + WINDOW_HEIGHT / 36.0;
    commands.spawn(ShipBundle::new(0.0, y, &asset_server));
    let texture = asset_server.load("ship-target.png");
    for (sign, team) in [(-1.0, Team::Red), (1.0, Team::Blue)] {
        commands.spawn(SpriteBundle {
            texture: texture.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::new(SHIP_WIN_SPOT_WIDTH, SHIP_WIN_SPOT_WIDTH)),
                color: team.color(),
                ..Default::default()
            },
            transform: Transform {
                translation: Vec3::new(SHIP_WIN_SPOT * sign, y, -1.0),
                ..Default::default()
            },
            ..Default::default()
        });
    }
}

fn get_on_ship(
    mut collision_events: EventReader<CollisionEvent>,
    ships: Query<(Option<&Team>, &Transform), With<Ship>>,
    workers: Query<(&Team, &Transform), (With<Player>, Without<Wings>)>,
    mut commands: Commands,
    mut ev_knockback: EventWriter<KnockBackEvent>,
) {
    for collision_event in collision_events.read() {
        if let CollisionEvent::Started(entity1, entity2, _flags) = collision_event {
            for (ship_entity, player_entity) in [(entity1, entity2), (entity2, entity1)] {
                if let Ok((worker_team, worker_transform)) = workers.get(*player_entity) {
                    if let Ok((maybe_ship_team, ship_transform)) = ships.get(*ship_entity) {
                        if maybe_ship_team.is_none() {
                            commands
                                .entity(*player_entity)
                                .insert(RigidBody::Fixed)
                                .insert(RidingOnShip { ship: *ship_entity });
                            commands.entity(*ship_entity).insert(*worker_team);
                        } else {
                            let direction =
                                if worker_transform.translation.x < ship_transform.translation.x {
                                    Direction::Left
                                } else {
                                    Direction::Right
                                };
                            ev_knockback.send(KnockBackEvent {
                                entity: *player_entity,
                                direction,
                            });
                        }
                    }
                }
            }
        };
    }
}

fn move_ship(
    mut workers_on_ships: Query<(&mut Transform, &RidingOnShip), Without<Ship>>,
    mut ships: Query<(&Team, &mut Transform), With<Ship>>,
    time: Res<Time>,
) {
    for (mut worker_transform, riding_on_ship) in workers_on_ships.iter_mut() {
        let (team, mut ship_transform) = ships.get_mut(riding_on_ship.ship).unwrap();
        let direction = match team {
            Team::Red => -1.0,
            Team::Blue => 1.0,
        };
        ship_transform.translation.x += direction * SHIP_SPEED * time.delta_seconds();
        worker_transform.translation = ship_transform.translation;
        worker_transform.translation.y += WORKER_RENDER_HEIGHT / 2.0 + SHIP_HEIGHT / 2.0;
    }
}

fn jump_off_ship(
    mut query: Query<(
        Entity,
        &ActionState<Action>,
        &mut ExternalImpulse,
        &RidingOnShip,
    )>,
    mut commands: Commands,
) {
    for (worker_entity, action_state, mut impulse, riding_on_ship) in query.iter_mut() {
        if action_state.just_pressed(&Action::Jump) {
            commands
                .entity(worker_entity)
                .remove::<RidingOnShip>()
                .insert(RigidBody::Dynamic);
            commands.entity(riding_on_ship.ship).remove::<Team>();
            impulse.impulse.y += PLAYER_JUMP_IMPULSE;
        }
    }
}

fn color_ships_with_drivers(
    mut ships_with_drivers: Query<(Option<&Team>, &mut Sprite), With<Ship>>,
) {
    for (maybe_team, mut sprite) in ships_with_drivers.iter_mut() {
        sprite.color = match maybe_team {
            Some(team) => team.color(),
            None => Color::WHITE,
        };
    }
}

fn check_for_ship_win(
    mut ships: Query<(&Transform, &Team), With<Ship>>,
    mut ev_win: EventWriter<WinEvent>,
) {
    for (transform, &team) in ships.iter_mut() {
        if transform.translation.x.abs() > SHIP_WIN_SPOT {
            ev_win.send(WinEvent {
                team,
                win_condition: WinCondition::Ship,
            });
        }
    }
}
