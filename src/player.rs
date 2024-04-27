use std::{f32::MAX, time::Duration};

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::{
    animation::Animation, berries::Berry, join::remove_player, ship::RidingOnShip, GameState,
    WinCondition, WinEvent, WINDOW_BOTTOM_Y, WINDOW_HEIGHT, WINDOW_LEFT_X, WINDOW_RIGHT_X,
    WINDOW_TOP_Y, WINDOW_WIDTH,
};

const PLAYER_MAX_VELOCITY_X: f32 = 600.0;
const PLAYER_MIN_VELOCITY_X: f32 = 40.0;
const PLAYER_MAX_FALL_SPEED: f32 = 400.0;
const PLAYER_MAX_DIVE_SPEED: f32 = 1200.0;
const PLAYER_MAX_RISE_SPEED: f32 = 600.0;
const PLAYER_FLY_IMPULSE: f32 = 55.0;
pub const PLAYER_JUMP_IMPULSE: f32 = 35.0;
const PLAYER_MOVEMENT_IMPULSE_GROUND: f32 = 180.0;
const PLAYER_MOVEMENT_IMPULSE_AIR: f32 = 115.0;
const PLAYER_FRICTION_GROUND: f32 = 0.5;
const PLAYER_FRICTION_AIR: f32 = 0.3;
const PLAYER_GRAVITY_SCALE: f32 = 15.0;
const DIVE_GRAVITY_SCALE: f32 = 45.0;
pub const PLAYER_COLLIDER_WIDTH_MULTIPLIER: f32 = 0.3;
const RESPAWN_DELAY: f32 = 2.0;
const INVINCIBILITY_DURATION: f32 = 2.0;

const SPRITESHEET_COLS: usize = 2;
const SPRITESHEET_ROWS: usize = 2;

const SPRITE_TILE_WIDTH: f32 = 25.0;
const SPRITE_TILE_HEIGHT: f32 = 25.0;
const SPRITE_PADDING: f32 = 3.0;

pub const WORKER_RENDER_WIDTH: f32 = 40.0;
pub const WORKER_RENDER_HEIGHT: f32 = 40.0;
pub const QUEEN_RENDER_WIDTH: f32 = 60.0;
pub const QUEEN_RENDER_HEIGHT: f32 = 60.0;

const SPRITE_IDX_STAND: usize = 0;
const SPRITE_IDX_WALKING: &[usize] = &[1, 0];
const SPRITE_IDX_FLYING: &[usize] = &[2, 0];
const SPRITE_IDX_DIVING: &[usize] = &[3];

const CYCLE_DELAY: Duration = Duration::from_millis(70);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<Action>::default())
            .init_resource::<QueenDeaths>()
            .add_event::<KnockBackEvent>()
            .add_event::<SpawnPlayerEvent>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    (
                        check_if_players_on_ground,
                        (
                            movement,
                            friction,
                            (fly, jump, dive).before(limit_fall_speed),
                            limit_fall_speed,
                            update_sprite_direction,
                            apply_movement_animation,
                            apply_idle_sprite.after(movement),
                            apply_fly_sprite,
                        )
                            .after(check_if_players_on_ground),
                    )
                        .before(players_attack),
                    players_attack,
                    (wrap_around_screen, apply_knockbacks).after(players_attack),
                    check_for_queen_death_win,
                    update_queen_lives_counter,
                    add_delayed_player_spawners,
                    spawn_players,
                    handle_invincibility,
                ),
            )
            .add_systems(
                OnExit(GameState::GameOver),
                (reset_all_players, reset_queen_lives_counter),
            );
    }
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum Action {
    Move,
    Jump,
    Disconnect,
    Dive,
}

#[derive(Component, Debug)]
pub enum Direction {
    Right,
    Left,
}

#[derive(Component, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Team {
    Yellow,
    Purple,
}

impl Team {
    pub fn color(&self) -> Color {
        match self {
            Team::Yellow => Color::YELLOW,
            Team::Purple => Color::PURPLE,
        }
    }
}

#[derive(Component)]
pub struct Queen;

#[derive(Component)]
struct Invincible {
    timer: Timer,
    animation_timer: Timer,
}

#[derive(Default, Resource)]
pub struct QueenDeaths {
    yellow_deaths: i32,
    purple_deaths: i32,
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
    for team in [Team::Yellow, Team::Purple] {
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
                        Team::Yellow => -WINDOW_WIDTH / 20.0,
                        Team::Purple => WINDOW_WIDTH / 20.0,
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

fn reset_queen_lives_counter(mut queen_deaths: ResMut<QueenDeaths>) {
    queen_deaths.yellow_deaths = 0;
    queen_deaths.purple_deaths = 0;
}

fn update_queen_lives_counter(
    mut counters: Query<(&mut Text, &Team)>,
    queen_deaths: Res<QueenDeaths>,
) {
    for (mut counter_text, counter_team) in counters.iter_mut() {
        counter_text.sections[0].value = format!(
            "Lives: {}",
            3 - match counter_team {
                Team::Yellow => queen_deaths.yellow_deaths,
                Team::Purple => queen_deaths.purple_deaths,
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
        // shouldn't be able to move if dove into ground
        if action_state.pressed(&Action::Dive) && player.is_on_ground {
            velocity.linvel.x = 0.0;
            continue;
        }
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
        if action_state.just_pressed(&Action::Jump) && !action_state.pressed(&Action::Dive) {
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

fn dive(mut queens: Query<(Entity, &ActionState<Action>)>, mut commands: Commands) {
    for (entity, action_state) in &mut queens {
        if action_state.just_pressed(&Action::Dive) {
            commands
                .entity(entity)
                .insert(GravityScale(DIVE_GRAVITY_SCALE))
                .insert(Animation::new(SPRITE_IDX_DIVING, CYCLE_DELAY));
        }
        if action_state.just_released(&Action::Dive) {
            commands
                .entity(entity)
                .insert(GravityScale(PLAYER_GRAVITY_SCALE))
                .remove::<Animation>();
        }
    }
}

fn limit_fall_speed(
    mut players: Query<(&mut Velocity, Has<Wings>, &ActionState<Action>), With<Player>>,
) {
    for (mut velocity, has_wings, action_state) in players.iter_mut() {
        velocity.linvel.y = velocity.linvel.y.clamp(
            if action_state.pressed(&Action::Dive) {
                -PLAYER_MAX_DIVE_SPEED
            } else {
                -PLAYER_MAX_FALL_SPEED
            },
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
    query: Query<(Entity, &Velocity, &Player, Option<&Animation>)>,
) {
    for (player_entity, velocity, player, animation) in query.iter() {
        if is_running(velocity)
            && player.is_on_ground
            && animation.map_or(true, |animation| animation.sprites != SPRITE_IDX_WALKING)
        {
            commands
                .entity(player_entity)
                .insert(Animation::new(SPRITE_IDX_WALKING, CYCLE_DELAY));
        }
    }
}

fn apply_idle_sprite(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &Velocity,
        &mut TextureAtlas,
        &Player,
        &ActionState<Action>,
    )>,
) {
    for (player_entity, velocity, mut sprite, player, action_state) in query.iter_mut() {
        if !is_running(velocity) && player.is_on_ground && !action_state.pressed(&Action::Dive) {
            commands.entity(player_entity).remove::<Animation>();
            sprite.index = SPRITE_IDX_STAND
        }
    }
}

fn apply_fly_sprite(
    mut commands: Commands,
    mut query: Query<(Entity, &Player, Option<&Animation>)>,
) {
    for (player_entity, player, animation) in query.iter_mut() {
        if !player.is_on_ground
            && animation.map_or(true, |animation| animation.sprites != SPRITE_IDX_FLYING)
        {
            commands
                .entity(player_entity)
                .insert(Animation::new(SPRITE_IDX_FLYING, CYCLE_DELAY));
        }
    }
}

fn update_sprite_direction(
    mut query: Query<(&mut Sprite, &Direction, Option<&Children>), With<Player>>,
    mut player_accessories: Query<(&mut Sprite, &mut Transform), (With<Berry>, Without<Player>)>,
) {
    for (mut sprite, direction, maybe_children) in query.iter_mut() {
        let should_flip_x = match direction {
            Direction::Right => true,
            Direction::Left => false,
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

#[derive(Event, Debug, Clone, Copy)]
pub struct SpawnPlayerEvent {
    pub team: Team,
    pub is_queen: bool,
    pub gamepad: Gamepad,
    pub delay: f32,
    pub start_invincible: bool,
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
            Has<Invincible>,
            &ActionState<Action>,
        ),
        With<Player>,
    >,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut ev_knockback: EventWriter<KnockBackEvent>,
    mut queen_deaths: ResMut<QueenDeaths>,
    mut ev_spawn_players: EventWriter<SpawnPlayerEvent>,
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
                                let mut apply_knockback = || {
                                    ev_knockback.send(KnockBackEvent {
                                        entity: left_player_components.0,
                                        direction: Direction::Left,
                                    });
                                    ev_knockback.send(KnockBackEvent {
                                        entity: right_player_components.0,
                                        direction: Direction::Right,
                                    });
                                };
                                let left_player_direction = left_player_components.6;
                                let right_player_direction = right_player_components.6;
                                let left_player_diving =
                                    left_player_components.11.pressed(&Action::Dive);
                                let right_player_diving =
                                    right_player_components.11.pressed(&Action::Dive);
                                match (left_player_diving, right_player_diving) {
                                    (true, true) => {
                                        // queens hit while both diving
                                        apply_knockback();
                                        continue;
                                    }
                                    (true, false) => {
                                        Some((left_player_components, right_player_components))
                                    }
                                    (false, true) => {
                                        Some((right_player_components, left_player_components))
                                    }
                                    (false, false) => {
                                        // neither player is diving
                                        match (left_player_direction, right_player_direction) {
                                            (Direction::Right, Direction::Right) => Some((
                                                right_player_components,
                                                left_player_components,
                                            )),
                                            (Direction::Left, Direction::Left) => Some((
                                                left_player_components,
                                                right_player_components,
                                            )),
                                            _ => {
                                                // hit swords or backs
                                                apply_knockback();
                                                continue;
                                            }
                                        }
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
                        &killed_player_team,
                        _,
                        killed_has_berry,
                        _,
                        _,
                        maybe_riding_on_ship,
                        killed_player_is_queen,
                        killed_player_invincible,
                        _,
                    ) = killed_player_components;
                    if killed_player_invincible {
                        continue;
                    }
                    if killed_player_is_queen {
                        match killed_player_team {
                            Team::Yellow => queen_deaths.yellow_deaths += 1,
                            Team::Purple => queen_deaths.purple_deaths += 1,
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
                    ev_spawn_players.send(SpawnPlayerEvent {
                        team: killed_player_team,
                        is_queen: killed_player_is_queen,
                        gamepad: killed_player.gamepad,
                        delay: RESPAWN_DELAY,
                        start_invincible: true,
                    });
                }
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
    if queen_deaths.yellow_deaths >= 3 {
        ev_win.send(WinEvent {
            team: Team::Purple,
            win_condition,
        });
    }
    if queen_deaths.purple_deaths >= 3 {
        ev_win.send(WinEvent {
            team: Team::Yellow,
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

#[derive(Component)]
struct DelayedPlayerSpawner {
    timer: Timer,
    event: SpawnPlayerEvent,
}

fn add_delayed_player_spawners(
    mut ev_spawn_players: EventReader<SpawnPlayerEvent>,
    mut commands: Commands,
) {
    for ev in ev_spawn_players.read() {
        commands.spawn(DelayedPlayerSpawner {
            timer: Timer::from_seconds(ev.delay, TimerMode::Once),
            event: *ev,
        });
    }
}

fn get_spritesheet(team: Team, is_queen: bool) -> String {
    match (team, is_queen) {
        (Team::Yellow, true) => String::from("spritesheets/queenYellow.png"),
        (Team::Purple, true) => String::from("spritesheets/queenPurple.png"),
        (Team::Yellow, false) => String::from("spritesheets/workerYellow.png"),
        (Team::Purple, false) => String::from("spritesheets/workerPurple.png"),
    }
}

fn spawn_players(
    server: Res<AssetServer>,
    mut atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut commands: Commands,
    mut delayed_player_spawners: Query<(&mut DelayedPlayerSpawner, Entity)>,
    time: Res<Time>,
) {
    for (mut delayed_player_spawner, entity) in &mut delayed_player_spawners {
        delayed_player_spawner.timer.tick(time.delta());

        if delayed_player_spawner.timer.finished() {
            commands.entity(entity).despawn();
            let ev = delayed_player_spawner.event;
            let texture: Handle<Image> = server.load(get_spritesheet(ev.team, ev.is_queen));
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
            if ev.is_queen {
                input_map.insert(
                    Action::Dive,
                    SingleAxis::negative_only(GamepadAxisType::LeftStickY, -0.9),
                );
                input_map.insert(Action::Dive, GamepadButtonType::DPadDown);
            }
            input_map.set_gamepad(ev.gamepad);

            let (player_width, player_height) = if ev.is_queen {
                (QUEEN_RENDER_WIDTH, QUEEN_RENDER_HEIGHT)
            } else {
                (WORKER_RENDER_WIDTH, WORKER_RENDER_HEIGHT)
            };

            let mut player = commands.spawn((
                SpriteSheetBundle {
                    texture,
                    atlas: TextureAtlas {
                        layout: atlas_handle,
                        index: SPRITE_IDX_STAND,
                    },
                    transform: Transform {
                        translation: Vec3::new(
                            match ev.team {
                                Team::Yellow => -WINDOW_WIDTH / 20.0,
                                Team::Purple => WINDOW_WIDTH / 20.0,
                            },
                            WINDOW_TOP_Y - (WINDOW_HEIGHT / 9.0),
                            2.0,
                        ),
                        ..Default::default()
                    },
                    sprite: Sprite {
                        rect: Some(Rect {
                            min: Vec2 {
                                x: SPRITE_PADDING,
                                y: SPRITE_PADDING,
                            },
                            max: Vec2 {
                                x: SPRITE_TILE_WIDTH - SPRITE_PADDING,
                                y: SPRITE_TILE_HEIGHT - SPRITE_PADDING,
                            },
                        }),
                        custom_size: Some(Vec2 {
                            x: player_width,
                            y: player_height,
                        }),
                        ..Default::default()
                    },

                    ..Default::default()
                },
                Player {
                    gamepad: ev.gamepad,
                    is_on_ground: false,
                },
                Name::new("Player"),
                InputManagerBundle::with_map(input_map),
                match ev.team {
                    Team::Yellow => Direction::Left,
                    Team::Purple => Direction::Right,
                },
                ev.team,
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
            if ev.is_queen {
                player.insert(Wings);
                player.insert(Queen);
            }
            if ev.start_invincible {
                player.insert(Invincible {
                    timer: Timer::from_seconds(INVINCIBILITY_DURATION, TimerMode::Once),
                    animation_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
                });
            }
        }
    }
}

fn handle_invincibility(
    mut invincible_players: Query<(Entity, &mut Invincible, &Visibility)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (player_entity, mut invincible, visibility) in &mut invincible_players {
        invincible.timer.tick(time.delta());
        invincible.animation_timer.tick(time.delta());

        if invincible.animation_timer.finished() {
            commands.entity(player_entity).insert(match visibility {
                Visibility::Visible | Visibility::Inherited => Visibility::Hidden,
                Visibility::Hidden => Visibility::Visible,
            });
        }

        if invincible.timer.finished() {
            commands
                .entity(player_entity)
                .insert(Visibility::Visible)
                .remove::<Invincible>();
        }
    }
}

fn reset_all_players(
    players: Query<(Entity, &Player, &Team, Has<Queen>)>,
    mut commands: Commands,
    mut ev_spawn_players: EventWriter<SpawnPlayerEvent>,
) {
    for (entity, player, &team, is_queen) in &players {
        commands.entity(entity).despawn_recursive();
        ev_spawn_players.send(SpawnPlayerEvent {
            team,
            is_queen,
            gamepad: player.gamepad,
            delay: 0.0,
            start_invincible: false,
        });
    }
}
