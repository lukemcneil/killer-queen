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
use bevy::{prelude::*, window::WindowResolution};
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
        .add_systems(Update, set_win_text)
        .run();
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    #[default]
    Join,
    Play,
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
    commands.spawn(Camera2dBundle::default());
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

fn set_win_text(
    mut ev_win: EventReader<WinEvent>,
    mut game_over: Local<bool>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    if *game_over {
        return;
    }
    for win_event in ev_win.read() {
        *game_over = true;
        let font = asset_server.load("fonts/FiraSans-Bold.ttf");
        let text_style = TextStyle {
            font: font.clone(),
            font_size: 60.0,
            color: win_event.team.color(),
        };
        commands.spawn(Text2dBundle {
            text: Text::from_section(
                format!(
                    "Team {:?} wins by {:?}",
                    win_event.team, win_event.win_condition
                ),
                text_style.clone(),
            ),
            ..Default::default()
        });
    }
}
