use std::time::Duration;

use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_xpbd_2d::prelude::*;

use crate::{
    path::{Path, WindDirection},
    player::{Cursor, LeftCursor, PIDController, RightCursor, TargetVelocity},
    ScoreDisplay,
};

pub struct SpawnPlugin;

impl Plugin for SpawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(SpawnState::Settling), spawn_level)
            .add_state::<SpawnState>()
            .insert_resource(SettleTimer(Timer::from_seconds(0.05, TimerMode::Once)))
            .add_systems(Startup, bevy_xpbd_2d::pause)
            .add_systems(OnExit(SpawnState::Settling), bevy_xpbd_2d::resume)
            .add_systems(Update, exit_spawning.run_if(in_state(SpawnState::Settling)));
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum SpawnState {
    #[default]
    Settling,
    Done,
}

#[derive(Resource)]
struct SettleTimer(Timer);

fn exit_spawning(
    mut timer: ResMut<SettleTimer>,
    mut spawn_state: ResMut<NextState<SpawnState>>,
    time: Res<Time>,
) {
    if timer
        .0
        .tick(Duration::from_secs_f32(time.delta_seconds()))
        .just_finished()
    {
        spawn_state.set(SpawnState::Done);
    }
}

pub const WIDTH: f32 = 16.0;
pub const HEIGHT: f32 = 9.0;

const BOTTOM: f32 = -HEIGHT / 2.0;
const TOP: f32 = HEIGHT / 2.0;
const LEFT: f32 = -WIDTH / 2.0;
const RIGHT: f32 = WIDTH / 2.0;

pub const SHAPE_SPAWN_REGION: Rect = Rect {
    min: Vec2::new(-3.0, 5.0),
    max: Vec2::new(3.0, 6.0),
};
pub const SHAPE_ALIVE_REGION: Rect = Rect {
    min: Vec2::new(-WIDTH, -HEIGHT),
    max: Vec2::new(WIDTH, HEIGHT),
};

const OUTER_WALL_THICKNESS: f32 = 0.25;

const BIN_WIDTH: f32 = 1.35;
const BIN_BOTTOM: f32 = BOTTOM + 0.4;
const BIN_TOP: f32 = 0.0;
pub const LEFT_SCORE_REGION: Rect = Rect {
    min: Vec2::new(LEFT + OUTER_WALL_THICKNESS, BIN_BOTTOM),
    max: Vec2::new(LEFT + OUTER_WALL_THICKNESS + BIN_WIDTH, BIN_TOP),
};

pub const RIGHT_SCORE_REGION: Rect = Rect {
    min: Vec2::new(RIGHT - OUTER_WALL_THICKNESS - BIN_WIDTH, BIN_BOTTOM),
    max: Vec2::new(RIGHT - OUTER_WALL_THICKNESS, BIN_TOP),
};

pub fn spawn_level(
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    spawn_cursors(&mut commands);
    spawn_walls(&mut commands, meshes, materials);
    spawn_score_displays(&mut commands, asset_server);
}

#[derive(PhysicsLayer)]
pub enum Layer {
    Rope,
    Level,
    Shapes,
    PlayerBlocker,
}

fn spawn_cursors(mut commands: &mut Commands) {
    // Spawns a rope of this length between two cursor-controlled objects.
    const ROPE_LENGTH: f32 = 4.0;
    // The rope is spawned in a shallow V shape, with this angle to the horizontal.
    // Horizontal is a physically impossible configuration.
    const RELAX_ANGLE_RAD: f32 = 0.4;

    let width = ROPE_LENGTH * RELAX_ANGLE_RAD.cos();
    let left_pos = Vec2::new(-width / 2.0, 0.0);
    let right_pos = Vec2::new(width / 2.0, 0.0);
    let v_bottom = Vec2::new(0.0, -ROPE_LENGTH * RELAX_ANGLE_RAD.sin() / 2.0);

    let player_id = commands
        .spawn((Name::new("Player"), SpatialBundle::default()))
        .id();

    let left_cursor =
        spawn_cursor::<LeftCursor>(&mut commands, player_id, left_pos, None, "Left Cursor");
    let middle_rope = spawn_rope(
        &mut commands,
        player_id,
        left_pos,
        v_bottom,
        10,
        left_cursor,
        Vec2::ZERO,
    );
    let last_rope = spawn_rope(
        &mut commands,
        player_id,
        v_bottom,
        right_pos,
        10,
        middle_rope.0,
        middle_rope.1,
    );
    spawn_cursor::<RightCursor>(
        &mut commands,
        player_id,
        right_pos,
        Some(last_rope),
        "Right Cursor",
    );
}

fn spawn_cursor<T>(
    commands: &mut Commands,
    player_id: Entity,
    start_pos: Vec2,
    connect_to: Option<(Entity, Vec2)>,
    name: &str,
) -> Entity
where
    T: Component + Default,
{
    let cursor_size = 0.3;
    let cursor_id = commands
        .spawn((
            SpriteBundle {
                transform: Transform::from_xyz(start_pos.x, start_pos.y, 0.0),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(cursor_size)),
                    ..default()
                },
                ..default()
            },
            RigidBody::Dynamic,
            TargetVelocity(Vec2::ZERO),
            PIDController {
                p: 1.0,
                i: 1.0,
                d: 0.0,
                max_positional_error: 3.0,
                max_integral_error: 0.5,
                prev_error: Vec2::ZERO,
                integral_error: Vec2::ZERO,
            },
            LinearVelocity::default(),
            ExternalForce::default().with_persistence(false),
            LockedAxes::ROTATION_LOCKED,
            Collider::cuboid(cursor_size, cursor_size),
            CollisionLayers::new(
                [Layer::Rope],
                [Layer::Level, Layer::Shapes, Layer::PlayerBlocker],
            ),
            Cursor(None),
            T::default(),
            Name::new(name.to_owned()),
        ))
        .id();

    commands.entity(player_id).push_children(&[cursor_id]);

    if let Some((entity, prev_anchor)) = connect_to {
        let joint_id = commands
            .spawn((
                RevoluteJoint::new(entity, cursor_id)
                    .with_local_anchor_1(prev_anchor)
                    .with_local_anchor_2(Vec2::new(0.0, 0.0)),
                Name::new("Rope joint final"),
            ))
            .id();
        commands.entity(player_id).push_children(&[joint_id]);
    };

    return cursor_id;
}

fn spawn_rope(
    commands: &mut Commands,
    player_id: Entity,
    start_pos: Vec2,
    end_pos: Vec2,
    num_segments: u32,
    parent_id: Entity,
    parent_anchor: Vec2,
) -> (Entity, Vec2) {
    // Spawn n segments, each of which has some body_length and half of a gap on either side.
    const GAP: f32 = 0.05;
    let per_segment_vector = (end_pos - start_pos) / num_segments as f32;
    let body_length = per_segment_vector.length() - GAP;
    let rotation =
        Quat::from_rotation_z(f32::atan2(end_pos.y - start_pos.y, end_pos.x - start_pos.x));
    const THICKNESS: f32 = 0.05;

    let mut prev_id = parent_id;
    let mut prev_anchor = parent_anchor;
    for i in 0..num_segments {
        let center = start_pos + per_segment_vector * (i as f32 + 0.5);

        let current_id = commands
            .spawn((
                SpriteBundle {
                    transform: Transform::from_xyz(center.x, center.y, 0.0).with_rotation(rotation),
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(body_length, THICKNESS)),
                        ..default()
                    },
                    ..default()
                },
                RigidBody::Dynamic,
                Collider::cuboid(body_length, THICKNESS),
                CollisionLayers::new(
                    [Layer::Rope],
                    [Layer::Level, Layer::Shapes, Layer::PlayerBlocker],
                ),
                Name::new(format!("Rope segment {}", i)),
            ))
            .id();
        commands.entity(player_id).push_children(&[current_id]);

        let joint_id = commands
            .spawn((
                RevoluteJoint::new(prev_id, current_id)
                    .with_local_anchor_1(prev_anchor)
                    .with_local_anchor_2(Vec2::new(-(body_length + GAP) / 2.0, 0.0)),
                Name::new(format!("Rope joint {}", i)),
            ))
            .id();
        commands.entity(player_id).push_children(&[joint_id]);

        prev_anchor = Vec2::new((body_length + GAP) / 2.0, 0.0);
        prev_id = current_id;
    }
    return (prev_id, prev_anchor);
}

fn spawn_walls(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let drain_width: f32 = 2.0;
    let inlet_width: f32 = 8.0;
    let playfield_wall_thickness: f32 = 0.4;
    let playfield_width: f32 =
        WIDTH - (OUTER_WALL_THICKNESS + playfield_wall_thickness + BIN_WIDTH) * 2.0;

    let mut left_side = Path::new();
    left_side.move_to(Vec2::new(LEFT, BOTTOM));
    left_side.line_to(Vec2::new(-drain_width / 2.0, BOTTOM));
    left_side.line_to(Vec2::new(-drain_width / 2.0, BOTTOM + OUTER_WALL_THICKNESS));
    left_side.line_to(Vec2::new(-playfield_width / 2.0, BOTTOM + 1.0));
    left_side.line_to(Vec2::new(-playfield_width / 2.0, BIN_TOP));
    left_side.line_to(Vec2::new(
        -playfield_width / 2.0 - playfield_wall_thickness,
        BIN_TOP,
    ));
    left_side.line_to(Vec2::new(
        -playfield_width / 2.0 - playfield_wall_thickness,
        BIN_BOTTOM,
    ));
    left_side.line_to(Vec2::new(LEFT + OUTER_WALL_THICKNESS, BIN_BOTTOM));
    left_side.line_to(Vec2::new(LEFT + OUTER_WALL_THICKNESS, TOP - 3.0));
    left_side.line_to(Vec2::new(-inlet_width / 2.0, TOP - OUTER_WALL_THICKNESS));
    left_side.line_to(Vec2::new(-inlet_width / 2.0, TOP));
    left_side.line_to(Vec2::new(LEFT, TOP));
    left_side.close();

    commands.spawn((
        Name::new("LeftWall"),
        RigidBody::Static,
        left_side.build_collider(),
        MaterialMesh2dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            mesh: meshes.add(left_side.build_triangle_mesh()).into(),
            material: materials.add(ColorMaterial::from(Color::PURPLE)),
            ..default()
        },
        CollisionLayers::new([Layer::Level], [Layer::Rope, Layer::Shapes]),
    ));

    let mut right_side = Path::new();
    right_side.move_to(Vec2::new(RIGHT, BOTTOM));
    right_side.line_to(Vec2::new(drain_width / 2.0, BOTTOM));
    right_side.line_to(Vec2::new(drain_width / 2.0, BOTTOM + OUTER_WALL_THICKNESS));
    right_side.line_to(Vec2::new(playfield_width / 2.0, BOTTOM + 1.0));
    right_side.line_to(Vec2::new(
        playfield_width / 2.0,
        BIN_TOP - playfield_wall_thickness / 2.0,
    ));
    right_side.arc_to(
        Vec2::new(
            playfield_width / 2.0 + playfield_wall_thickness,
            BIN_TOP - playfield_wall_thickness / 2.0,
        ),
        Vec2::new(
            playfield_width / 2.0 + playfield_wall_thickness / 2.0,
            BIN_TOP - playfield_wall_thickness / 2.0,
        ),
        10,
        WindDirection::Clockwise,
    );
    right_side.line_to(Vec2::new(
        playfield_width / 2.0 + playfield_wall_thickness,
        BIN_BOTTOM + BIN_WIDTH / 2.0,
    ));
    right_side.arc_to(
        Vec2::new(RIGHT - OUTER_WALL_THICKNESS, BIN_BOTTOM + BIN_WIDTH / 2.0),
        Vec2::new(
            playfield_width / 2.0 + playfield_wall_thickness + BIN_WIDTH / 2.0,
            BIN_BOTTOM + BIN_WIDTH / 2.0,
        ),
        10,
        WindDirection::CounterClockwise,
    );
    right_side.line_to(Vec2::new(RIGHT - OUTER_WALL_THICKNESS, TOP - 3.0));
    right_side.line_to(Vec2::new(inlet_width / 2.0, TOP - OUTER_WALL_THICKNESS));
    right_side.line_to(Vec2::new(inlet_width / 2.0, TOP));
    right_side.line_to(Vec2::new(RIGHT, TOP));
    right_side.close();
    right_side.reverse_winding_order();

    commands.spawn((
        Name::new("RightWall"),
        RigidBody::Static,
        right_side.build_collider(),
        MaterialMesh2dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            mesh: meshes.add(right_side.build_triangle_mesh()).into(),
            material: materials.add(ColorMaterial::from(Color::GREEN)),
            ..default()
        },
        CollisionLayers::new([Layer::Level], [Layer::Rope, Layer::Shapes]),
    ));

    // Prevent the player from passing through the inlet.
    let block_thickness = 0.2;
    commands.spawn((
        Name::new("InletBlock"),
        RigidBody::Static,
        Collider::cuboid(inlet_width, block_thickness),
        MaterialMesh2dBundle {
            transform: Transform::from_xyz(0.0, TOP - OUTER_WALL_THICKNESS / 2.0, 0.0),
            mesh: meshes
                .add(
                    shape::Quad {
                        size: Vec2::new(inlet_width, block_thickness),
                        ..default()
                    }
                    .into(),
                )
                .into(),
            material: materials.add(ColorMaterial::from(Color::RED)),
            ..default()
        },
        CollisionLayers::new([Layer::PlayerBlocker], [Layer::Rope]),
    ));

    // Prevent the player from passing through the drain.
    commands.spawn((
        Name::new("DrainBlock"),
        RigidBody::Static,
        Collider::cuboid(drain_width, block_thickness),
        MaterialMesh2dBundle {
            transform: Transform::from_xyz(0.0, BOTTOM + OUTER_WALL_THICKNESS / 2.0, 0.0),
            mesh: meshes
                .add(
                    shape::Quad {
                        size: Vec2::new(drain_width, block_thickness),
                        ..default()
                    }
                    .into(),
                )
                .into(),
            material: materials.add(ColorMaterial::from(Color::RED)),
            ..default()
        },
        CollisionLayers::new([Layer::PlayerBlocker], [Layer::Rope]),
    ));
}

fn spawn_score_displays(commands: &mut Commands, asset_server: Res<AssetServer>) {
    let text_style = TextStyle {
        font: asset_server.load("fonts/Roboto-Regular.ttf"),
        font_size: 100.0,
        color: Color::ANTIQUE_WHITE,
    };

    commands.spawn((
        Text2dBundle {
            transform: Transform::from_xyz(LEFT + 1.0, TOP - 1.0, 1.0)
                .with_scale(Vec3::splat(0.01)),
            text: Text {
                sections: vec![TextSection::new("0", text_style.clone())],
                alignment: TextAlignment::Left,
                linebreak_behavior: bevy::text::BreakLineOn::NoWrap,
            },
            ..default()
        },
        ScoreDisplay::Left,
        Name::new("LeftScoreDisplay"),
    ));

    commands.spawn((
        Text2dBundle {
            transform: Transform::from_xyz(RIGHT - 1.0, TOP - 1.0, 1.0)
                .with_scale(Vec3::splat(0.01)),
            text: Text {
                sections: vec![TextSection::new("0", text_style)],
                alignment: TextAlignment::Right,
                linebreak_behavior: bevy::text::BreakLineOn::NoWrap,
            },
            ..default()
        },
        ScoreDisplay::Right,
        Name::new("RightScoreDisplay"),
    ));
}
