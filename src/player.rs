use std::time::Duration;

use bevy::{prelude::*, utils::HashMap};
use bevy_rapier2d::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::{animation::Animation, WINDOW_BOTTOM_Y, WINDOW_LEFT_X};

const PLAYER_VELOCITY_X: f32 = 400.0;

const SPRITESHEET_COLS: usize = 7;
const SPRITESHEET_ROWS: usize = 8;

const SPRITE_TILE_WIDTH: f32 = 128.0;
const SPRITE_TILE_HEIGHT: f32 = 256.0;
const SPRITE_TILE_ACTUAL_HEIGHT: f32 = 160.0;

const SPRITE_RENDER_WIDTH: f32 = 64.0;
const SPRITE_RENDER_HEIGHT: f32 = 128.0;

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
                        movement,
                        jump,
                        update_sprite_direction,
                        apply_movement_animation,
                        update_direction.after(movement),
                        apply_idle_sprite.after(movement),
                        apply_jump_sprite,
                        join,
                    )
                        .before(disconnect),
                    disconnect,
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
        if button_inputs.pressed(GamepadButton::new(gamepad, GamepadButtonType::LeftTrigger))
            && button_inputs.pressed(GamepadButton::new(gamepad, GamepadButtonType::RightTrigger))
        {
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
                    SingleAxis::symmetric(GamepadAxisType::LeftStickX, 0.1),
                );
                input_map.insert(Action::Disconnect, GamepadButtonType::Select);
                input_map.set_gamepad(gamepad);

                let player = commands
                    .spawn(SpriteSheetBundle {
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
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .insert(Player { gamepad })
                    .insert(Name::new("Player"))
                    .insert(InputManagerBundle::with_map(input_map))
                    .insert(RigidBody::Dynamic)
                    .insert(GravityScale(40.0))
                    .insert(Collider::cuboid(
                        SPRITE_TILE_WIDTH / 4.0,
                        SPRITE_TILE_ACTUAL_HEIGHT / 2.0,
                    ))
                    .insert(Velocity::default())
                    .insert(Direction::Right)
                    .insert(LockedAxes::ROTATION_LOCKED)
                    .insert(Friction {
                        coefficient: 0.0,
                        combine_rule: CoefficientCombineRule::Min,
                    })
                    .id();

                // Insert the created player and its gamepad to the hashmap of joined players
                // Since uniqueness was already checked above, we can insert here unchecked
                joined_players.0.insert_unique_unchecked(gamepad, player);
            }
        }
    }
}

fn movement(mut query: Query<(&ActionState<Action>, &mut Velocity), With<Sprite>>) {
    for (action_state, mut velocity) in query.iter_mut() {
        let mut new_x_velocity = 0.0;

        if action_state.pressed(&Action::Move) {
            new_x_velocity = action_state.clamped_value(&Action::Move) * PLAYER_VELOCITY_X;
        }

        velocity.linvel.x = new_x_velocity;
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
            commands.entity(player_entity).despawn();
            joined_players.0.remove(&player.gamepad);
        }
    }
}

fn jump(mut query: Query<(&ActionState<Action>, &mut Velocity)>) {
    for (action_state, mut velocity) in query.iter_mut() {
        if action_state.just_pressed(&Action::Jump) {
            velocity.linvel.y = 1500.0;
        }
    }
}

fn is_close_to_zero(num: f32) -> bool {
    num.abs() < 10.0
}

fn apply_movement_animation(
    mut commands: Commands,
    query: Query<(Entity, &Velocity), Without<Animation>>,
) {
    for (player, velocity) in query.iter() {
        if velocity.linvel.x != 0.0 && is_close_to_zero(velocity.linvel.y) {
            commands
                .entity(player)
                .insert(Animation::new(SPRITE_IDX_WALKING, CYCLE_DELAY));
        }
    }
}

fn apply_idle_sprite(
    mut commands: Commands,
    mut query: Query<(Entity, &Velocity, &mut TextureAtlas)>,
) {
    for (player, velocity, mut sprite) in query.iter_mut() {
        if velocity.linvel.x == 0.0 && is_close_to_zero(velocity.linvel.y) {
            commands.entity(player).remove::<Animation>();
            sprite.index = SPRITE_IDX_STAND
        }
    }
}

fn apply_jump_sprite(
    mut commands: Commands,
    mut query: Query<(Entity, &Velocity, &mut TextureAtlas)>,
) {
    for (player, velocity, mut sprite) in query.iter_mut() {
        if !is_close_to_zero(velocity.linvel.y) {
            commands.entity(player).remove::<Animation>();
            if velocity.linvel.y > 0.0 {
                sprite.index = SPRITE_IDX_JUMP
            } else {
                sprite.index = SPRITE_IDX_FALL
            }
        }
    }
}

fn update_direction(mut commands: Commands, query: Query<(Entity, &Velocity)>) {
    for (player, velocity) in query.iter() {
        if velocity.linvel.x > 0.0 {
            commands.entity(player).insert(Direction::Right);
        } else if velocity.linvel.x < 0.0 {
            commands.entity(player).insert(Direction::Left);
        }
    }
}

fn update_sprite_direction(mut query: Query<(&mut Sprite, &Direction)>) {
    for (mut sprite, direction) in query.iter_mut() {
        match direction {
            Direction::Right => sprite.flip_x = false,
            Direction::Left => sprite.flip_x = true,
        }
    }
}
