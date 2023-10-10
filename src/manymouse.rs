#[allow(warnings)]
mod bindings {
    include!("bindings.rs");
}

use std::{error::Error, ffi::CStr};

pub use self::bindings::ManyMouseEvent;
use self::bindings::ManyMouseEventType_MANYMOUSE_EVENT_ABSMOTION;

pub struct ManyMouse {
    pub devices: Vec<InputDevice>
}

impl ManyMouse {
    pub fn init() -> Result<Self, Box<dyn Error>> {
        let num_devices: u32 = ManyMouse::call_init()?;
        let mut devices = Vec::new();

        for id in 0..num_devices {
            let name = unsafe {
                let ptr = bindings::ManyMouse_DeviceName(id);
                if ptr.is_null() {
                    return Err("Error getting device name".into());
                }
                CStr::from_ptr(ptr)
            };
            devices.push(InputDevice{id, name: name.to_string_lossy().into_owned()});
        }

        Ok(ManyMouse{devices})
    }

    pub fn poll_event(&self) -> Result<Option<ManyMouseEvent>, Box<dyn Error>> {
        let mut event = ManyMouseEvent::default();
        let poll_response: i32 = unsafe {
            bindings::ManyMouse_PollEvent(&mut event)
        };

        if poll_response == -1 {
            return Err("Error polling ManyMouse".into());
        }

        if poll_response == 0 {
            return Ok(None);
        }

        Ok(Some(event))
    }

    fn call_init() -> Result<u32, Box<dyn Error>> {
        let init_response: i32 = unsafe {
            bindings::ManyMouse_Init()
        };
    
        if init_response == -1 {
            return Err("Error initializing ManyMouse".into());
        }
        Ok(init_response as u32)
    }
}

impl Drop for ManyMouse {
    fn drop(&mut self) {
        unsafe {
            bindings::ManyMouse_Quit()
        };
    }
}

pub struct InputDevice {
    pub id:u32,
    pub name:String,
}

impl Default for ManyMouseEvent {
    fn default() -> Self {
        return ManyMouseEvent {
            type_: ManyMouseEventType_MANYMOUSE_EVENT_ABSMOTION,
            device: 0,
            item: 0,
            value: 0,
            minval: 0,
            maxval: 0,
        };
    }
}
