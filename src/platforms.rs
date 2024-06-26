use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{WINDOW_BOTTOM_Y, WINDOW_HEIGHT, WINDOW_RIGHT_X, WINDOW_TOP_Y, WINDOW_WIDTH};

pub const PLATFORM_HEIGHT: f32 = 20.0;

#[derive(Bundle)]
pub struct PlatformBundle {
    sprite_bundle: SpriteBundle,
    body: RigidBody,
    collider: Collider,
    image_scale_mode: ImageScaleMode,
}

impl PlatformBundle {
    pub fn new(
        x: f32,
        y: f32,
        scale: Vec3,
        is_floor: bool,
        color: Option<Color>,
        asset_server: &Res<AssetServer>,
    ) -> Self {
        let texture = asset_server.load("ground.png");
        Self {
            sprite_bundle: SpriteBundle {
                transform: Transform {
                    translation: Vec3::new(x, y, -10.0),
                    ..Default::default()
                },
                sprite: Sprite {
                    color: color.unwrap_or(Color::WHITE),
                    custom_size: Some(scale.truncate()),
                    ..Default::default()
                },
                texture,
                ..Default::default()
            },
            body: RigidBody::Fixed,
            collider: Collider::cuboid(scale.x / 2.0, scale.y / 2.0),
            image_scale_mode: ImageScaleMode::Tiled {
                tile_x: is_floor,
                tile_y: !is_floor,
                stretch_value: 1.0,
            },
        }
    }
}

pub struct PlatformsPlugin;

impl Plugin for PlatformsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    for sign in [1.0, -1.0] {
        for (x, y, width) in [
            // layer 0
            (0.0, WINDOW_BOTTOM_Y, WINDOW_WIDTH),
            // layer 1
            (
                (WINDOW_RIGHT_X - WINDOW_WIDTH / 24.0),
                WINDOW_BOTTOM_Y + WINDOW_HEIGHT / 9.0,
                WINDOW_WIDTH / 12.0,
            ),
            (
                (WINDOW_RIGHT_X - WINDOW_WIDTH / 5.0),
                WINDOW_BOTTOM_Y + WINDOW_HEIGHT / 9.0,
                WINDOW_WIDTH / 30.0,
            ),
            // layer 2
            (
                (WINDOW_RIGHT_X - WINDOW_WIDTH / 7.0),
                WINDOW_BOTTOM_Y + 2.0 * WINDOW_HEIGHT / 9.0,
                WINDOW_WIDTH / 25.0,
            ),
            (
                (WINDOW_RIGHT_X - WINDOW_WIDTH / 3.2),
                WINDOW_BOTTOM_Y + 2.0 * WINDOW_HEIGHT / 9.0,
                WINDOW_WIDTH / 20.0,
            ),
            // layer 3
            (
                (WINDOW_RIGHT_X - WINDOW_WIDTH / 40.0),
                WINDOW_BOTTOM_Y + 3.0 * WINDOW_HEIGHT / 9.0,
                WINDOW_WIDTH / 20.0,
            ),
            (
                WINDOW_WIDTH / 10.0,
                WINDOW_BOTTOM_Y + 3.0 * WINDOW_HEIGHT / 9.0,
                WINDOW_WIDTH / 20.0,
            ),
            // layer 4
            (
                (WINDOW_RIGHT_X - WINDOW_WIDTH / 5.0),
                WINDOW_BOTTOM_Y + 4.0 * WINDOW_HEIGHT / 9.0,
                WINDOW_WIDTH / 5.0,
            ),
            // layer 5
            (
                (WINDOW_RIGHT_X - WINDOW_WIDTH / 40.0),
                WINDOW_BOTTOM_Y + 5.0 * WINDOW_HEIGHT / 9.0,
                WINDOW_WIDTH / 20.0,
            ),
            (
                WINDOW_WIDTH / 10.0,
                WINDOW_BOTTOM_Y + 5.0 * WINDOW_HEIGHT / 9.0,
                WINDOW_WIDTH / 20.0,
            ),
            (
                (WINDOW_RIGHT_X - WINDOW_WIDTH / 5.0),
                WINDOW_BOTTOM_Y + 5.0 * WINDOW_HEIGHT / 9.0,
                WINDOW_WIDTH / 15.0,
            ),
            // layer 6
            (
                (WINDOW_RIGHT_X - WINDOW_WIDTH / 8.0),
                WINDOW_BOTTOM_Y + 6.0 * WINDOW_HEIGHT / 9.0,
                WINDOW_WIDTH / 25.0,
            ),
            (
                (WINDOW_RIGHT_X - WINDOW_WIDTH / 3.2),
                WINDOW_BOTTOM_Y + 6.0 * WINDOW_HEIGHT / 9.0,
                WINDOW_WIDTH / 25.0,
            ),
            // layer 7
            (
                (WINDOW_RIGHT_X - WINDOW_WIDTH / 40.0),
                WINDOW_BOTTOM_Y + 7.0 * WINDOW_HEIGHT / 9.0,
                WINDOW_WIDTH / 20.0,
            ),
            (
                WINDOW_WIDTH / 20.0,
                WINDOW_BOTTOM_Y + 7.0 * WINDOW_HEIGHT / 9.0,
                WINDOW_WIDTH / 10.0,
            ),
            (
                (WINDOW_RIGHT_X - WINDOW_WIDTH / 5.0),
                WINDOW_BOTTOM_Y + 7.0 * WINDOW_HEIGHT / 9.0,
                WINDOW_WIDTH / 15.0,
            ),
            // layer 8
            (
                (WINDOW_RIGHT_X - WINDOW_WIDTH / 3.2),
                WINDOW_BOTTOM_Y + 8.0 * WINDOW_HEIGHT / 9.0,
                WINDOW_WIDTH / 25.0,
            ),
            // layer 9 (top)
            (0.0, WINDOW_TOP_Y, WINDOW_WIDTH),
        ] {
            commands.spawn(PlatformBundle::new(
                x * sign,
                y,
                Vec3::new(width, PLATFORM_HEIGHT, 1.0),
                true,
                None,
                &asset_server,
            ));
        }
    }
    for (y, width) in [
        // layer 1
        (WINDOW_BOTTOM_Y + WINDOW_HEIGHT / 9.0, WINDOW_WIDTH / 4.0),
        // layer 2
        (
            WINDOW_BOTTOM_Y + 2.0 * WINDOW_HEIGHT / 9.0,
            WINDOW_WIDTH / 20.0,
        ),
        // layer 4
        (
            WINDOW_BOTTOM_Y + 4.0 * WINDOW_HEIGHT / 9.0,
            WINDOW_WIDTH / 20.0,
        ),
    ] {
        commands.spawn(PlatformBundle::new(
            0.0,
            y,
            Vec3::new(width, PLATFORM_HEIGHT, 1.0),
            true,
            None,
            &asset_server,
        ));
    }
    // divider
    commands.spawn(PlatformBundle::new(
        0.0,
        WINDOW_BOTTOM_Y + 8.0 * WINDOW_HEIGHT / 9.0,
        Vec3::new(PLATFORM_HEIGHT, 2.0 * WINDOW_HEIGHT / 9.0, 1.0),
        false,
        None,
        &asset_server,
    ));
    for sign in [-1.0, 1.0] {
        commands.spawn(PlatformBundle::new(
            WINDOW_RIGHT_X * sign,
            WINDOW_BOTTOM_Y + 7.0 * WINDOW_HEIGHT / 9.0,
            Vec3::new(PLATFORM_HEIGHT, 4.0 * WINDOW_HEIGHT / 9.0, 1.0),
            false,
            None,
            &asset_server,
        ));
    }
}
