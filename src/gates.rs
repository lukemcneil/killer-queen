use bevy::{prelude::*, sprite::Anchor};
use bevy_inspector_egui::egui::lerp;
use bevy_rapier2d::prelude::*;

use crate::{
    berries::Berry,
    player::{
        Player, Queen, Team, Wings, PLAYER_COLLIDER_WIDTH_MULTIPLIER, QUEEN_RECT,
        QUEEN_RENDER_HEIGHT, QUEEN_RENDER_WIDTH, WORKER_RENDER_HEIGHT, WORKER_RENDER_WIDTH,
    },
    GameState, WINDOW_BOTTOM_Y, WINDOW_HEIGHT, WINDOW_RIGHT_X, WINDOW_WIDTH,
};

pub struct GatePlugin;

const GATE_WIDTH: f32 = WORKER_RENDER_WIDTH * 1.5;
pub const GATE_HEIGHT: f32 = WORKER_RENDER_HEIGHT * 1.5;
const GATE_TIME: f32 = 1.0;

impl Plugin for GatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Join), setup)
            .add_systems(Update, (check_worker_gate_collisions, progress_gate_timers))
            .add_systems(OnExit(GameState::GameOver), remove_gates);
    }
}

#[derive(Component)]
pub struct Gate;

#[derive(Bundle)]
pub struct GateBundle {
    gate: Gate,
    sprite_bundle: SpriteBundle,
    collider: Collider,
    sensor: Sensor,
}

impl GateBundle {
    pub fn new(x: f32, y: f32, asset_server: &Res<AssetServer>) -> Self {
        let texture = asset_server.load("gate.png");
        Self {
            gate: Gate,
            sprite_bundle: SpriteBundle {
                texture,
                sprite: Sprite {
                    custom_size: Some(Vec2::new(GATE_WIDTH, GATE_HEIGHT)),
                    ..Default::default()
                },
                transform: Transform {
                    translation: Vec3::new(x, y, -11.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            collider: Collider::cuboid(GATE_WIDTH / 2.0, GATE_HEIGHT / 2.0),
            sensor: Sensor,
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(GateBundle::new(
        0.0,
        WINDOW_BOTTOM_Y + 4.0 * WINDOW_HEIGHT / 9.0 + GATE_HEIGHT / 2.0,
        &asset_server,
    ));
    commands.spawn(GateBundle::new(
        WINDOW_RIGHT_X - WINDOW_WIDTH / 3.2,
        WINDOW_BOTTOM_Y + 2.0 * WINDOW_HEIGHT / 9.0 + GATE_HEIGHT / 2.0,
        &asset_server,
    ));
    commands.spawn(GateBundle::new(
        -(WINDOW_RIGHT_X - WINDOW_WIDTH / 3.2),
        WINDOW_BOTTOM_Y + 2.0 * WINDOW_HEIGHT / 9.0 + GATE_HEIGHT / 2.0,
        &asset_server,
    ));
    commands.spawn(GateBundle::new(
        WINDOW_RIGHT_X - WINDOW_WIDTH / 5.0,
        WINDOW_BOTTOM_Y + 7.0 * WINDOW_HEIGHT / 9.0 + GATE_HEIGHT / 2.0,
        &asset_server,
    ));
    commands.spawn(GateBundle::new(
        -(WINDOW_RIGHT_X - WINDOW_WIDTH / 5.0),
        WINDOW_BOTTOM_Y + 7.0 * WINDOW_HEIGHT / 9.0 + GATE_HEIGHT / 2.0,
        &asset_server,
    ));
}

#[derive(Component)]
struct GateTimer {
    timer: Timer,
}

fn check_worker_gate_collisions(
    mut players_with_berries: Query<
        (Has<GateTimer>, Has<Berry>, Has<Queen>, &Team, &mut Sprite),
        With<Player>,
    >,
    mut gates: Query<(Option<&Team>, &mut Sprite), (With<Gate>, Without<Player>)>,
    mut collision_events: EventReader<CollisionEvent>,
    mut commands: Commands,
) {
    for collision_event in collision_events.read() {
        match collision_event {
            CollisionEvent::Started(entity1, entity2, _) => {
                for (gate_entity, player_entity) in [(entity1, entity2), (entity2, entity1)] {
                    if let Ok((maybe_gate_team, mut gate_sprite)) = gates.get_mut(*gate_entity) {
                        if let Ok((player_has_gate_timer, player_has_berry, is_queen, team, _)) =
                            players_with_berries.get(*player_entity)
                        {
                            if is_queen {
                                commands.entity(*gate_entity).insert(*team);
                                gate_sprite.color = team.color();
                            }
                            if let Some(gate_team) = maybe_gate_team {
                                if gate_team != team {
                                    continue;
                                }
                            }
                            if !player_has_gate_timer && player_has_berry {
                                commands.entity(*player_entity).insert(GateTimer {
                                    timer: Timer::from_seconds(GATE_TIME, TimerMode::Once),
                                });
                            }
                        }
                    }
                }
            }
            CollisionEvent::Stopped(entity1, entity2, _) => {
                for (gate_entity, player_entity) in [(entity1, entity2), (entity2, entity1)] {
                    if gates.get(*gate_entity).is_ok() {
                        if let Ok((player_has_gate_timer, player_has_berry, _, _, mut sprite)) =
                            players_with_berries.get_mut(*player_entity)
                        {
                            if player_has_berry && player_has_gate_timer {
                                commands.entity(*player_entity).remove::<GateTimer>();
                                let (player_width, player_height) =
                                    (WORKER_RENDER_WIDTH, WORKER_RENDER_HEIGHT);
                                sprite.custom_size = Some(Vec2 {
                                    x: player_width,
                                    y: player_height,
                                });
                                sprite.anchor = Anchor::Center;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn progress_gate_timers(
    mut commands: Commands,
    mut players_with_gate_timers: Query<(
        (Entity, &mut Sprite, &mut Transform, &Team),
        &mut GateTimer,
    )>,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
) {
    for ((entity, mut sprite, mut transform, team), mut gate_timer) in
        players_with_gate_timers.iter_mut()
    {
        gate_timer.timer.tick(time.delta());

        if gate_timer.timer.finished() {
            let (player_width, player_height) = (QUEEN_RENDER_WIDTH, QUEEN_RENDER_HEIGHT);
            sprite.custom_size = Some(Vec2 {
                x: player_width,
                y: player_height,
            });
            sprite.anchor = Anchor::Center;
            sprite.rect = Some(QUEEN_RECT);
            transform.translation.y += (QUEEN_RENDER_HEIGHT - WORKER_RENDER_HEIGHT) / 2.0;
            commands
                .entity(entity)
                .remove::<GateTimer>()
                .remove::<Berry>()
                .insert(Wings)
                .insert(Collider::cuboid(
                    player_width / 2.0 * PLAYER_COLLIDER_WIDTH_MULTIPLIER,
                    player_height / 2.0,
                ))
                .despawn_descendants();
            commands.entity(entity).insert(match team {
                Team::Orange => asset_server.load::<Image>("spritesheets/fighterYellow.png"),
                Team::Purple => asset_server.load::<Image>("spritesheets/fighterPurple.png"),
            });
        } else {
            // grow sprite
            let percent_done = gate_timer.timer.elapsed_secs() / GATE_TIME;
            let (player_width, player_height) = (
                lerp(WORKER_RENDER_WIDTH..=QUEEN_RENDER_WIDTH, percent_done),
                lerp(WORKER_RENDER_HEIGHT..=QUEEN_RENDER_HEIGHT, percent_done),
            );
            sprite.custom_size = Some(Vec2 {
                x: player_width,
                y: player_height,
            });
            sprite.anchor = Anchor::Custom(
                Vec2::new(0.0, -(player_height - WORKER_RENDER_HEIGHT) / 2.0) / player_height,
            )
        }
    }
}

fn remove_gates(gates: Query<Entity, With<Gate>>, mut commands: Commands) {
    for gate in &gates {
        commands.entity(gate).despawn();
    }
}
