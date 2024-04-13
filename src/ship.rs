use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use leafwing_input_manager::action_state::ActionState;

use crate::{
    player::{Action, Player, Team, Wings, PLAYER_JUMP_IMPULSE, WORKER_RENDER_HEIGHT},
    WINDOW_BOTTOM_Y, WINDOW_HEIGHT,
};

pub struct ShipPlugin;

const SHIP_WIDTH: f32 = 124.0 / 2.0;
const SHIP_HEIGHT: f32 = 67.0 / 2.0;
const SHIP_SPEED: f32 = 15.0;

impl Plugin for ShipPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (
                get_on_ship,
                move_ship,
                jump_off_ship,
                color_ships_with_drivers,
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
    commands.spawn(ShipBundle::new(
        0.0,
        WINDOW_BOTTOM_Y + WINDOW_HEIGHT / 36.0,
        &asset_server,
    ));
}

fn get_on_ship(
    mut collision_events: EventReader<CollisionEvent>,
    ships: Query<Entity, (With<Ship>, Without<Team>)>,
    workers: Query<&Team, (With<Player>, Without<Wings>)>,
    mut commands: Commands,
) {
    for collision_event in collision_events.read() {
        if let CollisionEvent::Started(entity1, entity2, _flags) = collision_event {
            for (ship_entity, player_entity) in [(entity1, entity2), (entity2, entity1)] {
                if ships.get(*ship_entity).is_ok() {
                    if let Ok(worker_team) = workers.get(*player_entity) {
                        commands
                            .entity(*player_entity)
                            .insert(RigidBody::Fixed)
                            .insert(RidingOnShip { ship: *ship_entity });
                        commands.entity(*ship_entity).insert(*worker_team);
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
            Team::Red => 1.0,
            Team::Blue => -1.0,
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
            Some(team) => team.color(false),
            None => Color::WHITE,
        };
    }
}
