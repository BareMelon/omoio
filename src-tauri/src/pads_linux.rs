//! Read native SDL controller names and GUIDs on Linux.

use super::*;
use sdl2::controller::{Axis, Button, GameController};

fn held(controller: &GameController, by_label: bool) -> Vec<&'static str> {
    let buttons = [
        (Button::A, if by_label { "East" } else { "South" }),
        (Button::B, if by_label { "South" } else { "East" }),
        (Button::X, if by_label { "North" } else { "West" }),
        (Button::Y, if by_label { "West" } else { "North" }),
        (Button::Back, "Back"), (Button::Guide, "Guide"), (Button::Start, "Start"),
        (Button::LeftStick, "LS"), (Button::RightStick, "RS"),
        (Button::LeftShoulder, "LB"), (Button::RightShoulder, "RB"),
        (Button::DPadUp, "Up"), (Button::DPadDown, "Down"),
        (Button::DPadLeft, "Left"), (Button::DPadRight, "Right"),
    ];
    let mut inputs: Vec<_> = buttons.into_iter().filter(|(b, _)| controller.button(*b)).map(|(_, n)| n).collect();
    for (axis, name) in [(Axis::TriggerLeft, "LT"), (Axis::TriggerRight, "RT")] {
        if controller.axis(axis) > 8192 { inputs.push(name); }
    }
    for (axis, plus, minus) in [
        (Axis::LeftX, "LS X+", "LS X-"), (Axis::LeftY, "LS Y-", "LS Y+"),
        (Axis::RightX, "RS X+", "RS X-"), (Axis::RightY, "RS Y-", "RS Y+"),
    ] {
        let value = controller.axis(axis);
        if value > 16000 { inputs.push(plus); }
        else if value < -16000 { inputs.push(minus); }
    }
    inputs
}

pub(super) fn watch() {
    static STARTED: OnceLock<()> = OnceLock::new();
    STARTED.get_or_init(|| {
        std::thread::spawn(|| {
            // Match the HIDAPI drivers explicitly enabled by Cemu v2.6.
            for key in ["PS4", "PS5", "PS4_RUMBLE", "PS5_RUMBLE", "GAMECUBE", "SWITCH", "JOY_CONS", "STADIA", "STEAM", "LUNA"] {
                sdl2::hint::set(&format!("SDL_JOYSTICK_HIDAPI_{key}"), "1");
            }
            sdl2::hint::set("SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS", "1");
            let Ok(sdl) = sdl2::init() else { *seen().lock().unwrap() = Some(Vec::new()); return; };
            let (Ok(controllers), Ok(joysticks), Ok(mut events)) = (sdl.game_controller(), sdl.joystick(), sdl.event_pump()) else {
                *seen().lock().unwrap() = Some(Vec::new()); return;
            };
            let mut opened: HashMap<u32, GameController> = HashMap::new();
            loop {
                for _ in events.poll_iter() {}
                controllers.update();
                opened.retain(|_, c| c.attached());
                let mut names: HashMap<String, usize> = HashMap::new();
                let mut guids: HashMap<String, usize> = HashMap::new();
                let mut now = Vec::new();
                for index in 0..controllers.num_joysticks().unwrap_or(0) {
                    if !controllers.is_game_controller(index) { continue; }
                    // SDL only reads its own device list; a negative id means it disappeared.
                    let instance = unsafe { sdl2::sys::SDL_JoystickGetDeviceInstanceID(index as i32) };
                    if instance < 0 { continue; }
                    let instance = instance as u32;
                    if !opened.contains_key(&instance) {
                        if let Ok(controller) = controllers.open(index) { opened.insert(instance, controller); }
                    }
                    let Some(controller) = opened.get(&instance) else { continue; };
                    let Ok(guid) = joysticks.device_guid(index) else { continue; };
                    let name = controller.name();
                    let name_index = names.entry(name.clone()).or_default();
                    let device = format!("{name} {name_index}");
                    *name_index += 1;
                    let guid = guid.to_string();
                    let guid_index = guids.entry(guid.clone()).or_default();
                    let uuid = format!("{guid_index}_{guid}");
                    *guid_index += 1;
                    let vendor = controller.vendor_id();
                    let by_label = vendor == Some(NINTENDO);
                    now.push(Seen {
                        pad: Pad { device, name, handler: "SDL".into(), family: family_of(vendor).into() },
                        vendor, product: controller.product_id(), held: held(controller, by_label),
                        cemu: crate::backends::cemu::sdl::Found { uuid, by_label },
                    });
                }
                *seen().lock().unwrap() = Some(now);
                std::thread::sleep(Duration::from_millis(16));
            }
        });
    });
    for _ in 0..50 {
        if seen().lock().unwrap().is_some() { return; }
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub(super) fn cemu_pad(device: &str) -> Option<crate::backends::cemu::sdl::Found> {
    watch();
    seen().lock().unwrap().as_ref()?.iter().find(|s| s.pad.device == device).map(|s| s.cemu.clone())
}
