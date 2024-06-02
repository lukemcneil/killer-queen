use bevy::{prelude::*, utils::HashSet};
use bevy_midi::input::{MidiData, MidiInput, MidiInputPlugin, MidiInputSettings};
use leafwing_input_manager::action_state::ActionState;

use crate::{
    player::{Action, PlayerController, Queen, SpawnPlayerEvent, Team},
    GameState,
};

pub struct MidiPlugin;

impl Plugin for MidiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (handle_keyboard_presses, connect_to_last_input_port),
        )
        .init_resource::<MidiInputSettings>()
        .init_resource::<JoinedOctaves>()
        .add_plugins(MidiInputPlugin);
    }
}

#[derive(Resource, Default)]
pub struct JoinedOctaves(pub HashSet<u8>);

fn connect_to_last_input_port(input: Res<MidiInput>) {
    if input.ports().len() == 0 {
        return;
    }
    if input.is_changed() {
        if let Some((_, port)) = input.ports().get(input.ports().len() - 1) {
            input.connect(port.clone());
        }
    }
}

fn handle_keyboard_presses(
    mut midi_data: EventReader<MidiData>,
    mut ev_spawn_players: EventWriter<SpawnPlayerEvent>,
    queens: Query<&Team, With<Queen>>,
    mut action_states: Query<(&mut ActionState<Action>, &PlayerController)>,
    mut pressed_keys: Local<HashSet<(u8, u8)>>,
    mut joined_octaves: ResMut<JoinedOctaves>,
    state: Res<State<GameState>>,
) {
    for data in midi_data.read() {
        let [_, index, _value] = data.message.msg;
        let off = index % 12;
        let octave = index.overflowing_div(12).0;

        match off {
            1 | 3 => {
                if data.message.is_note_on() {
                    if joined_octaves.0.contains(&octave) {
                        // player is already in the game
                        if *state.get() != GameState::Join {
                            return;
                        }
                        for (mut action_state, player_controller) in &mut action_states {
                            if let PlayerController::Midi {
                                octave: player_octave,
                            } = player_controller
                            {
                                if *player_octave == octave {
                                    action_state.press(&Action::Disconnect);
                                }
                            }
                        }
                        joined_octaves.0.remove(&octave);
                        return;
                    }
                    let team = if off == 1 { Team::Yellow } else { Team::Purple };
                    let is_queen = !queens.iter().any(|&queen_team| queen_team == team);
                    ev_spawn_players.send(SpawnPlayerEvent {
                        team,
                        is_queen,
                        player_controller: PlayerController::Midi { octave },
                        delay: 0.0,
                        start_invincible: false,
                    });
                    joined_octaves.0.insert(octave);
                }
            }
            // move both direction and dive
            0 | 2 | 5 => {
                if data.message.is_note_on() {
                    pressed_keys.insert((off, octave));
                } else if data.message.is_note_off() {
                    pressed_keys.remove(&(off, octave));
                }
            }
            4 => {
                if data.message.is_note_on() {
                    for (mut action_state, player_controller) in &mut action_states {
                        if let PlayerController::Midi {
                            octave: player_octave,
                        } = player_controller
                        {
                            if *player_octave == octave {
                                action_state.press(&Action::Jump);
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }
    for (pressed_key, octave) in &pressed_keys {
        let mut value = 0.0;
        let mut action = Action::Move;
        match pressed_key {
            0 | 2 => {
                value = if *pressed_key == 0 { -1.0 } else { 1.0 };
                action = Action::Move;
            }
            // 5 => {
            //     value = 1.0;
            //     action = Action::Dive;
            // }
            _ => (),
        }
        for (mut action_state, player_controller) in &mut action_states {
            if let PlayerController::Midi {
                octave: player_octave,
            } = player_controller
            {
                if player_octave == octave {
                    let action_data = action_state.action_data_mut_or_default(&action);
                    // Consumed actions cannot be pressed until they are released
                    if action_data.consumed {
                        return;
                    }
                    if action_data.state.released() {
                        action_data.timing.flip();
                    }
                    action_data.state.press();
                    action_data.value = value;
                }
            }
        }
    }
}
