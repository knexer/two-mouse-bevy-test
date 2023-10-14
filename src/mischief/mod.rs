use bevy::prelude::*;

use std::error::Error;

#[allow(warnings)]
mod bindings {
    include!("bindings.rs");
}
pub mod manymouse_session;
use manymouse_session::{ManyMouseEvent, ManyMouseSession};

pub struct MischiefPlugin;

impl Plugin for MischiefPlugin {
    fn build(&self, app: &mut App) {
        app.insert_non_send_resource::<MischiefSession>(MischiefSession::new().unwrap())
            .add_event::<MischiefEvent>()
            .add_systems(Update, poll_events);
    }
}

#[derive(Resource)]
pub struct MischiefSession {
    pub session: ManyMouseSession,
}

impl MischiefSession {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        println!("Initializing ManyMouse");
        let session = ManyMouseSession::init()?;
        println!("Found {} mice", session.devices.len());
        Ok(Self { session })
    }
}

#[derive(Event, Debug)]
pub struct MischiefEvent {
    pub device: u32,
    pub event_data: MischiefEventData,
}

#[derive(Debug)]
pub enum MischiefEventData {
    AbsMotion,
    RelMotion { x: i32, y: i32 },
    Button { button: u32, pressed: bool },
    Scroll,
    Disconnect,
}

fn parse_event(event: ManyMouseEvent) -> MischiefEvent {
    let event_data = match event.type_ {
        bindings::ManyMouseEventType_MANYMOUSE_EVENT_ABSMOTION => MischiefEventData::AbsMotion,
        bindings::ManyMouseEventType_MANYMOUSE_EVENT_RELMOTION => {
            let x = event.item == 0;
            MischiefEventData::RelMotion {
                x: if x { event.value } else { 0 },
                y: if !x { event.value } else { 0 },
            }
        }
        bindings::ManyMouseEventType_MANYMOUSE_EVENT_BUTTON => {
            println!("Button event: {:?}", event);
            MischiefEventData::Button {
                button: event.item,
                pressed: event.value == 1,
            }
        }
        bindings::ManyMouseEventType_MANYMOUSE_EVENT_SCROLL => MischiefEventData::Scroll,
        bindings::ManyMouseEventType_MANYMOUSE_EVENT_DISCONNECT => MischiefEventData::Disconnect,
        _ => {
            panic!("Unknown event type");
        }
    };
    MischiefEvent {
        device: event.device,
        event_data,
    }
}

pub fn poll_events(session: NonSend<MischiefSession>, mut events: EventWriter<MischiefEvent>) {
    // println!("Polling events");
    while let Some(event) = session.session.poll_event().unwrap() {
        events.send(parse_event(event));
    }
}
