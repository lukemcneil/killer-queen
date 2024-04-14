use std::time::Duration;

use bevy::{prelude::*, utils::HashMap};
use bevy_rapier2d::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::{
    animation::Animation,
    berries::{Berry, BerryBundle},
    ship::RidingOnShip,
    WinCondition, WinEvent, WINDOW_BOTTOM_Y, WINDOW_HEIGHT, WINDOW_LEFT_X, WINDOW_RIGHT_X,
    WINDOW_TOP_Y, WINDOW_WIDTH,
};

const PLAYER_MAX_VELOCITY_X: f32 = 600.0;
const PLAYER_MIN_VELOCITY_X: f32 = 40.0;
const PLAYER_MAX_VELOCITY_Y: f32 = 600.0;
const PLAYER_FLY_IMPULSE: f32 = 67.5;
pub const PLAYER_JUMP_IMPULSE: f32 = 45.0;
const PLAYER_MOVEMENT_IMPULSE_GROUND: f32 = 180.0;
const PLAYER_MOVEMENT_IMPULSE_AIR: f32 = 115.0;
const PLAYER_FRICTION_GROUND: f32 = 0.5;
const PLAYER_FRICTION_AIR: f32 = 0.3;
const PLAYER_GRAVITY_SCALE: f32 = 15.0;
pub const PLAYER_COLLIDER_WIDTH_MULTIPLIER: f32 = 0.5;

const SPRITESHEET_COLS: usize = 7;
const SPRITESHEET_ROWS: usize = 8;

const SPRITE_TILE_WIDTH: f32 = 128.0;
const SPRITE_TILE_HEIGHT: f32 = 256.0;
const SPRITE_TILE_ACTUAL_HEIGHT: f32 = 160.0;

pub const WORKER_RENDER_WIDTH: f32 = 32.0;
pub const WORKER_RENDER_HEIGHT: f32 = 40.0;
pub const QUEEN_RENDER_WIDTH: f32 = 48.0;
pub const QUEEN_RENDER_HEIGHT: f32 = 60.0;

const SPRITE_IDX_STAND: usize = 28;
const SPRITE_IDX_WALKING: &[usize] = &[7, 0];
const SPRITE_IDX_JUMP: usize = 35;
const SPRITE_IDX_FALL: usize = 42;

const CYCLE_DELAY: Duration = Duration::from_millis(70);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<Action>::default())
            .init_resource::<JoinedPlayers>()
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
                            fly,
                            jump,
                            update_sprite_direction,
                            apply_movement_animation,
                            apply_idle_sprite.after(movement),
                            apply_jump_sprite,
                            join,
                            wrap_around_screen,
                        )
                            .after(check_if_players_on_ground),
                    )
                        .before(disconnect)
                        .before(players_attack),
                    disconnect,
                    players_attack,
                    apply_knockbacks.after(players_attack),
                    check_for_queen_death_win,
                    update_queen_lives_counter,
                ),
            );
    }
}

#[derive(Resource, Default)]
struct JoinedPlayers(pub HashMap<Gamepad, Entity>);

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum Action {
    Move,
    Jump,
    Disconnect,
}

#[derive(Component)]
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
    gamepad: Gamepad,
    is_on_ground: bool,
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

fn join(
    mut commands: Commands,
    mut joined_players: ResMut<JoinedPlayers>,
    gamepads: Res<Gamepads>,
    button_inputs: Res<ButtonInput<GamepadButton>>,
    mut atlases: ResMut<Assets<TextureAtlasLayout>>,
    server: Res<AssetServer>,
) {
    for gamepad in gamepads.iter() {
        // Join the game when both bumpers (L+R) on the controller are pressed
        // We drop down the Bevy's input to get the input from each gamepad
        if button_inputs.any_just_pressed([
            GamepadButton::new(gamepad, GamepadButtonType::LeftTrigger),
            GamepadButton::new(gamepad, GamepadButtonType::LeftTrigger2),
            GamepadButton::new(gamepad, GamepadButtonType::RightTrigger),
            GamepadButton::new(gamepad, GamepadButtonType::RightTrigger2),
        ]) {
            let join_as_queen = button_inputs.any_just_pressed([
                GamepadButton::new(gamepad, GamepadButtonType::LeftTrigger2),
                GamepadButton::new(gamepad, GamepadButtonType::RightTrigger2),
            ]);
            let (player_width, player_height) = if join_as_queen {
                (QUEEN_RENDER_WIDTH, QUEEN_RENDER_HEIGHT)
            } else {
                (WORKER_RENDER_WIDTH, WORKER_RENDER_HEIGHT)
            };
            let team = if button_inputs.any_just_pressed([
                GamepadButton::new(gamepad, GamepadButtonType::LeftTrigger),
                GamepadButton::new(gamepad, GamepadButtonType::LeftTrigger2),
            ]) {
                Team::Red
            } else {
                Team::Blue
            };
            // Make sure a player cannot join twice
            if !joined_players.0.contains_key(&gamepad) {
                let texture: Handle<Image> = server.load("spritesheets/spritesheet_players.png");
                let texture_atlas = TextureAtlasLayout::from_grid(
                    Vec2::new(SPRITE_TILE_WIDTH, SPRITE_TILE_HEIGHT),
                    SPRITESHEET_COLS,
                    SPRITESHEET_ROWS,
                    None,
                    None,
                );
                let atlas_handle = atlases.add(texture_atlas);

                let mut input_map = InputMap::default();
                input_map.insert(Action::Jump, GamepadButtonType::South);
                input_map.insert(
                    Action::Move,
                    SingleAxis::symmetric(GamepadAxisType::LeftStickX, 0.5),
                );
                input_map.insert(Action::Move, VirtualAxis::horizontal_dpad());
                input_map.insert(Action::Disconnect, GamepadButtonType::Select);
                input_map.set_gamepad(gamepad);

                let mut player = commands.spawn((
                    SpriteSheetBundle {
                        texture,
                        atlas: TextureAtlas {
                            layout: atlas_handle,
                            index: SPRITE_IDX_STAND,
                        },
                        transform: Transform {
                            translation: Vec3::new(
                                match team {
                                    Team::Red => -WINDOW_WIDTH / 20.0,
                                    Team::Blue => WINDOW_WIDTH / 20.0,
                                },
                                WINDOW_TOP_Y - (WINDOW_HEIGHT / 9.0),
                                2.0,
                            ),
                            ..Default::default()
                        },
                        sprite: Sprite {
                            rect: Some(Rect {
                                min: Vec2 {
                                    x: 0.0,
                                    y: SPRITE_TILE_HEIGHT - SPRITE_TILE_ACTUAL_HEIGHT,
                                },
                                max: Vec2 {
                                    x: SPRITE_TILE_WIDTH,
                                    y: SPRITE_TILE_HEIGHT,
                                },
                            }),
                            custom_size: Some(Vec2 {
                                x: player_width,
                                y: player_height,
                            }),
                            color: team.color(),
                            ..Default::default()
                        },

                        ..Default::default()
                    },
                    Player {
                        gamepad,
                        is_on_ground: false,
                    },
                    Name::new("Player"),
                    InputManagerBundle::with_map(input_map),
                    match team {
                        Team::Red => Direction::Left,
                        Team::Blue => Direction::Right,
                    },
                    team,
                    (
                        RigidBody::Dynamic,
                        GravityScale(PLAYER_GRAVITY_SCALE),
                        Collider::cuboid(
                            player_width / 2.0 * PLAYER_COLLIDER_WIDTH_MULTIPLIER,
                            player_height / 2.0,
                        ),
                        Velocity::default(),
                        ExternalImpulse::default(),
                        LockedAxes::ROTATION_LOCKED,
                        Friction {
                            coefficient: 0.0,
                            combine_rule: CoefficientCombineRule::Min,
                        },
                        ActiveEvents::all(),
                        Ccd::enabled(),
                    ),
                ));
                if join_as_queen {
                    player.insert(Wings);
                    player.insert(Queen);
                }

                if join_as_queen {
                    player.with_children(|children| {
                        let crown_texture: Handle<Image> = server.load("crown.png");
                        children.spawn((
                            SpriteBundle {
                                sprite: Sprite {
                                    custom_size: Some(Vec2::splat(player_width * 1.5)),
                                    ..Default::default()
                                },
                                transform: Transform::from_translation(Vec3 {
                                    x: 0.0,
                                    y: player_height / 2.0,
                                    z: 1.0,
                                }),
                                texture: crown_texture,
                                ..Default::default()
                            },
                            Crown,
                        ));
                    });
                }

                // Insert the created player and its gamepad to the hashmap of joined players
                // Since uniqueness was already checked above, we can insert here unchecked
                joined_players
                    .0
                    .insert_unique_unchecked(gamepad, player.id());
            }
        }
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

fn disconnect(
    mut commands: Commands,
    action_query: Query<(
        Entity,
        &ActionState<Action>,
        &Player,
        Has<Berry>,
        &Transform,
        Option<&RidingOnShip>,
    )>,
    mut joined_players: ResMut<JoinedPlayers>,
    asset_server: Res<AssetServer>,
) {
    for (
        player_entity,
        action_state,
        player,
        killed_has_berry,
        killed_player_transform,
        maybe_riding_on_ship,
    ) in action_query.iter()
    {
        if action_state.pressed(&Action::Disconnect) {
            remove_player(
                &mut commands,
                player_entity,
                &mut joined_players,
                player,
                killed_has_berry,
                killed_player_transform,
                &asset_server,
                maybe_riding_on_ship,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn remove_player(
    commands: &mut Commands,
    player_entity: Entity,
    joined_players: &mut ResMut<JoinedPlayers>,
    player: &Player,
    has_berry: bool,
    transform: &Transform,
    asset_server: &Res<AssetServer>,
    maybe_riding_on_ship: Option<&RidingOnShip>,
) {
    // Despawn the disconnected player and remove them from the joined player list
    commands.entity(player_entity).despawn_recursive();
    joined_players.0.remove(&player.gamepad);

    if has_berry {
        commands.spawn(BerryBundle::new(
            transform.translation.x,
            transform.translation.y,
            RigidBody::Dynamic,
            asset_server,
        ));
    }
    if let Some(riding_on_ship) = maybe_riding_on_ship {
        commands.entity(riding_on_ship.ship).remove::<Team>();
    }
}

fn fly(mut query: Query<(&ActionState<Action>, &mut ExternalImpulse, &mut Velocity), With<Wings>>) {
    for (action_state, mut impulse, mut velocity) in query.iter_mut() {
        if action_state.just_pressed(&Action::Jump) {
            impulse.impulse.y += PLAYER_FLY_IMPULSE;
        }

        velocity.linvel.y = velocity
            .linvel
            .y
            .clamp(-PLAYER_MAX_VELOCITY_Y, PLAYER_MAX_VELOCITY_Y);
    }
}

fn jump(mut query: Query<(&ActionState<Action>, &mut ExternalImpulse, &Player), Without<Wings>>) {
    for (action_state, mut impulse, player) in query.iter_mut() {
        if action_state.just_pressed(&Action::Jump) && player.is_on_ground {
            impulse.impulse.y += PLAYER_JUMP_IMPULSE;
        }
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
    mut joined_players: ResMut<JoinedPlayers>,
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
                        killed_player,
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
                        &mut joined_players,
                        killed_player,
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

fn wrap_around_screen(mut players: Query<&mut Transform, With<Player>>) {
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
