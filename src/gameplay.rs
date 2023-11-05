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

const NUM_SHAPES: u32 = 20;

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, configure_shapes)
            .add_systems(OnEnter(AppState::Playing), start_level)
            .add_systems(
                Update,
                (
                    increase_intensity,
                    (spawn_shapes, despawn_shapes),
                    apply_deferred,
                    detect_game_over,
                )
                    .chain()
                    .run_if(in_state(AppState::Playing)),
            )
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
        spawn_state: ShapeSpawnState {
            // Initial one-second delay
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            num_shapes: 0,
            strategy: None,
        },
        num_shapes_remaining: NUM_SHAPES,
        intensity: 0.0,
    });
    for entity in shapes.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn increase_intensity(mut level_state: ResMut<LevelState>) {
    level_state.intensity =
        (NUM_SHAPES - level_state.num_shapes_remaining) as f32 / NUM_SHAPES as f32;
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
}

struct ShapeSpawnState {
    timer: Timer,
    num_shapes: u32,
    strategy: Option<Box<dyn ShapeSpawnStrategy>>,
}

impl ShapeSpawnState {
    fn tick(
        &mut self,
        commands: &mut Commands,
        shape_configs: Query<&ShapeConfig>,
        time: Res<Time>,
        intensity: f32,
    ) -> u32 {
        if !self.timer.tick(time.delta()).just_finished() {
            return 0;
        }

        if self.num_shapes == 0 {
            return 0;
        }

        let strategy = self.strategy.take();

        let (num_shapes, duration) = match strategy {
            Some(mut s) => {
                let result = s.on_timer_finish(self, commands, shape_configs, intensity);
                self.strategy = Some(s);
                result
            }
            None => (0, None),
        };

        self.num_shapes -= num_shapes;
        if let Some(duration) = duration {
            self.timer.set_duration(duration);
            self.timer.reset();
        }

        return num_shapes;
    }

    fn is_done(&self) -> bool {
        self.timer.finished()
    }
}

trait ShapeSpawnStrategy: Send + Sync {
    fn on_timer_finish(
        &mut self,
        state: &ShapeSpawnState,
        commands: &mut Commands,
        shape_configs: Query<&ShapeConfig>,
        intensity: f32,
    ) -> (u32, Option<Duration>);
}

fn interpolate_ranges(
    zero_intensity_range: std::ops::Range<f32>,
    max_intensity_range: std::ops::Range<f32>,
    intensity: f32,
) -> std::ops::Range<f32> {
    zero_intensity_range.start * (1.0 - intensity) + max_intensity_range.start * intensity
        ..zero_intensity_range.end * (1.0 - intensity) + max_intensity_range.end * intensity
}

struct RandomSequence;

impl RandomSequence {
    fn new(num_shapes_remaining: u32, intensity: f32) -> ShapeSpawnState {
        let zero_intensity_timer_range = 2.0..3.0;
        let max_intensity_timer_range = 0.75..1.25;
        let mut rng = rand::thread_rng();
        ShapeSpawnState {
            num_shapes: u32::min(rng.gen_range(1..=3), num_shapes_remaining),
            timer: Timer::from_seconds(
                rng.gen_range(interpolate_ranges(
                    zero_intensity_timer_range,
                    max_intensity_timer_range,
                    intensity,
                )),
                TimerMode::Once,
            ),
            strategy: Some(Box::new(RandomSequence)),
        }
    }
}

// Spawns a sequence of random shapes
impl ShapeSpawnStrategy for RandomSequence {
    fn on_timer_finish(
        &mut self,
        state: &ShapeSpawnState,
        commands: &mut Commands,
        shape_configs: Query<&ShapeConfig>,
        intensity: f32,
    ) -> (u32, Option<Duration>) {
        let mut rng = rand::thread_rng();
        // Pick a random shape config
        let shape_configs = shape_configs.iter().collect::<Vec<_>>();
        let shape_config = &shape_configs[rng.gen_range(0..shape_configs.len())];

        spawn_shape(commands, shape_config);

        let zero_intensity_timer_range = 2.0..3.0;
        let max_intensity_timer_range = 0.75..1.25;
        (
            1,
            match state.num_shapes {
                0 => None,
                _ => Some(Duration::from_secs_f32(rng.gen_range(interpolate_ranges(
                    zero_intensity_timer_range,
                    max_intensity_timer_range,
                    intensity,
                )))),
            },
        )
    }
}

struct Shotgun;

impl Shotgun {
    fn new(num_shapes_remaining: u32, intensity: f32) -> ShapeSpawnState {
        let mut rng = rand::thread_rng();

        let zero_intensity_timer_range = 2.0..3.0;
        let max_intensity_timer_range = 0.75..1.25;

        ShapeSpawnState {
            num_shapes: u32::min(rng.gen_range(2..=3), num_shapes_remaining),
            timer: Timer::from_seconds(
                rng.gen_range(interpolate_ranges(
                    zero_intensity_timer_range,
                    max_intensity_timer_range,
                    intensity,
                )),
                TimerMode::Once,
            ),
            strategy: Some(Box::new(Shotgun)),
        }
    }
}

// Spawns a shotgun blast of shapes of the same type
impl ShapeSpawnStrategy for Shotgun {
    fn on_timer_finish(
        &mut self,
        state: &ShapeSpawnState,
        commands: &mut Commands,
        shape_configs: Query<&ShapeConfig>,
        intensity: f32,
    ) -> (u32, Option<Duration>) {
        let mut rng = rand::thread_rng();
        // Pick a random shape config
        let shape_configs = shape_configs.iter().collect::<Vec<_>>();
        let shape_config = &shape_configs[rng.gen_range(0..shape_configs.len())];

        let zero_intensity_timer_range = 3.0..4.0;
        let max_intensity_timer_range = 1.25..1.75;

        for _ in 0..state.num_shapes {
            spawn_shape(commands, shape_config);
        }
        (
            state.num_shapes,
            Some(Duration::from_secs_f32(rng.gen_range(interpolate_ranges(
                zero_intensity_timer_range,
                max_intensity_timer_range,
                intensity,
            )))),
        )
    }
}

fn spawn_shape(commands: &mut Commands, shape: &ShapeConfig) {
    let mut rng = rand::thread_rng();
    let x = rng.gen_range(SHAPE_SPAWN_REGION.min.x..SHAPE_SPAWN_REGION.max.x);
    let y = rng.gen_range(SHAPE_SPAWN_REGION.min.y..SHAPE_SPAWN_REGION.max.y);
    commands.spawn((
        MaterialMesh2dBundle {
            transform: Transform::from_xyz(x, y, 0.0),
            mesh: shape.mesh.clone(),
            material: shape.material.clone(),
            ..default()
        },
        RigidBody::Dynamic,
        shape.collider.clone(),
        shape.shape.clone(),
        CollisionLayers::new([Layer::Shapes], [Layer::Rope, Layer::Level, Layer::Shapes]),
        Name::new(shape.shape.to_string()),
    ));
}

fn spawn_shapes(
    mut commands: Commands,
    shape_configs: Query<&ShapeConfig>,
    mut level_state: ResMut<LevelState>,
    time: Res<Time>,
) {
    if level_state.num_shapes_remaining == 0 {
        return;
    }
    let intensity = level_state.intensity;
    let num_shapes = level_state
        .spawn_state
        .tick(&mut commands, shape_configs, time, intensity);
    level_state.num_shapes_remaining -= num_shapes;

    if level_state.spawn_state.is_done() {
        let mut rng = rand::thread_rng();

        level_state.spawn_state = match rng.gen_bool((1.0 - intensity) as f64) {
            true => RandomSequence::new(level_state.num_shapes_remaining, level_state.intensity),
            false => Shotgun::new(level_state.num_shapes_remaining, level_state.intensity),
        };
    }
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
    spawn_state: ShapeSpawnState,
    intensity: f32,
}
