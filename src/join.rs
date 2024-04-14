use bevy::prelude::*;

use crate::{
    gates::{GateBundle, GATE_HEIGHT},
    platforms::{PlatformBundle, PLATFORM_HEIGHT},
    player::Team,
    GameState, WINDOW_BOTTOM_Y, WINDOW_HEIGHT, WINDOW_RIGHT_X, WINDOW_WIDTH,
};

const TEMP_PLATFORM_COLOR: Color = Color::BLACK;
pub struct JoinPlugin;

impl Plugin for JoinPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            check_for_start_game.run_if(in_state(GameState::Join)),
        )
        .add_systems(OnEnter(GameState::Join), setup_join)
        .add_systems(OnExit(GameState::Join), delete_temp_platforms);
    }
}

fn check_for_start_game(
    mut next_state: ResMut<NextState<GameState>>,
    join_gates: Query<Has<Team>, With<JoinGate>>,
) {
    if join_gates.iter().all(|x| x) {
        next_state.set(GameState::Play);
    }
}

#[derive(Component)]
pub struct TempPlatform;

#[derive(Component)]
pub struct JoinGate;

fn setup_join(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        PlatformBundle::new(
            0.0,
            WINDOW_BOTTOM_Y + 7.0 * WINDOW_HEIGHT / 9.0 + PLATFORM_HEIGHT,
            Vec3::new(WINDOW_WIDTH, PLATFORM_HEIGHT, 1.0),
            Some(TEMP_PLATFORM_COLOR),
        ),
        TempPlatform,
    ));

    for sign in [-1.0, 1.0] {
        commands.spawn((
            GateBundle::new(
                (WINDOW_RIGHT_X - WINDOW_WIDTH / 3.2) * sign,
                WINDOW_BOTTOM_Y + 8.0 * WINDOW_HEIGHT / 9.0 + GATE_HEIGHT / 2.0,
                &asset_server,
            ),
            JoinGate,
        ));
    }
}

fn delete_temp_platforms(
    mut commands: Commands,
    temp_platforms: Query<Entity, With<TempPlatform>>,
    join_gates: Query<Entity, With<JoinGate>>,
) {
    for temp_platform in temp_platforms.iter() {
        commands.entity(temp_platform).despawn();
    }
    for join_gate in join_gates.iter() {
        commands.entity(join_gate).despawn();
    }
}
