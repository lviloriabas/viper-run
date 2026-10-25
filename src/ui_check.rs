//! Reproducible Windows/OpenGL menu integration check, using the live input handlers.
//! Run with --ui-check --screen menu --shot menu-check.png --frames 200.
use crate::{App, State, Action, MissionKind, Gamepad, gamepad};
use miniquad::{EventHandler, KeyCode, KeyMods, MouseButton};

fn key(app: &mut App, k: KeyCode) { app.key_down_event(k, KeyMods::default(), false); }
fn select(app: &mut App, action: Action, method: usize) {
    let i = app.buttons.iter().position(|b| b.action == action).expect("menu action visible");
    let b = app.buttons[i];
    match method {
        0 => app.mouse_button_down_event(MouseButton::Left, b.x+b.w*0.5, b.y+b.h*0.5),
        1 => {
            app.selected = i;
            app.pad = Gamepad::new();
            app.pad.state.connected = true;
            app.pad.state.buttons = gamepad::A;
            app.pad_active = true;
            app.pad_buttons();
            assert!(app.pad_active, "controller context lost while activating A");
            app.pad = Gamepad::new();
        },
        _ => { app.selected = i; key(app, KeyCode::Enter); }
    }
}

pub(super) fn step(app: &mut App, frame: u32) -> bool {
    let i = frame as usize - 1;
    if i < MissionKind::ALL.len()*6 {
        let mode = MissionKind::ALL[i/6];
        let method = (i%6)/2;
        if i%2 == 0 {
            assert_eq!(app.state, State::Attract);
            select(app, Action::Pick(mode), method);
        } else {
            assert_eq!(app.state, State::Loadout, "mode must open its setup directly");
            assert_eq!(app.kind, mode);
            println!("PASS mode {:?} via {}", mode, ["click", "controller A", "Enter"][method]);
            key(app, KeyCode::Escape);
            assert_eq!(app.state, State::Attract);
        }
        return false;
    }
    match i-MissionKind::ALL.len()*6 {
        0 => key(app, KeyCode::Escape),
        1 => { assert!(app.controls_open); key(app, KeyCode::Escape); },
        2 => select(app, Action::Pick(MissionKind::Free), 2),
        3 => { assert_eq!(app.state, State::Loadout); select(app, Action::Launch, 0); },
        4 => { assert_eq!(app.state, State::Flying); key(app, KeyCode::Escape); },
        5 => {
            assert!(app.controls_open && app.state == State::Paused);
            app.settings.expo = 0.5; app.settings.sensitivity = 2.0;
            select(app, Action::SensUp, 1);
            assert!((app.settings.expo-0.6).abs() < 1e-8);
            assert_eq!(app.settings.sensitivity, 2.0);
            println!("PASS Escape opens configuration; controller A adjusts expo");
        },
        6 => { key(app, KeyCode::Escape); assert_eq!(app.state, State::Flying); },
        7 => key(app, KeyCode::P),
        8 => { assert_eq!(app.state, State::Frozen); key(app, KeyCode::Escape); },
        9 => { assert!(app.controls_open); key(app, KeyCode::Escape); },
        10 => { assert_eq!(app.state, State::Frozen); app.pause(); },
        11 => { assert_eq!(app.state, State::Paused); key(app, KeyCode::Escape); },
        12 => { assert!(app.controls_open); key(app, KeyCode::Escape); assert_eq!(app.state, State::Frozen); },
        13 => key(app, KeyCode::K),
        14 => select(app, Action::Menu, 0),
        _ => {
            assert!(!app.controls_open && app.state == State::Attract);
            println!("PASS configuration resumes flying/frozen states and Main Menu works");
            assert_eq!(app.performance.frame, frame as u64, "mirror passes counted as display frames");
            println!("PASS one performance sample per display frame, including mirror views");
            println!("PASS menu integration check complete");
            return true;
        }
    }
    false
}
