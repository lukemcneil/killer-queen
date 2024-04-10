use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{WINDOW_HEIGHT, WINDOW_WIDTH};

const COLOR_PLATFORM: Color = Color::rgb(0.29, 0.31, 0.41);

#[derive(Bundle)]
struct PlatformBundle {
    sprite_bundle: SpriteBundle,
    body: RigidBody,
    collider: Collider,
}

impl PlatformBundle {
    fn new(x: f32, y: f32, scale: Vec3) -> Self {
        Self {
            sprite_bundle: SpriteBundle {
                sprite: Sprite {
                    color: COLOR_PLATFORM,
                    ..Default::default()
                },
                transform: Transform {
                    translation: Vec3::new(x, y, 0.0),
                    scale,
                    ..Default::default()
                },
                ..Default::default()
            },
            body: RigidBody::Fixed,
            collider: Collider::cuboid(0.5, 0.5),
        }
    }
}

pub struct PlatformsPlugin;

impl Plugin for PlatformsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(PlatformBundle::new(
        -100.0,
        0.0,
        Vec3::new(WINDOW_WIDTH / 2.0, 20.0, 1.0),
    ));
    commands.spawn(PlatformBundle::new(
        300.0,
        -WINDOW_HEIGHT / 4.0,
        Vec3::new(WINDOW_WIDTH / 3.0, 20.0, 1.0),
    ));
    commands.spawn(PlatformBundle::new(
        -300.0,
        -WINDOW_HEIGHT / 4.0,
        Vec3::new(WINDOW_WIDTH / 3.0, 100.0, 1.0),
    ));
    // commands.spawn(PlatformBundle::new(100.0, 0.0, Vec3::new(50.0, 350.0, 1.0)));
    // commands.spawn(PlatformBundle::new(
    //     350.0,
    //     0.0,
    //     Vec3::new(150.0, 250.0, 1.0),
    // ));
}
