#![allow(clippy::type_complexity)]

mod animation;
mod berries;
mod gates;
mod join;
mod platforms;
mod player;
mod ship;

use animation::AnimationPlugin;
use berries::BerriesPlugin;
use bevy::{prelude::*, render::camera::ScalingMode, window::WindowResolution};
// use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::prelude::*;
use gates::GatePlugin;
use iyes_perf_ui::{diagnostics::PerfUiEntryFPS, PerfUiPlugin, PerfUiRoot};
use join::JoinPlugin;
use platforms::PlatformsPlugin;
use player::{PlayerPlugin, Team};
use ship::ShipPlugin;

const WINDOW_WIDTH: f32 = 1920.0;
const WINDOW_HEIGHT: f32 = 1016.0;

pub const WINDOW_BOTTOM_Y: f32 = WINDOW_HEIGHT / -2.0;
pub const WINDOW_LEFT_X: f32 = WINDOW_WIDTH / -2.0;
pub const WINDOW_TOP_Y: f32 = WINDOW_HEIGHT / 2.0;
pub const WINDOW_RIGHT_X: f32 = WINDOW_WIDTH / 2.0;

const COLOR_BACKGROUND: Color = Color::rgb(0.5, 0.5, 0.5);

fn main() {
    App::new()
        .insert_resource(ClearColor(COLOR_BACKGROUND))
        .init_state::<GameState>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Platformer".to_string(),
                resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                resizable: true,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins((
            RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0),
            // RapierDebugRenderPlugin::default(),
            PlatformsPlugin,
            PlayerPlugin,
            AnimationPlugin,
            BerriesPlugin,
            ShipPlugin,
            GatePlugin,
            JoinPlugin,
        ))
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(PerfUiPlugin)
        // .add_plugins(WorldInspectorPlugin::new())
        .add_event::<WinEvent>()
        .add_systems(Startup, setup)
        .add_systems(Update, (set_win_text, start_next_game))
        .add_systems(OnExit(GameState::GameOver), remove_win_text)
        .run();
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    #[default]
    Join,
    Play,
    GameOver,
}

fn setup(mut commands: Commands) {
    commands.spawn((
        PerfUiRoot {
            display_labels: false,
            layout_horizontal: true,
            ..default()
        },
        PerfUiEntryFPS::default(),
    ));
    let mut camera = Camera2dBundle::default();
    camera.projection.scaling_mode = ScalingMode::AutoMax {
        max_width: WINDOW_WIDTH,
        max_height: WINDOW_HEIGHT,
    };
    commands.spawn(camera);
}

#[derive(Debug, Clone, Copy)]
pub enum WinCondition {
    Military,
    Economic,
    Ship,
}

#[derive(Event)]
pub struct WinEvent {
    pub team: Team,
    pub win_condition: WinCondition,
}

#[derive(Component)]
struct WinText;

fn set_win_text(
    mut ev_win: EventReader<WinEvent>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if *state.get() != GameState::Play {
        return;
    }
    for win_event in ev_win.read() {
        next_state.set(GameState::GameOver);
        let font = asset_server.load("fonts/FiraSans-Bold.ttf");
        let text_style = TextStyle {
            font: font.clone(),
            font_size: 60.0,
            color: win_event.team.color(),
        };
        commands.spawn((
            WinText,
            Text2dBundle {
                text: Text::from_section(
                    format!(
                        "{:?} victory by {:?}",
                        win_event.win_condition, win_event.team
                    ),
                    text_style.clone(),
                ),
                ..Default::default()
            },
        ));
        commands.spawn(NextGameTimer {
            timer: Timer::from_seconds(3.0, TimerMode::Once),
        });
    }
}

fn remove_win_text(win_texts: Query<Entity, With<WinText>>, mut commands: Commands) {
    for win_text in &win_texts {
        commands.entity(win_text).despawn();
    }
}

#[derive(Component)]
struct NextGameTimer {
    timer: Timer,
}

fn start_next_game(
    mut next_game_timers: Query<(Entity, &mut NextGameTimer)>,
    time: Res<Time>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (entity, mut next_game_timer) in &mut next_game_timers {
        next_game_timer.timer.tick(time.delta());

        if next_game_timer.timer.finished() {
            commands.entity(entity).despawn();
            next_state.set(GameState::Join);
        }
    }
}
