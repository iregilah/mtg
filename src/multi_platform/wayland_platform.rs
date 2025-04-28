// src/multiplatform/wayland_platform.rs
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
    // Megtarthatjuk a Display-t is, bár az EventQueue már utal rá
    display: Display,
}

impl WaylandPlatform {
    pub fn new() -> Result<Self, String> {
        // Kapcsolódás a Wayland displayhez a $WAYLAND_DISPLAY alapján
        let display = Display::connect_to_env().map_err(|e| format!("Wayland connect error: {}", e))?;
        let mut event_queue = display.create_event_queue();
        let attached_display = (*&display).attach(event_queue.token());  // attach to event queue
        // Globális manager létrehozása a registry kezeléséhez
        let globals = GlobalManager::new(&attached_display);

        // Lefuttatunk egy szinkron kört, hogy a globális objektumok listája betöltődjön
        event_queue.sync_roundtrip(&mut (), |_, _, _| {}).unwrap();

        // wl_seat (input seat) lekérése
        let seat: Main<wl_seat::WlSeat> = globals
            .instantiate_exact(1)  // version 1 seat elég
            .map_err(|_| "wl_seat global not found".to_string())?;

        // Virtual Pointer Manager lekérése (wlroots)
        let pointer_mgr: Main<ZwlrVirtualPointerManagerV1> = globals
            .instantiate_exact(2)  // version 2-t próbáljuk (ha nem menne, lehet 1)
            .map_err(|_| "zwlr_virtual_pointer_manager_v1 global not found (Wayland compositor doesn't support it)".to_string())?;

        // Virtual Keyboard Manager lekérése
        let kb_mgr: Main<ZwpVirtualKeyboardManagerV1> = globals
            .instantiate_exact(1)
            .map_err(|_| "zwp_virtual_keyboard_manager_v1 global not found".to_string())?;

        // Virtuális pointer létrehozása a seat-hez
        let pointer: Main<ZwlrVirtualPointerV1> = pointer_mgr.create_virtual_pointer(&seat);
        // Virtuális billentyűzet létrehozása a seat-hez
        let keyboard: Main<ZwpVirtualKeyboardV1> = kb_mgr.create_virtual_keyboard(&seat);

        // Visszaadjuk az inicializált struktúrát
        Ok(WaylandPlatform { event_queue, pointer, keyboard, display })
    }

    pub fn move_mouse_absolute(&mut self, x: u32, y: u32) -> Result<(), String> {
        // Az abszolút mozgáshoz szükség van a felület méretére (x_extent, y_extent).
        // Egyszerűségképpen feltételezzük, hogy 1920x1080-as képernyő van:
        let (width, height) = (1920, 1080);
        let time = current_millis();
        self.pointer.motion_absolute(time, x, y, width, height);
        self.pointer.frame();  // frame jelzi az események végét
        // Az esemény kiküldéséhez flush-olunk és futtatunk egy roundtrip-et
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
        // A Wayland pointer protokollban a gombkódok megegyeznek a linux evdev/BTN_ számokkal.
        // Tipikusan BTN_LEFT = 0x110 (272), BTN_RIGHT = 0x111 (273), BTN_MIDDLE = 0x112 (274).
        // Egyszerűség kedvéért itt 272=bal, 273=jobb, 274=középső gomb.
        let time = current_millis();
        // Gomb lenyomás (state = 1, azaz pressed)
        self.pointer.button(time, button, 1);
        self.pointer.frame();
        self.flush()?;
        sleep(Duration::from_millis(10));
        let time2 = current_millis();
        self.pointer.button(time2, button, 0);
        self.pointer.frame();
        self.flush()?;
        Ok(())
    }

    pub fn key_press(&mut self, keycode: u32) -> Result<(), String> {
        let time = current_millis();
        self.keyboard.key(time, keycode, KeyState::Pressed as u32);
        self.keyboard.frame();
        self.flush()?;

        sleep(Duration::from_millis(10));
        let time2 = current_millis();
        self.keyboard.key(time2, keycode, KeyState::Released as u32);
        self.keyboard.frame();
        self.flush()?;
        Ok(())
    }

    pub fn key_release(&mut self, keycode: u32) -> Result<(), String> {
        let time = current_millis();
        self.keyboard.key(time, keycode, KeyState::Released as u32);
        self.keyboard.frame();
        self.flush()?;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), String> {
        // Kiürítjük a kliens->szerver üzenet sort
        self.display.flush().map_err(|e| format!("Wayland flush error: {:?}", e))
            .and_then(|_| {
                // A flush után érdemes feldolgozni az esetleg visszajövő eventeket (pl. error).
                // Itt non-blocking módon dispatch-elünk.
                self.event_queue.dispatch_pending(&mut (), |_, _, _| {}).map_err(|e| format!("Wayland dispatch error: {:?}", e))?;
                Ok(())
            })
    }
}

fn current_millis() -> u32 {
    // Wayland időbélyeghez milliseconds since epoch mod 2^32
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u32
}
