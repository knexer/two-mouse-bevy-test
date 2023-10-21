use std::time::Duration;

use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use bevy_xpbd_2d::prelude::*;
use rand::Rng;

use crate::{
    spawn_level::{
        Layer, LEFT_SCORE_REGION, PLAY_REGION, RIGHT_SCORE_REGION, SHAPE_ALIVE_REGION,
        SHAPE_SPAWN_REGION,
    },
    AppState, LEFT_COLOR, RIGHT_COLOR,
};

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, configure_shapes)
            .add_systems(
                Update,
                (spawn_shapes, despawn_shapes).run_if(in_state(AppState::Playing)),
            )
            .add_systems(OnEnter(AppState::Playing), start_level)
            .add_systems(Update, detect_game_over.run_if(in_state(AppState::Playing)))
            .add_systems(
                Update,
                (update_score, display_score)
                    .chain()
                    .run_if(in_state(AppState::Playing)),
            )
            .add_systems(Update, display_score.run_if(in_state(AppState::GameOver)));
    }
}

fn start_level(mut commands: Commands, shapes: Query<Entity, With<Shape>>) {
    commands.insert_resource(Score::default());
    commands.insert_resource(LevelState {
        num_shapes_remaining: 10,
    });
    for entity in shapes.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn detect_game_over(
    mut app_state: ResMut<NextState<AppState>>,
    level_state: Res<LevelState>,
    shapes: Query<&Transform, With<Shape>>,
) {
    if level_state.num_shapes_remaining == 0 {
        if shapes.iter().all(|transform| {
            let location = transform.translation.truncate();
            LEFT_SCORE_REGION.contains(location) || RIGHT_SCORE_REGION.contains(location)
        }) {
            println!("Game over!");
            app_state.set(AppState::GameOver);
        }
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    Square,
    Circle,
}

impl std::fmt::Display for Shape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Shape::Square => write!(f, "Square"),
            Shape::Circle => write!(f, "Circle"),
        }
    }
}

#[derive(Component)]
struct ShapeConfig {
    mesh: Mesh2dHandle,
    material: Handle<ColorMaterial>,
    collider: Collider,
    shape: Shape,
}

fn configure_shapes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let default_size = 0.25;
    commands.spawn((
        ShapeConfig {
            mesh: meshes
                .add(
                    shape::Quad {
                        size: Vec2::splat(default_size),
                        ..default()
                    }
                    .into(),
                )
                .into(),
            material: materials.add(ColorMaterial::from(LEFT_COLOR)),
            collider: Collider::cuboid(default_size, default_size),
            shape: Shape::Square,
        },
        Name::new("SquareConfig"),
    ));
    commands.spawn((
        ShapeConfig {
            mesh: meshes
                .add(
                    shape::Circle {
                        radius: default_size / 2.0,
                        ..default()
                    }
                    .into(),
                )
                .into(),
            material: materials.add(ColorMaterial::from(RIGHT_COLOR)),
            collider: Collider::ball(default_size / 2.0),
            shape: Shape::Circle,
        },
        Name::new("CircleConfig"),
    ));
    commands.insert_resource(ShapeTimer(Timer::from_seconds(2.0, TimerMode::Once)));
}

#[derive(Resource)]
struct ShapeTimer(Timer);

fn spawn_shapes(
    mut commands: Commands,
    mut shape_configs: Query<&mut ShapeConfig>,
    mut shape_timer: ResMut<ShapeTimer>,
    mut level_state: ResMut<LevelState>,
    time: Res<Time>,
) {
    if level_state.num_shapes_remaining == 0 {
        return;
    }
    if !shape_timer
        .0
        .tick(Duration::from_secs_f32(time.delta_seconds()))
        .just_finished()
    {
        return;
    }
    level_state.num_shapes_remaining -= 1;

    let mut rng = rand::thread_rng();
    // Pick a random shape config
    let shape_configs = shape_configs.iter_mut().collect::<Vec<_>>();
    let shape_config = &shape_configs[rng.gen_range(0..shape_configs.len())];

    let x = rng.gen_range(SHAPE_SPAWN_REGION.min.x..SHAPE_SPAWN_REGION.max.x);
    let y = rng.gen_range(SHAPE_SPAWN_REGION.min.y..SHAPE_SPAWN_REGION.max.y);
    commands.spawn((
        MaterialMesh2dBundle {
            transform: Transform::from_xyz(x, y, 0.0),
            mesh: shape_config.mesh.clone(),
            material: shape_config.material.clone(),
            ..default()
        },
        RigidBody::Dynamic,
        shape_config.collider.clone(),
        shape_config.shape.clone(),
        CollisionLayers::new([Layer::Shapes], [Layer::Rope, Layer::Level, Layer::Shapes]),
        Name::new(shape_config.shape.to_string()),
    ));

    shape_timer
        .0
        .set_duration(Duration::from_secs_f32(rng.gen_range(1.0..3.0)));
    shape_timer.0.reset();
}

fn despawn_shapes(mut commands: Commands, mut shapes: Query<(Entity, &Transform), With<Shape>>) {
    for (entity, transform) in shapes.iter_mut() {
        if !PLAY_REGION.contains(transform.translation.truncate())
            && !SHAPE_ALIVE_REGION.contains(transform.translation.truncate())
        {
            commands.entity(entity).despawn_recursive();
        }
    }
}

#[derive(Resource, Default)]
struct Score {
    left: i32,
    right: i32,
}

fn update_score(mut score: ResMut<Score>, shapes: Query<(&Transform, &Shape)>) {
    score.left = 0;
    score.right = 0;
    for (transform, shape) in shapes.iter() {
        if LEFT_SCORE_REGION.contains(transform.translation.truncate()) {
            match shape {
                Shape::Square => score.left += 1,
                Shape::Circle => score.left -= 1,
            }
        } else if RIGHT_SCORE_REGION.contains(transform.translation.truncate()) {
            match shape {
                Shape::Square => score.right -= 1,
                Shape::Circle => score.right += 1,
            }
        }
    }
}

#[derive(Component)]
pub enum ScoreDisplay {
    Left,
    Right,
    Sum,
}

fn display_score(score: Res<Score>, mut displays: Query<(&mut Text, &ScoreDisplay)>) {
    for (mut text, display) in displays.iter_mut() {
        text.sections[0].value = match display {
            ScoreDisplay::Left => format!("{}", score.left),
            ScoreDisplay::Right => format!("{}", score.right),
            ScoreDisplay::Sum => format!("{}", score.left + score.right),
        };
    }
}

#[derive(Resource)]
struct LevelState {
    num_shapes_remaining: u32,
}
