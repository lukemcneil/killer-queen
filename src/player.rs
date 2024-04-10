use std::time::Duration;

use bevy::{prelude::*, utils::HashMap};
use bevy_rapier2d::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::{animation::Animation, WINDOW_BOTTOM_Y, WINDOW_LEFT_X};

const PLAYER_MAX_VELOCITY_X: f32 = 600.0;
const PLAYER_MIN_VELOCITY_X: f32 = 40.0;
const PLAYER_MAX_VELOCITY_Y: f32 = 600.0;
const PLAYER_FLY_IMPULSE: f32 = 30.0;
const PLAYER_JUMP_IMPULSE: f32 = 60.0;
const PLAYER_MOVEMENT_IMPULSE_GROUND: f32 = 140.0;
const PLAYER_MOVEMENT_IMPULSE_AIR: f32 = 40.0;
const PLAYER_FRICTION_GROUND: f32 = 0.5;
const PLAYER_FRICTION_AIR: f32 = 0.1;
const PLAYER_GRAVITY_SCALE: f32 = 15.0;

const SPRITESHEET_COLS: usize = 7;
const SPRITESHEET_ROWS: usize = 8;

const SPRITE_TILE_WIDTH: f32 = 128.0;
const SPRITE_TILE_HEIGHT: f32 = 256.0;
const SPRITE_TILE_ACTUAL_HEIGHT: f32 = 160.0;

const SPRITE_RENDER_WIDTH: f32 = 32.0;
const SPRITE_RENDER_HEIGHT: f32 = 64.0;

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
                        )
                            .after(check_if_players_on_ground),
                    )
                        .before(disconnect)
                        .before(players_attack),
                    disconnect,
                    players_attack,
                ),
            );
    }
}

#[derive(Resource, Default)]
struct JoinedPlayers(pub HashMap<Gamepad, Entity>);

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
enum Action {
    Move,
    Jump,
    Disconnect,
}

#[derive(Component)]
enum Direction {
    Right,
    Left,
}

#[derive(Component)]
struct Player {
    // This gamepad is used to index each player
    gamepad: Gamepad,
    is_on_ground: bool,
}

#[derive(Component)]
struct Wings;

#[derive(Component)]
struct PlayerBackCollider;

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
        if button_inputs.just_pressed(GamepadButton::new(gamepad, GamepadButtonType::LeftTrigger))
            || button_inputs
                .just_pressed(GamepadButton::new(gamepad, GamepadButtonType::RightTrigger))
        {
            let join_as_queen = button_inputs
                .just_pressed(GamepadButton::new(gamepad, GamepadButtonType::LeftTrigger));
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
                                WINDOW_LEFT_X + 100.0,
                                WINDOW_BOTTOM_Y + 300.0,
                                0.0,
                            ),
                            scale: Vec3::new(
                                SPRITE_RENDER_WIDTH / SPRITE_TILE_WIDTH,
                                SPRITE_RENDER_HEIGHT / SPRITE_TILE_HEIGHT,
                                1.0,
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
                            color: if join_as_queen {
                                Color::GOLD
                            } else {
                                Color::WHITE
                            },
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
                    RigidBody::Dynamic,
                    GravityScale(PLAYER_GRAVITY_SCALE),
                    Collider::cuboid(SPRITE_TILE_WIDTH / 4.0, SPRITE_TILE_ACTUAL_HEIGHT / 2.0),
                    Velocity::default(),
                    ExternalImpulse::default(),
                    Direction::Right,
                    LockedAxes::ROTATION_LOCKED,
                    Friction {
                        coefficient: 0.0,
                        combine_rule: CoefficientCombineRule::Min,
                    },
                    ActiveEvents::CONTACT_FORCE_EVENTS,
                    Ccd::enabled(),
                ));
                if join_as_queen {
                    player.insert(Wings);
                }

                player.with_children(|children| {
                    children
                        .spawn(Collider::ball(SPRITE_RENDER_WIDTH / 2.0))
                        .insert(TransformBundle::from_transform(Transform::from_xyz(
                            0.0,
                            SPRITE_TILE_ACTUAL_HEIGHT / 2.0,
                            0.0,
                        )))
                        .insert(Sensor)
                        .insert(ActiveEvents::COLLISION_EVENTS)
                        .insert(PlayerBackCollider);
                });

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
    action_query: Query<(&ActionState<Action>, &Player)>,
    mut joined_players: ResMut<JoinedPlayers>,
) {
    for (action_state, player) in action_query.iter() {
        if action_state.pressed(&Action::Disconnect) {
            let player_entity = *joined_players.0.get(&player.gamepad).unwrap();

            // Despawn thea disconnected player and remove them from the joined player list
            commands.entity(player_entity).despawn_recursive();
            joined_players.0.remove(&player.gamepad);
        }
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
    mut query: Query<(&mut Sprite, &Direction, &Children)>,
    mut back_colliders: Query<&mut Transform, With<PlayerBackCollider>>,
) {
    for (mut sprite, direction, children) in query.iter_mut() {
        match direction {
            Direction::Right => sprite.flip_x = false,
            Direction::Left => sprite.flip_x = true,
        }
        for child in children {
            let mut transform = back_colliders
                .get_mut(*child)
                .expect("player should have collider");
            transform.translation.x = match direction {
                Direction::Right => -SPRITE_TILE_WIDTH / 4.0,
                Direction::Left => SPRITE_TILE_WIDTH / 4.0,
            };
        }
    }
}

fn players_attack(
    mut collision_events: EventReader<CollisionEvent>,
    mut commands: Commands,
    mut joined_players: ResMut<JoinedPlayers>,
    collider_parents: Query<&Parent, With<PlayerBackCollider>>,
    players: Query<(&Player, Option<&Wings>)>,
) {
    for collision_event in collision_events.read() {
        if let CollisionEvent::Started(entity1, entity2, _flags) = collision_event {
            let (back_collider, killer) = if players.get(*entity1).is_ok() {
                (entity2, entity1)
            } else if players.get(*entity2).is_ok() {
                (entity1, entity2)
            } else {
                // neither is a player, collision between sensors
                continue;
            };

            if let Ok(killed_player) = collider_parents.get(*back_collider) {
                let killer_has_wings = players.get(*killer).unwrap().1.is_some();
                if killer_has_wings {
                    commands.entity(killed_player.get()).despawn_recursive();
                    joined_players.0.remove(
                        &players
                            .get(killed_player.get())
                            .expect("killed should have player component")
                            .0
                            .gamepad,
                    );
                }
            }
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
