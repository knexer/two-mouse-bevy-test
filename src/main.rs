use bevy::prelude::*;

mod mischief;

use mischief::{MischiefPlugin, poll_events, MischiefEvent, MischiefEventData};

// Making a game with Bevy + Mischief
// Specifically, a game where you control two ends of a rope with two mice
// Candy falls from the top of the screen, and you have to catch it with the rope
// You can move the rope ends independently, but you can't move them too far apart
// You must deposit the candy in a receptacle on one side of the screen

// Or... what if it's a mouth? Two-mouse pacman controls???
// Could open and close the mouth (by moving cursors together and apart) to move forward,
// and then turn left and right by moving both cursors left and right.
// BONKERS controls lol

// First steps:
// Make two virtual cursors that you can move around (done)
// Assign each cursor to a hand (i.e. click LMB to assign to left hand, RMB to assign to right hand) (done)
// Capture and hide the OS cursor (done)
// Press escape to quit (done)
// Make two rigid bodies that fall from the top of the screen
// Make the bodies dangle from the cursors

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MischiefPlugin)
        .add_systems(Startup, (spawn_camera, hide_os_cursor))
        .add_systems(Update, bevy::window::close_on_esc)
        .add_systems(Update, spawn_left_cursor().run_if(not(any_with_component::<LeftCursor>())))
        .add_systems(Update, spawn_right_cursor().run_if(not(any_with_component::<RightCursor>())))
        .add_systems(Update, move_cursors.after(poll_events))
        .run();
}

fn hide_os_cursor(mut windows: Query<&mut Window>) {
    let mut window = windows.single_mut();
    window.cursor.visible = false;
    window.cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
}

#[derive(Component)]
struct Cursor(u32);

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Component, Default)]
struct LeftCursor;

fn spawn_left_cursor() -> impl Fn(Commands, EventReader<MischiefEvent>) {
    spawn_cursor::<LeftCursor>(Vec2::new(-100.0, 0.0), 0)
}

#[derive(Component, Default)]
struct RightCursor;

fn spawn_right_cursor() -> impl Fn(Commands, EventReader<MischiefEvent>) {
    spawn_cursor::<RightCursor>(Vec2::new(100.0, 0.0), 1)
}

fn spawn_cursor<T>(start_pos: Vec2, button_id: u32) -> impl Fn(Commands, EventReader<MischiefEvent>) where T: Component + Default {
    return move |mut commands: Commands, mut mouse_events: EventReader<MischiefEvent>| {
        for event in mouse_events.iter() {
            match event.event_data {
                MischiefEventData::Button { button, pressed } => {
                    if button == button_id && pressed {
                        commands.spawn((
                            SpriteBundle {
                                transform: Transform::from_xyz(start_pos.x, start_pos.y, 0.0),
                                sprite: Sprite {
                                    custom_size: Some(Vec2::splat(10.0)),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            Cursor(event.device),
                            T::default()
                        ));
                        return;
                    }
                },
                _=> {}
            }
        }
    };
}

fn move_cursors(
    mut mouse_events: EventReader<MischiefEvent>,
    mut cursor_query: Query<(&mut Transform, &Cursor)>
) {
    for event in mouse_events.iter() {
        for (mut transform, cursor) in cursor_query.iter_mut() {
            if cursor.0 == event.device {
                match event.event_data {
                    MischiefEventData::RelMotion { x, y } => {
                        transform.translation.x += x as f32;
                        transform.translation.y -= y as f32;
                    },
                    MischiefEventData::Disconnect => {
                        panic!("Mouse disconnected");
                    }
                    _ => {}
                }
            }
        }
    }
}
