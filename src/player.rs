use std::{f32::MAX, time::Duration};

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::{
    animation::Animation, berries::Berry, join::remove_player, ship::RidingOnShip, WinCondition,
    WinEvent, WINDOW_BOTTOM_Y, WINDOW_HEIGHT, WINDOW_LEFT_X, WINDOW_RIGHT_X, WINDOW_TOP_Y,
    WINDOW_WIDTH,
};

const PLAYER_MAX_VELOCITY_X: f32 = 600.0;
const PLAYER_MIN_VELOCITY_X: f32 = 40.0;
const PLAYER_MAX_FALL_SPEED: f32 = 400.0;
const PLAYER_MAX_RISE_SPEED: f32 = 600.0;
const PLAYER_FLY_IMPULSE: f32 = 67.5;
pub const PLAYER_JUMP_IMPULSE: f32 = 45.0;
const PLAYER_MOVEMENT_IMPULSE_GROUND: f32 = 180.0;
const PLAYER_MOVEMENT_IMPULSE_AIR: f32 = 115.0;
const PLAYER_FRICTION_GROUND: f32 = 0.5;
const PLAYER_FRICTION_AIR: f32 = 0.3;
pub const PLAYER_GRAVITY_SCALE: f32 = 15.0;
pub const PLAYER_COLLIDER_WIDTH_MULTIPLIER: f32 = 0.5;

pub const SPRITESHEET_COLS: usize = 7;
pub const SPRITESHEET_ROWS: usize = 8;

pub const SPRITE_TILE_WIDTH: f32 = 128.0;
pub const SPRITE_TILE_HEIGHT: f32 = 256.0;
pub const SPRITE_TILE_ACTUAL_HEIGHT: f32 = 160.0;

pub const WORKER_RENDER_WIDTH: f32 = 32.0;
pub const WORKER_RENDER_HEIGHT: f32 = 40.0;
pub const QUEEN_RENDER_WIDTH: f32 = 48.0;
pub const QUEEN_RENDER_HEIGHT: f32 = 60.0;

pub const SPRITE_IDX_STAND: usize = 28;
pub const SPRITE_IDX_WALKING: &[usize] = &[7, 0];
pub const SPRITE_IDX_JUMP: usize = 35;
pub const SPRITE_IDX_FALL: usize = 42;

const CYCLE_DELAY: Duration = Duration::from_millis(70);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<Action>::default())
            .init_resource::<QueenDeaths>()
            .add_event::<KnockBackEvent>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    (
                        check_if_players_on_ground,
                        (
                            movement,
                            friction,
                            (fly, jump).before(limit_fall_speed),
                            limit_fall_speed,
                            update_sprite_direction,
                            apply_movement_animation,
                            apply_idle_sprite.after(movement),
                            apply_jump_sprite,
                        )
                            .after(check_if_players_on_ground),
                    )
                        .before(players_attack),
                    players_attack,
                    (wrap_around_screen, apply_knockbacks).after(players_attack),
                    check_for_queen_death_win,
                    update_queen_lives_counter,
                ),
            );
    }
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum Action {
    Move,
    Jump,
    Disconnect,
}

#[derive(Component, Debug)]
pub enum Direction {
    Right,
    Left,
}

#[derive(Component, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Team {
    Red,
    Blue,
}

#[derive(Component)]
pub struct Crown;

#[derive(Component)]
pub struct Queen;

#[derive(Default, Resource)]
pub struct QueenDeaths {
    red_deaths: i32,
    blue_deaths: i32,
}

impl Team {
    pub fn color(&self) -> Color {
        match self {
            Team::Red => Color::rgb_u8(235, 33, 46),
            Team::Blue => Color::rgb_u8(46, 103, 248),
        }
    }
}

#[derive(Component)]
pub struct Player {
    // This gamepad is used to index each player
    pub gamepad: Gamepad,
    pub is_on_ground: bool,
}

#[derive(Component)]
pub struct Wings;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    for team in [Team::Red, Team::Blue] {
        let font = asset_server.load("fonts/FiraSans-Bold.ttf");
        let text_style = TextStyle {
            font: font.clone(),
            font_size: 40.0,
            color: team.color(),
        };
        commands.spawn((
            Text2dBundle {
                text: Text::from_section("", text_style.clone()),
                transform: Transform::from_translation(Vec3::new(
                    match team {
                        Team::Red => -WINDOW_WIDTH / 20.0,
                        Team::Blue => WINDOW_WIDTH / 20.0,
                    },
                    WINDOW_TOP_Y - (WINDOW_HEIGHT / 30.0),
                    2.0,
                )),
                ..Default::default()
            },
            team,
        ));
    }
}

fn update_queen_lives_counter(
    mut counters: Query<(&mut Text, &Team)>,
    queen_deaths: Res<QueenDeaths>,
) {
    for (mut counter_text, counter_team) in counters.iter_mut() {
        counter_text.sections[0].value = format!(
            "Lives: {}",
            3 - match counter_team {
                Team::Red => queen_deaths.red_deaths,
                Team::Blue => queen_deaths.blue_deaths,
            }
        )
    }
}

fn movement(
    mut query: Query<(
        Entity,
        &Player,
        &ActionState<Action>,
        &mut ExternalImpulse,
        &mut Velocity,
    )>,
    mut commands: Commands,
    time: Res<Time>,
) {
    for (player_entity, player, action_state, mut impulse, mut velocity) in query.iter_mut() {
        if action_state.pressed(&Action::Move) {
            let joystick_value = action_state.clamped_value(&Action::Move);
            if joystick_value > 0.0 {
                commands.entity(player_entity).insert(Direction::Right);
            } else if joystick_value < 0.0 {
                commands.entity(player_entity).insert(Direction::Left);
            }
            if player.is_on_ground {
                impulse.impulse.x +=
                    joystick_value * PLAYER_MOVEMENT_IMPULSE_GROUND * time.delta_seconds();
            } else {
                impulse.impulse.x +=
                    joystick_value * PLAYER_MOVEMENT_IMPULSE_AIR * time.delta_seconds();
            }
        } else {
            // stop the player from moving if joystick is not being pressed and moving slowly
            if velocity.linvel.x.abs() < PLAYER_MIN_VELOCITY_X {
                velocity.linvel.x = 0.0;
            }
        }

        velocity.linvel.x = velocity
            .linvel
            .x
            .clamp(-PLAYER_MAX_VELOCITY_X, PLAYER_MAX_VELOCITY_X);
    }
}

fn friction(mut query: Query<(&mut ExternalImpulse, &Velocity, &Player)>, time: Res<Time>) {
    for (mut impulse, velocity, player) in query.iter_mut() {
        if player.is_on_ground {
            impulse.impulse.x -= velocity.linvel.x * PLAYER_FRICTION_GROUND * time.delta_seconds();
        } else {
            impulse.impulse.x -= velocity.linvel.x * PLAYER_FRICTION_AIR * time.delta_seconds();
        }
    }
}

fn fly(mut query: Query<(&ActionState<Action>, &mut ExternalImpulse), With<Wings>>) {
    for (action_state, mut impulse) in query.iter_mut() {
        if action_state.just_pressed(&Action::Jump) {
            impulse.impulse.y += PLAYER_FLY_IMPULSE;
        }
    }
}

fn jump(mut query: Query<(&ActionState<Action>, &mut ExternalImpulse, &Player), Without<Wings>>) {
    for (action_state, mut impulse, player) in query.iter_mut() {
        if action_state.just_pressed(&Action::Jump) && player.is_on_ground {
            impulse.impulse.y += PLAYER_JUMP_IMPULSE;
        }
    }
}

fn limit_fall_speed(mut players: Query<(&mut Velocity, Has<Wings>), With<Player>>) {
    for (mut velocity, has_wings) in players.iter_mut() {
        velocity.linvel.y = velocity.linvel.y.clamp(
            -PLAYER_MAX_FALL_SPEED,
            if has_wings {
                PLAYER_MAX_RISE_SPEED
            } else {
                MAX
            },
        );
    }
}

fn is_close_to_zero(num: f32) -> bool {
    num.abs() < 10.0
}

fn is_running(velocity: &Velocity) -> bool {
    !is_close_to_zero(velocity.linvel.x)
}

fn apply_movement_animation(
    mut commands: Commands,
    query: Query<(Entity, &Velocity, &Player), Without<Animation>>,
) {
    for (player_entity, velocity, player) in query.iter() {
        if is_running(velocity) && player.is_on_ground {
            commands
                .entity(player_entity)
                .insert(Animation::new(SPRITE_IDX_WALKING, CYCLE_DELAY));
        }
    }
}

fn apply_idle_sprite(
    mut commands: Commands,
    mut query: Query<(Entity, &Velocity, &mut TextureAtlas, &Player)>,
) {
    for (player_entity, velocity, mut sprite, player) in query.iter_mut() {
        if !is_running(velocity) && player.is_on_ground {
            commands.entity(player_entity).remove::<Animation>();
            sprite.index = SPRITE_IDX_STAND
        }
    }
}

fn apply_jump_sprite(
    mut commands: Commands,
    mut query: Query<(Entity, &Velocity, &mut TextureAtlas, &Player)>,
) {
    for (player_entity, velocity, mut sprite, player) in query.iter_mut() {
        if !player.is_on_ground {
            commands.entity(player_entity).remove::<Animation>();
            if velocity.linvel.y > 0.0 {
                sprite.index = SPRITE_IDX_JUMP
            } else {
                sprite.index = SPRITE_IDX_FALL
            }
        }
    }
}

fn update_sprite_direction(
    mut query: Query<(&mut Sprite, &Direction, Option<&Children>), With<Player>>,
    mut player_accessories: Query<
        (&mut Sprite, &mut Transform),
        (Or<(With<Crown>, With<Berry>)>, Without<Player>),
    >,
) {
    for (mut sprite, direction, maybe_children) in query.iter_mut() {
        let should_flip_x = match direction {
            Direction::Right => false,
            Direction::Left => true,
        };
        sprite.flip_x = should_flip_x;
        if let Some(children) = maybe_children {
            for child in children {
                if let Ok((mut sprite, mut transform)) = player_accessories.get_mut(*child) {
                    sprite.flip_x = should_flip_x;
                    transform.translation.x =
                        if should_flip_x { 1.0 } else { -1.0 } * transform.translation.x.abs();
                }
            }
        }
    }
}

#[derive(Event)]
pub struct KnockBackEvent {
    pub entity: Entity,
    pub direction: Direction,
}

fn players_attack(
    mut collision_events: EventReader<CollisionEvent>,
    players: Query<
        (
            Entity,
            &Transform,
            &Player,
            &Team,
            Has<Wings>,
            Has<Berry>,
            &Direction,
            &Sprite,
            Option<&RidingOnShip>,
            Has<Queen>,
        ),
        With<Player>,
    >,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut ev_knockback: EventWriter<KnockBackEvent>,
    mut queen_deaths: ResMut<QueenDeaths>,
) {
    for collision_event in collision_events.read() {
        if let CollisionEvent::Started(entity1, entity2, _flags) = collision_event {
            if let (Ok(player1_components), Ok(player2_components)) =
                (players.get(*entity1), players.get(*entity2))
            {
                if player1_components.3 == player2_components.3 {
                    continue;
                }
                let player1_translation = player1_components.1.translation;
                let player2_translation = player2_components.1.translation;
                let player1_half_width = player1_components.7.custom_size.unwrap().x / 2.0
                    * PLAYER_COLLIDER_WIDTH_MULTIPLIER;
                let player2_half_width = player2_components.7.custom_size.unwrap().x / 2.0
                    * PLAYER_COLLIDER_WIDTH_MULTIPLIER;
                let player1_half_height = player1_components.7.custom_size.unwrap().y / 2.0;
                let player2_half_height = player2_components.7.custom_size.unwrap().y / 2.0;

                let x_diff = (player1_translation.x - player2_translation.x).abs();
                let y_diff = (player1_translation.y - player2_translation.y).abs();

                let one_player_on_top = (y_diff - (player1_half_height + player2_half_height))
                    > (x_diff - (player1_half_width + player2_half_width));

                if let Some((killed_player_components, _killer_player_components)) = {
                    let player1_has_wings = player1_components.4;
                    let player2_has_wings = player2_components.4;
                    match (player1_has_wings, player2_has_wings) {
                        (true, true) => {
                            // both are queens
                            if one_player_on_top {
                                // one player on top
                                if player1_translation.y > player2_translation.y {
                                    Some((player2_components, player1_components))
                                } else {
                                    Some((player1_components, player2_components))
                                }
                            } else {
                                // hit each other horizontally
                                let (left_player_components, right_player_components) =
                                    if player1_translation.x < player2_translation.x {
                                        (player1_components, player2_components)
                                    } else {
                                        (player2_components, player1_components)
                                    };
                                let left_player_direction = left_player_components.6;
                                let right_player_direction = right_player_components.6;
                                match (left_player_direction, right_player_direction) {
                                    (Direction::Right, Direction::Right) => {
                                        Some((right_player_components, left_player_components))
                                    }
                                    (Direction::Left, Direction::Left) => {
                                        Some((left_player_components, right_player_components))
                                    }
                                    _ => {
                                        // hit swords or backs
                                        ev_knockback.send(KnockBackEvent {
                                            entity: left_player_components.0,
                                            direction: Direction::Left,
                                        });
                                        ev_knockback.send(KnockBackEvent {
                                            entity: right_player_components.0,
                                            direction: Direction::Right,
                                        });
                                        continue;
                                    }
                                }
                            }
                        }
                        (true, false) => Some((player2_components, player1_components)),
                        (false, true) => Some((player1_components, player2_components)),
                        (false, false) => {
                            // both are workers
                            if !one_player_on_top {
                                let (left_player_components, right_player_components) =
                                    if player1_translation.x < player2_translation.x {
                                        (player1_components, player2_components)
                                    } else {
                                        (player2_components, player1_components)
                                    };
                                // workers hit
                                ev_knockback.send(KnockBackEvent {
                                    entity: left_player_components.0,
                                    direction: Direction::Left,
                                });
                                ev_knockback.send(KnockBackEvent {
                                    entity: right_player_components.0,
                                    direction: Direction::Right,
                                });
                            }
                            None
                        }
                    }
                } {
                    let (
                        killed_entity,
                        killed_player_transform,
                        _,
                        killed_player_team,
                        _,
                        killed_has_berry,
                        _,
                        _,
                        maybe_riding_on_ship,
                        killed_player_is_queen,
                    ) = killed_player_components;

                    if killed_player_is_queen {
                        match killed_player_team {
                            Team::Red => queen_deaths.red_deaths += 1,
                            Team::Blue => queen_deaths.blue_deaths += 1,
                        }
                    }
                    remove_player(
                        &mut commands,
                        killed_entity,
                        killed_has_berry,
                        killed_player_transform,
                        &asset_server,
                        maybe_riding_on_ship,
                    );
                };
            }
        }
    }
}

fn apply_knockbacks(
    mut ev_knockback: EventReader<KnockBackEvent>,
    mut players: Query<&mut ExternalImpulse, With<Player>>,
) {
    for ev in ev_knockback.read() {
        if let Ok(mut impulse) = players.get_mut(ev.entity) {
            impulse.impulse.x += PLAYER_FLY_IMPULSE
                * match ev.direction {
                    Direction::Right => 1.0,
                    Direction::Left => -1.0,
                };
        }
    }
}

fn check_if_players_on_ground(
    mut contact_force_events: EventReader<ContactForceEvent>,
    mut players: Query<&mut Player>,
) {
    for mut player in players.iter_mut() {
        player.is_on_ground = false;
    }

    for contact_force_event in contact_force_events.read() {
        if let Ok(mut player) = players.get_mut(contact_force_event.collider1) {
            if contact_force_event.max_force_direction.y != 0.0 {
                player.is_on_ground = true;
            }
        }

        if let Ok(mut player) = players.get_mut(contact_force_event.collider2) {
            if contact_force_event.max_force_direction.y != 0.0 {
                player.is_on_ground = true;
            }
        }
    }
}

fn check_for_queen_death_win(mut ev_win: EventWriter<WinEvent>, queen_deaths: Res<QueenDeaths>) {
    let win_condition = WinCondition::Military;
    if queen_deaths.red_deaths >= 3 {
        ev_win.send(WinEvent {
            team: Team::Blue,
            win_condition,
        });
    }
    if queen_deaths.blue_deaths >= 3 {
        ev_win.send(WinEvent {
            team: Team::Red,
            win_condition,
        });
    }
}

fn wrap_around_screen(mut players: Query<&mut Transform>) {
    for mut transform in players.iter_mut() {
        if transform.translation.x > WINDOW_RIGHT_X {
            transform.translation.x -= WINDOW_WIDTH;
        }
        if transform.translation.x < WINDOW_LEFT_X {
            transform.translation.x += WINDOW_WIDTH;
        }
        if transform.translation.y > WINDOW_TOP_Y {
            transform.translation.y -= WINDOW_HEIGHT;
        }
        if transform.translation.y < WINDOW_BOTTOM_Y {
            transform.translation.y += WINDOW_HEIGHT;
        }
    }
}
