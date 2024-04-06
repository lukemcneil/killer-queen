use std::time::Duration;

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{animation::Animation, WINDOW_BOTTOM_Y, WINDOW_LEFT_X};

const PLAYER_VELOCITY_X: f32 = 400.0;

const SPRITESHEET_COLS: usize = 7;
const SPRITESHEET_ROWS: usize = 8;

const SPRITE_TILE_WIDTH: f32 = 128.0;
const SPRITE_TILE_HEIGHT: f32 = 256.0;

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
        app.add_systems(Startup, setup).add_systems(
            Update,
            (
                movement,
                jump,
                update_direction.after(movement),
                update_sprite_direction,
                apply_movement_animation,
                apply_idle_sprite.after(movement),
                apply_jump_sprite,
            ),
        );
    }
}

#[derive(Component)]
enum Direction {
    Right,
    Left,
}

fn setup(
    mut commands: Commands,
    mut atlases: ResMut<Assets<TextureAtlasLayout>>,
    server: Res<AssetServer>,
) {
    let texture: Handle<Image> = server.load("spritesheets/spritesheet_players.png");
    let texture_atlas = TextureAtlasLayout::from_grid(
        Vec2::new(SPRITE_TILE_WIDTH, SPRITE_TILE_HEIGHT),
        SPRITESHEET_COLS,
        SPRITESHEET_ROWS,
        None,
        None,
    );
    let atlas_handle = atlases.add(texture_atlas);

    commands
        .spawn(SpriteSheetBundle {
            texture,
            atlas: TextureAtlas {
                layout: atlas_handle,
                index: SPRITE_IDX_STAND,
            },
            transform: Transform {
                translation: Vec3::new(WINDOW_LEFT_X + 100.0, WINDOW_BOTTOM_Y + 300.0, 0.0),
                scale: Vec3::new(
                    SPRITE_RENDER_WIDTH / SPRITE_TILE_WIDTH,
                    SPRITE_RENDER_HEIGHT / SPRITE_TILE_HEIGHT,
                    1.0,
                ),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(RigidBody::Dynamic)
        .insert(GravityScale(40.0))
        .insert(Collider::cuboid(
            SPRITE_TILE_WIDTH / 2.0,
            SPRITE_TILE_HEIGHT / 2.0,
        ))
        .insert(Velocity::default())
        .insert(Direction::Right)
        .insert(LockedAxes::ROTATION_LOCKED)
        .insert(Friction {
            coefficient: 0.0,
            combine_rule: CoefficientCombineRule::Min,
        });
}

fn movement(input: Res<ButtonInput<KeyCode>>, mut query: Query<&mut Velocity, With<Sprite>>) {
    let mut velocity = query.single_mut();

    let mut new_x_velocity = 0.0;

    if input.pressed(KeyCode::KeyD) {
        new_x_velocity = PLAYER_VELOCITY_X;
    }

    if input.pressed(KeyCode::KeyA) {
        new_x_velocity = -PLAYER_VELOCITY_X;
    }

    velocity.linvel.x = new_x_velocity;
}

fn jump(input: Res<ButtonInput<KeyCode>>, mut query: Query<&mut Velocity>) {
    if query.is_empty() {
        return;
    }

    let mut velocity = query.single_mut();

    if input.just_pressed(KeyCode::KeyW) {
        velocity.linvel.y = 1500.0;
    }
}

fn is_close_to_zero(num: f32) -> bool {
    num.abs() < 0.00001
}

fn apply_movement_animation(
    mut commands: Commands,
    query: Query<(Entity, &Velocity), Without<Animation>>,
) {
    if query.is_empty() {
        return;
    }

    let (player, velocity) = query.single();
    if velocity.linvel.x != 0.0 && is_close_to_zero(velocity.linvel.y) {
        commands
            .entity(player)
            .insert(Animation::new(SPRITE_IDX_WALKING, CYCLE_DELAY));
    }
}

fn apply_idle_sprite(
    mut commands: Commands,
    mut query: Query<(Entity, &Velocity, &mut TextureAtlas)>,
) {
    if query.is_empty() {
        return;
    }

    let (player, velocity, mut sprite) = query.single_mut();
    if velocity.linvel.x == 0.0 && is_close_to_zero(velocity.linvel.y) {
        commands.entity(player).remove::<Animation>();
        sprite.index = SPRITE_IDX_STAND
    }
}

fn apply_jump_sprite(
    mut commands: Commands,
    mut query: Query<(Entity, &Velocity, &mut TextureAtlas)>,
) {
    if query.is_empty() {
        return;
    }

    let (player, velocity, mut sprite) = query.single_mut();
    if !is_close_to_zero(velocity.linvel.y) {
        commands.entity(player).remove::<Animation>();
        if velocity.linvel.y > 0.0 {
            sprite.index = SPRITE_IDX_JUMP
        } else {
            sprite.index = SPRITE_IDX_FALL
        }
    }
}

fn update_direction(mut commands: Commands, query: Query<(Entity, &Velocity)>) {
    if query.is_empty() {
        return;
    }

    let (player, velocity) = query.single();

    if velocity.linvel.x > 0.0 {
        commands.entity(player).insert(Direction::Right);
    } else if velocity.linvel.x < 0.0 {
        commands.entity(player).insert(Direction::Left);
    }
}

fn update_sprite_direction(mut query: Query<(&mut Sprite, &Direction)>) {
    if query.is_empty() {
        return;
    }

    let (mut sprite, direction) = query.single_mut();

    match direction {
        Direction::Right => sprite.flip_x = false,
        Direction::Left => sprite.flip_x = true,
    }
}
