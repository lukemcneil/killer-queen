use bevy::{prelude::*, utils::HashSet};
use bevy_rapier2d::dynamics::RigidBody;
use leafwing_input_manager::action_state::ActionState;

use crate::{
    berries::{Berry, BerryBundle},
    gates::{GateBundle, GATE_HEIGHT},
    platforms::{PlatformBundle, PLATFORM_HEIGHT},
    player::{Action, Player, Queen, SpawnPlayerEvent, Team},
    ship::RidingOnShip,
    GameState, WINDOW_BOTTOM_Y, WINDOW_HEIGHT, WINDOW_RIGHT_X, WINDOW_WIDTH,
};

const TEMP_PLATFORM_COLOR: Color = Color::BLACK;
pub struct JoinPlugin;

#[derive(Resource, Default)]
pub struct JoinedGamepads(pub HashSet<Gamepad>);

impl Plugin for JoinPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JoinedGamepads>()
            .add_systems(
                Update,
                (
                    (check_for_start_game, disconnect).run_if(in_state(GameState::Join)),
                    join,
                ),
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
    for sign in [-1.0, 1.0] {
        commands.spawn((
            PlatformBundle::new(
                sign * (WINDOW_RIGHT_X - WINDOW_WIDTH / 40.0 - WINDOW_WIDTH / 10.0
                    + WINDOW_WIDTH / 60.0),
                WINDOW_BOTTOM_Y + 7.0 * WINDOW_HEIGHT / 9.0,
                Vec3::new(
                    (WINDOW_RIGHT_X - WINDOW_WIDTH / 20.0)
                        - (WINDOW_RIGHT_X - WINDOW_WIDTH / 5.0 + WINDOW_WIDTH / 30.0),
                    PLATFORM_HEIGHT / 4.0,
                    1.0,
                ),
                Some(TEMP_PLATFORM_COLOR),
            ),
            TempPlatform,
        ));
        commands.spawn((
            PlatformBundle::new(
                sign * (((WINDOW_WIDTH / 10.0)
                    + (WINDOW_RIGHT_X - WINDOW_WIDTH / 5.0 - WINDOW_WIDTH / 30.0))
                    / 2.0),
                WINDOW_BOTTOM_Y + 7.0 * WINDOW_HEIGHT / 9.0,
                Vec3::new(
                    (WINDOW_RIGHT_X - WINDOW_WIDTH / 5.0 - WINDOW_WIDTH / 30.0)
                        - WINDOW_WIDTH / 10.0,
                    PLATFORM_HEIGHT / 4.0,
                    1.0,
                ),
                Some(TEMP_PLATFORM_COLOR),
            ),
            TempPlatform,
        ));

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

fn join(
    mut joined_gamepads: ResMut<JoinedGamepads>,
    gamepads: Res<Gamepads>,
    button_inputs: Res<ButtonInput<GamepadButton>>,
    queens: Query<&Team, With<Queen>>,
    mut ev_spawn_players: EventWriter<SpawnPlayerEvent>,
) {
    for gamepad in gamepads.iter() {
        // Join the game when both bumpers (L+R) on the controller are pressed
        // We drop down the Bevy's input to get the input from each gamepad
        if button_inputs.any_just_pressed([
            GamepadButton::new(gamepad, GamepadButtonType::LeftTrigger),
            GamepadButton::new(gamepad, GamepadButtonType::RightTrigger),
        ]) {
            let team = if button_inputs
                .just_pressed(GamepadButton::new(gamepad, GamepadButtonType::LeftTrigger))
            {
                Team::Red
            } else {
                Team::Blue
            };
            let is_queen = !queens.iter().any(|&queen_team| queen_team == team);

            // Make sure a player cannot join twice
            if !joined_gamepads.0.contains(&gamepad) {
                ev_spawn_players.send(SpawnPlayerEvent {
                    team,
                    is_queen,
                    gamepad,
                    delay: 0.0,
                    start_invincible: false,
                });
                // Insert the created player and its gamepad to the hashmap of joined players
                // Since uniqueness was already checked above, we can insert here unchecked
                joined_gamepads.0.insert(gamepad);
            }
        }
    }
}

fn disconnect(
    mut commands: Commands,
    action_query: Query<(
        Entity,
        &ActionState<Action>,
        &Player,
        Has<Berry>,
        &Transform,
        Option<&RidingOnShip>,
        &Team,
        Has<Queen>,
    )>,
    mut joined_gamepads: ResMut<JoinedGamepads>,
    asset_server: Res<AssetServer>,
    mut join_gates: Query<(Entity, &Team, &mut Sprite), With<JoinGate>>,
) {
    for (
        player_entity,
        action_state,
        player,
        killed_has_berry,
        killed_player_transform,
        maybe_riding_on_ship,
        team,
        is_queen,
    ) in action_query.iter()
    {
        if action_state.pressed(&Action::Disconnect) {
            joined_gamepads.0.remove(&player.gamepad);
            remove_player(
                &mut commands,
                player_entity,
                killed_has_berry,
                killed_player_transform,
                &asset_server,
                maybe_riding_on_ship,
            );
            if is_queen {
                for (join_gate, join_gate_team, mut gate_sprite) in join_gates.iter_mut() {
                    if join_gate_team == team {
                        commands.entity(join_gate).remove::<Team>();
                        gate_sprite.color = Color::WHITE;
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn remove_player(
    commands: &mut Commands,
    player_entity: Entity,
    has_berry: bool,
    transform: &Transform,
    asset_server: &Res<AssetServer>,
    maybe_riding_on_ship: Option<&RidingOnShip>,
) {
    // Despawn the disconnected player and remove them from the joined player list
    commands.entity(player_entity).despawn_recursive();

    if has_berry {
        commands.spawn(BerryBundle::new(
            transform.translation.x,
            transform.translation.y,
            RigidBody::Dynamic,
            asset_server,
        ));
    }
    if let Some(riding_on_ship) = maybe_riding_on_ship {
        commands.entity(riding_on_ship.ship).remove::<Team>();
    }
}
