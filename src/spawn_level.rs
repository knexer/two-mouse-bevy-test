use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_xpbd_2d::prelude::*;

use crate::{
    path::{Path, WindDirection},
    player::{Cursor, LeftCursor, PIDController, RightCursor, TargetVelocity},
};

pub const WIDTH: f32 = 16.0;
pub const HEIGHT: f32 = 9.0;

pub const SHAPE_SPAWN_REGION: Rect = Rect {
    min: Vec2::new(-3.0, 5.0),
    max: Vec2::new(3.0, 6.0),
};
pub const SHAPE_ALIVE_REGION: Rect = Rect {
    min: Vec2::new(-WIDTH, -HEIGHT),
    max: Vec2::new(WIDTH, HEIGHT),
};

pub fn spawn_level(
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
) {
    spawn_cursors(&mut commands);
    spawn_walls(&mut commands, meshes, materials);
}

#[derive(PhysicsLayer)]
enum Layer {
    Rope,
    Other,
}

fn spawn_cursors(mut commands: &mut Commands) {
    let left_pos = Vec2::new(-2.0, 0.0);
    let right_pos = Vec2::new(2.0, 0.0);
    let player_id = commands
        .spawn((Name::new("Player"), SpatialBundle::default()))
        .id();

    let left_cursor =
        spawn_cursor::<LeftCursor>(&mut commands, player_id, left_pos, None, "Left Cursor");
    let last_rope = spawn_rope(
        &mut commands,
        player_id,
        left_pos,
        right_pos,
        20,
        left_cursor,
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
            CollisionLayers::new([Layer::Rope], [Layer::Other]),
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
) -> (Entity, Vec2) {
    // Total width of n segments: width = (n + 1) * GAP + n * body_size
    // Solving for body_size: body_size = (width - (n + 1) * GAP) / n
    const GAP: f32 = 0.05;
    let total_gap_width = (num_segments + 1) as f32 * GAP;
    let body_length = ((end_pos.x - start_pos.x) - total_gap_width) / num_segments as f32;
    const THICKNESS: f32 = 0.05;

    let mut prev_id = parent_id;
    let mut prev_anchor = Vec2::new(0.0, 0.0);
    for i in 0..num_segments {
        let dx = (i as f32 + 1.0) * GAP + (i as f32) * body_length;

        let current_id = commands
            .spawn((
                SpriteBundle {
                    transform: Transform::from_xyz(
                        start_pos.x + dx + body_length / 2.0,
                        start_pos.y,
                        0.0,
                    ),
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(body_length, THICKNESS)),
                        ..default()
                    },
                    ..default()
                },
                RigidBody::Dynamic,
                Collider::cuboid(body_length, THICKNESS),
                CollisionLayers::new([Layer::Rope], [Layer::Other]),
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
    let bottom: f32 = -HEIGHT / 2.0;
    let top: f32 = HEIGHT / 2.0;
    let left: f32 = -WIDTH / 2.0;
    let right: f32 = WIDTH / 2.0;

    let drain_width: f32 = 2.0;
    let playfield_width: f32 = 12.0;
    let target_bottom: f32 = 1.0;
    let playfield_wall_thickness: f32 = 0.4;
    let bin_bottom: f32 = bottom + 0.4;
    let inlet_width: f32 = 8.0;
    let outer_wall_thickness: f32 = 0.25;

    let mut left_side = Path::new();
    left_side.move_to(Vec2::new(left, bottom));
    left_side.line_to(Vec2::new(-drain_width / 2.0, bottom));
    left_side.line_to(Vec2::new(-drain_width / 2.0, bottom + outer_wall_thickness));
    left_side.line_to(Vec2::new(-playfield_width / 2.0, bottom + 1.0));
    left_side.line_to(Vec2::new(-playfield_width / 2.0, target_bottom));
    left_side.line_to(Vec2::new(
        -playfield_width / 2.0 - playfield_wall_thickness,
        target_bottom,
    ));
    left_side.line_to(Vec2::new(
        -playfield_width / 2.0 - playfield_wall_thickness,
        bin_bottom,
    ));
    left_side.line_to(Vec2::new(left + outer_wall_thickness, bin_bottom));
    left_side.line_to(Vec2::new(left + outer_wall_thickness, top - 3.0));
    left_side.line_to(Vec2::new(-inlet_width / 2.0, top));
    left_side.line_to(Vec2::new(left, top));
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
        CollisionLayers::new([Layer::Other], [Layer::Rope, Layer::Other]),
    ));

    let mut right_side = Path::new();
    right_side.move_to(Vec2::new(right, bottom));
    right_side.line_to(Vec2::new(drain_width / 2.0, bottom));
    right_side.line_to(Vec2::new(drain_width / 2.0, bottom + outer_wall_thickness));
    right_side.line_to(Vec2::new(playfield_width / 2.0, bottom + 1.0));
    right_side.line_to(Vec2::new(
        playfield_width / 2.0,
        target_bottom - playfield_wall_thickness / 2.0,
    ));
    right_side.arc_to(
        Vec2::new(
            playfield_width / 2.0 + playfield_wall_thickness,
            target_bottom - playfield_wall_thickness / 2.0,
        ),
        Vec2::new(
            playfield_width / 2.0 + playfield_wall_thickness / 2.0,
            target_bottom - playfield_wall_thickness / 2.0,
        ),
        10,
        WindDirection::Clockwise,
    );
    let bin_width =
        (WIDTH - playfield_width) / 2.0 - playfield_wall_thickness - outer_wall_thickness;
    right_side.line_to(Vec2::new(
        playfield_width / 2.0 + playfield_wall_thickness,
        bin_bottom + bin_width / 2.0,
    ));
    right_side.arc_to(
        Vec2::new(right - outer_wall_thickness, bin_bottom + bin_width / 2.0),
        Vec2::new(
            playfield_width / 2.0 + playfield_wall_thickness + bin_width / 2.0,
            bin_bottom + bin_width / 2.0,
        ),
        10,
        WindDirection::CounterClockwise,
    );
    right_side.line_to(Vec2::new(right - outer_wall_thickness, top - 3.0));
    right_side.line_to(Vec2::new(inlet_width / 2.0, top));
    right_side.line_to(Vec2::new(right, top));
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
        CollisionLayers::new([Layer::Other], [Layer::Rope, Layer::Other]),
    ));
}
