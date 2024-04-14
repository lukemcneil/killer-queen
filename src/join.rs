use bevy::{prelude::*, utils::HashMap};
use bevy_rapier2d::{
    dynamics::{
        Ccd, CoefficientCombineRule, ExternalImpulse, GravityScale, LockedAxes, RigidBody, Velocity,
    },
    geometry::{ActiveEvents, Collider, Friction},
};
use leafwing_input_manager::{
    action_state::ActionState,
    axislike::{SingleAxis, VirtualAxis},
    input_map::InputMap,
    InputManagerBundle,
};

use crate::{
    berries::{Berry, BerryBundle},
    gates::{GateBundle, GATE_HEIGHT},
    platforms::{PlatformBundle, PLATFORM_HEIGHT},
    player::{
        Action, Crown, Direction, Player, Queen, Team, Wings, PLAYER_COLLIDER_WIDTH_MULTIPLIER,
        PLAYER_GRAVITY_SCALE, QUEEN_RENDER_HEIGHT, QUEEN_RENDER_WIDTH, SPRITESHEET_COLS,
        SPRITESHEET_ROWS, SPRITE_IDX_STAND, SPRITE_TILE_ACTUAL_HEIGHT, SPRITE_TILE_HEIGHT,
        SPRITE_TILE_WIDTH, WORKER_RENDER_HEIGHT, WORKER_RENDER_WIDTH,
    },
    ship::RidingOnShip,
    GameState, WINDOW_BOTTOM_Y, WINDOW_HEIGHT, WINDOW_RIGHT_X, WINDOW_TOP_Y, WINDOW_WIDTH,
};

const TEMP_PLATFORM_COLOR: Color = Color::BLACK;
pub struct JoinPlugin;

#[derive(Resource, Default)]
pub struct JoinedPlayers(pub HashMap<Gamepad, Entity>);

impl Plugin for JoinPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JoinedPlayers>()
            .add_systems(
                Update,
                (check_for_start_game, join, disconnect).run_if(in_state(GameState::Join)),
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

fn join(
    mut commands: Commands,
    mut joined_players: ResMut<JoinedPlayers>,
    gamepads: Res<Gamepads>,
    button_inputs: Res<ButtonInput<GamepadButton>>,
    mut atlases: ResMut<Assets<TextureAtlasLayout>>,
    server: Res<AssetServer>,
    queens: Query<&Team, With<Queen>>,
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
            let join_as_queen = !queens.iter().any(|&queen_team| queen_team == team);
            let (player_width, player_height) = if join_as_queen {
                (QUEEN_RENDER_WIDTH, QUEEN_RENDER_HEIGHT)
            } else {
                (WORKER_RENDER_WIDTH, WORKER_RENDER_HEIGHT)
            };

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
                    SingleAxis::symmetric(GamepadAxisType::LeftStickX, 0.5),
                );
                input_map.insert(Action::Move, VirtualAxis::horizontal_dpad());
                input_map.insert(Action::Disconnect, GamepadButtonType::Select);
                input_map.set_gamepad(gamepad);

                let mut player = commands.spawn((
                    SpriteSheetBundle {
                        texture,
                        atlas: TextureAtlas {
                            layout: atlas_handle,
                            index: SPRITE_IDX_STAND,
                        },
                        transform: Transform {
                            translation: Vec3::new(
                                match team {
                                    Team::Red => -WINDOW_WIDTH / 20.0,
                                    Team::Blue => WINDOW_WIDTH / 20.0,
                                },
                                WINDOW_TOP_Y - (WINDOW_HEIGHT / 9.0),
                                2.0,
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
                            custom_size: Some(Vec2 {
                                x: player_width,
                                y: player_height,
                            }),
                            color: team.color(),
                            ..Default::default()
                        },

                        ..Default::default()
                    },
                    Player {
                        gamepad,
                        is_on_ground: false,
                    },
                    Name::new("Player"),
                    InputManagerBundle::with_map(input_map),
                    match team {
                        Team::Red => Direction::Left,
                        Team::Blue => Direction::Right,
                    },
                    team,
                    (
                        RigidBody::Dynamic,
                        GravityScale(PLAYER_GRAVITY_SCALE),
                        Collider::cuboid(
                            player_width / 2.0 * PLAYER_COLLIDER_WIDTH_MULTIPLIER,
                            player_height / 2.0,
                        ),
                        Velocity::default(),
                        ExternalImpulse::default(),
                        LockedAxes::ROTATION_LOCKED,
                        Friction {
                            coefficient: 0.0,
                            combine_rule: CoefficientCombineRule::Min,
                        },
                        ActiveEvents::all(),
                        Ccd::enabled(),
                    ),
                ));
                if join_as_queen {
                    player.insert(Wings);
                    player.insert(Queen);
                }

                if join_as_queen {
                    player.with_children(|children| {
                        let crown_texture: Handle<Image> = server.load("crown.png");
                        children.spawn((
                            SpriteBundle {
                                sprite: Sprite {
                                    custom_size: Some(Vec2::splat(player_width * 1.5)),
                                    ..Default::default()
                                },
                                transform: Transform::from_translation(Vec3 {
                                    x: 0.0,
                                    y: player_height / 2.0,
                                    z: 1.0,
                                }),
                                texture: crown_texture,
                                ..Default::default()
                            },
                            Crown,
                        ));
                    });
                }

                // Insert the created player and its gamepad to the hashmap of joined players
                // Since uniqueness was already checked above, we can insert here unchecked
                joined_players
                    .0
                    .insert_unique_unchecked(gamepad, player.id());
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
    mut joined_players: ResMut<JoinedPlayers>,
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
            joined_players.0.remove(&player.gamepad);
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
