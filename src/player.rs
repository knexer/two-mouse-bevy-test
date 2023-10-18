use bevy::{input::common_conditions::input_toggle_active, prelude::*};
use bevy_xpbd_2d::prelude::*;

use crate::{
    mischief::{poll_events, MischiefEvent, MischiefEventData, MischiefPlugin},
    AppState, PIXELS_PER_METER,
};

#[derive(Component)]
pub struct Cursor(pub Option<u32>);

#[derive(Component, Default)]
pub struct LeftCursor;

#[derive(Component, Default)]
pub struct RightCursor;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MischiefPlugin)
            .register_type::<TargetVelocity>()
            .add_systems(Update, attach_cursors)
            .add_systems(
                Update,
                move_cursors
                    .after(poll_events)
                    .run_if(input_toggle_active(true, KeyCode::Grave)),
            )
            .add_systems(
                FixedUpdate,
                apply_cursor_force
                    .before(PhysicsSet::Prepare)
                    .run_if(in_state(AppState::Playing)),
            );
    }
}

fn attach_cursors(
    mut mouse_events: EventReader<MischiefEvent>,
    mut left_cursors: Query<&mut Cursor, (With<LeftCursor>, Without<RightCursor>)>,
    mut right_cursors: Query<&mut Cursor, (With<RightCursor>, Without<LeftCursor>)>,
) {
    let left_cursor_device = left_cursors.single().0;
    let right_cursor_device = right_cursors.single().0;
    for event in mouse_events.iter() {
        match event.event_data {
            MischiefEventData::Button {
                button: 0,
                pressed: true,
            } => {
                if left_cursor_device == None && right_cursor_device != Some(event.device) {
                    let mut cursor = left_cursors.single_mut();
                    cursor.0 = Some(event.device);
                }
            }
            MischiefEventData::Button {
                button: 1,
                pressed: true,
            } => {
                if right_cursor_device == None && left_cursor_device != Some(event.device) {
                    let mut cursor = right_cursors.single_mut();
                    cursor.0 = Some(event.device);
                }
            }
            _ => {}
        }
    }
}

fn move_cursors(
    mut mouse_events: EventReader<MischiefEvent>,
    mut cursor_query: Query<(&mut TargetVelocity, &Cursor)>,
    time: Res<Time>,
) {
    for (mut target_velocity, _) in cursor_query.iter_mut() {
        target_velocity.0 = Vec2::ZERO;
    }

    for event in mouse_events.iter() {
        for (mut target_velocity, cursor) in cursor_query.iter_mut() {
            if cursor.0 == Some(event.device) {
                match event.event_data {
                    MischiefEventData::RelMotion { x, y } => {
                        target_velocity.0 += Vec2::new(x as f32, -y as f32)
                            / (PIXELS_PER_METER * time.delta_seconds());
                    }
                    MischiefEventData::Disconnect => {
                        panic!("Mouse disconnected");
                    }
                    _ => {}
                }
            }
        }
    }
}

#[derive(Component, Reflect, Debug, Default)]
pub struct TargetVelocity(pub Vec2);

#[derive(Component)]
pub struct PIDController {
    pub p: f32,
    pub i: f32,
    pub d: f32,
    pub max_positional_error: f32,
    pub max_integral_error: f32,
    pub integral_error: Vec2,
    pub prev_error: Vec2,
}

fn apply_cursor_force(
    mut cursors: Query<(
        &TargetVelocity,
        &mut PIDController,
        &Mass,
        &LinearVelocity,
        &mut ExternalForce,
    )>,
    time: Res<FixedTime>,
) {
    for (target_velocity, mut pd, mass, velocity, mut force) in cursors.iter_mut() {
        let error = target_velocity.0 - velocity.0;

        pd.integral_error += error * time.period.as_secs_f32();
        pd.integral_error = pd.integral_error.clamp_length_max(pd.max_integral_error);
        let d_error = (error - pd.prev_error) / time.period.as_secs_f32();
        let u_pd = pd.p * error.clamp_length_max(pd.max_positional_error)
            + pd.i * pd.integral_error
            + pd.d * d_error;

        let applied_acceleration = u_pd / time.period.as_secs_f32();
        force.apply_force(mass.0 * applied_acceleration);

        pd.prev_error = error;
    }
}
