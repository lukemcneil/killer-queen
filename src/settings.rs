use bevy::{input::common_conditions::input_toggle_active, prelude::*};
use bevy_inspector_egui::{bevy_egui::EguiContexts, egui};

use crate::berries::RespawnBerriesEvent;

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            show_game_settings.run_if(input_toggle_active(true, KeyCode::Escape)),
        )
        .init_resource::<GameSettings>();
    }
}

#[derive(Resource)]
pub struct GameSettings {
    pub queen_lives: i32,
    pub ship_speed: f32,
    pub berries_to_win: i32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            queen_lives: 3,
            ship_speed: 30.0,
            berries_to_win: 6,
        }
    }
}

fn show_game_settings(
    mut contexts: EguiContexts,
    mut game_settings: ResMut<GameSettings>,
    mut respawn_berries_ev: EventWriter<RespawnBerriesEvent>,
) {
    egui::Window::new("Settings").show(contexts.ctx_mut(), |ui| {
        ui.add(egui::Slider::new(&mut game_settings.queen_lives, 1..=15).text("queen lives"));
        ui.add(egui::Slider::new(&mut game_settings.ship_speed, 10.0..=200.0).text("ship speed"));
        if ui
            .add(
                egui::Slider::new(&mut game_settings.berries_to_win, 1..=18).text("berries to win"),
            )
            .changed()
        {
            respawn_berries_ev.send(RespawnBerriesEvent);
        }
    });
}
