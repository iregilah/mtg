#![cfg(target_os = "linux")]

use wayland_client::platform::WaylandPlatform;
use wayland_client::{Display, EventQueue, GlobalManager, Main};
use wayland_client::protocol::{wl_seat, wl_surface};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    ZwlrVirtualPointerManagerV1, ZwlrVirtualPointerV1,
};
use wayland_protocols_misc::zwp_virtual_keyboard::v1::client::{
    ZwpVirtualKeyboardManagerV1, ZwpVirtualKeyboardV1,
};
use wayland_client::protocol::wl_keyboard::KeyState;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{thread::sleep, time::Duration};

pub struct WaylandPlatform {
    event_queue: EventQueue,
    pointer: Main<ZwlrVirtualPointerV1>,
    keyboard: Main<ZwpVirtualKeyboardV1>,
    display: Display,  // Retain display handle for flushing events
}

impl WaylandPlatform {
    pub fn new() -> Result<Self, String> {
        // Connect to Wayland display using $WAYLAND_DISPLAY
        let display = Display::connect_to_env()
            .map_err(|e| format!("Wayland connect error: {}", e))?;
        let mut event_queue = display.create_event_queue();
        let attached_display = display.attach(event_queue.token());
        // Create global manager for registry handling
        let globals = GlobalManager::new(&attached_display);

        // Run sync roundtrip to populate globals
        event_queue.sync_roundtrip(&mut (), |_, _, _| {}).unwrap();

        // Get wl_seat (input seat)
        let seat: Main<wl_seat::WlSeat> = globals
            .instantiate_exact(1)
            .map_err(|_| "wl_seat global not found".to_string())?;

        // Get virtual pointer manager (wlroots)
        let pointer_mgr: Main<ZwlrVirtualPointerManagerV1> = globals
            .instantiate_exact(2)
            .map_err(|_| "zwlr_virtual_pointer_manager_v1 global not found".to_string())?;

        // Get virtual keyboard manager
        let kb_mgr: Main<ZwpVirtualKeyboardManagerV1> = globals
            .instantiate_exact(1)
            .map_err(|_| "zwp_virtual_keyboard_manager_v1 global not found".to_string())?;

        // Create virtual pointer for seat
        let pointer: Main<ZwlrVirtualPointerV1> = pointer_mgr.create_virtual_pointer(&seat);
        // Create virtual keyboard for seat
        let keyboard: Main<ZwpVirtualKeyboardV1> = kb_mgr.create_virtual_keyboard(&seat);

        Ok(WaylandPlatform { event_queue, pointer, keyboard, display })
    }

    pub fn move_mouse_absolute(&mut self, x: u32, y: u32) -> Result<(), String> {
        // Absolute move assumes 1920x1080 display for simplicity
        let (width, height) = (1920, 1080);
        let time = current_millis();
        self.pointer.motion_absolute(time, x, y, width, height);
        self.pointer.frame();  // Signal end of events
        // Flush and dispatch to send events
        self.flush()?;
        Ok(())
    }

    pub fn move_mouse_relative(&mut self, dx: i32, dy: i32) -> Result<(), String> {
        let time = current_millis();
        self.pointer.motion(time, (dx as f64).into(), (dy as f64).into());
        self.pointer.frame();
        self.flush()?;
        Ok(())
    }

    pub fn mouse_click(&mut self, button: u32) -> Result<(), String> {
        // Wayland button codes match evdev: 272=left, 273=right, 274=middle
        let time = current_millis();
        // Press button (state=1)
        self.pointer.button(time, button, 1);
        self.pointer.frame();
        self.flush()?;
        sleep(Duration::from_millis(10));  // Short delay
        let time2 = current_millis();
        // Release button (state=0)
        self.pointer.button(time2, button, 0);
        self.pointer.frame();
        self.flush()?;
        Ok(())
    }

    pub fn key_press(&mut self, keycode: u32) -> Result<(), String> {
        let time = current_millis();
        // Press key
        self.keyboard.key(time, keycode, KeyState::Pressed as u32);
        self.keyboard.frame();
        self.flush()?;

        sleep(Duration::from_millis(10));  // Short delay
        let time2 = current_millis();
        // Release key
        self.keyboard.key(time2, keycode, KeyState::Released as u32);
        self.keyboard.frame();
        self.flush()?;
        Ok(())
    }

    pub fn key_release(&mut self, keycode: u32) -> Result<(), String> {
        let time = current_millis();
        // Release key
        self.keyboard.key(time, keycode, KeyState::Released as u32);
        self.keyboard.frame();
        self.flush()?;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), String> {
        // Flush client-to-server queue
        self.display
            .flush()
            .map_err(|e| format!("Wayland flush error: {:?}", e))
            .and_then(|_| {
                // Dispatch pending events non-blocking
                self.event_queue
                    .dispatch_pending(&mut (), |_, _, _| {})
                    .map_err(|e| format!("Wayland dispatch error: {:?}", e))?;
                Ok(())
            })
    }
}

fn current_millis() -> u32 {
    // Milliseconds since epoch mod 2^32 for Wayland timestamps
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u32
}