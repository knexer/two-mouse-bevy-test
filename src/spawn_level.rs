use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_xpbd_2d::prelude::*;

use crate::{
    path::Path,
    player::{Cursor, LeftCursor, PIDController, RightCursor, TargetVelocity},
    PIXELS_PER_METER,
};

pub fn spawn_level(
    mut commands: Commands,
    windows: Query<&Window>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
) {
    spawn_cursors(&mut commands);
    spawn_walls(&mut commands, windows);
    spawn_walls_v2(&mut commands, meshes, materials);
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

fn spawn_walls(commands: &mut Commands, windows: Query<&Window>) {
    let window = windows.single();
    let wall_inset = 0.1;
    let wall_thickness = 0.1;
    let wall_width = window.width() / PIXELS_PER_METER - 2.0 * wall_inset;
    let wall_height = window.height() / PIXELS_PER_METER - 2.0 * wall_inset;

    commands
        .spawn((SpatialBundle::default(), Name::new("Walls")))
        .with_children(|parent| {
            parent.spawn((
                SpriteBundle {
                    transform: Transform::from_xyz(-(wall_width - wall_thickness) / 2.0, 0.0, 0.0),
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(wall_thickness, wall_height)),
                        ..default()
                    },
                    ..default()
                },
                RigidBody::Static,
                Collider::cuboid(wall_thickness, wall_height),
                Name::new("Left wall"),
            ));
            parent.spawn((
                SpriteBundle {
                    transform: Transform::from_xyz((wall_width - wall_thickness) / 2.0, 0.0, 0.0),
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(wall_thickness, wall_height)),
                        ..default()
                    },
                    ..default()
                },
                RigidBody::Static,
                Collider::cuboid(wall_thickness, wall_height),
                Name::new("Right wall"),
            ));
            parent.spawn((
                SpriteBundle {
                    transform: Transform::from_xyz(0.0, -(wall_height - wall_thickness) / 2.0, 0.0),
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(wall_width, wall_thickness)),
                        ..default()
                    },
                    ..default()
                },
                RigidBody::Static,
                Collider::cuboid(wall_width, wall_thickness),
                Name::new("Bottom wall"),
            ));
            parent.spawn((
                SpriteBundle {
                    transform: Transform::from_xyz(0.0, (wall_height - wall_thickness) / 2.0, 0.0),
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(wall_width, wall_thickness)),
                        ..default()
                    },
                    ..default()
                },
                RigidBody::Static,
                Collider::cuboid(wall_width, wall_thickness),
                Name::new("Top wall"),
            ));
        });
}

fn spawn_walls_v2(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    const WIDTH: f32 = 16.0;
    const HEIGHT: f32 = 9.0;

    let mut path = Path::new();
    path.move_to(Vec2::new(0.0, 0.0));
    path.line_to(Vec2::new(0.0, 1.0));
    path.line_to(Vec2::new(0.5, 1.0));
    // path.line_to(Vec2::new(0.5, 0.5));
    // path.line_to(Vec2::new(1.0, 0.5));
    // path.line_to(Vec2::new(1.0, 0.0));
    path.arc_to(Vec2::new(1.0, 0.5), Vec2::new(0.5, 0.5), 10);
    path.arc_to(Vec2::new(0.5, 0.0), Vec2::new(1.0, 0.0), 10);
    path.close();
    path.reverse();

    commands.spawn((
        Name::new("TestShape"),
        RigidBody::Static,
        path.build_collider(),
        MaterialMesh2dBundle {
            transform: Transform::from_xyz(2.0, -2.0, 0.0),
            mesh: meshes.add(path.build_triangle_mesh()).into(),
            material: materials.add(ColorMaterial::from(Color::PURPLE)),
            ..default()
        },
        CollisionLayers::new([Layer::Other], [Layer::Rope, Layer::Other]),
    ));
}
