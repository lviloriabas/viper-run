//! Menu screens, the controls drawer and the flight overlays, drawn with the
//! pixel font. Screens return clickable buttons for mouse and keyboard use.
use crate::air::AirData;
use crate::canvas::{rgba, with_alpha, Align, Canvas, Col};
use crate::cockpit::pad;
use crate::combat::{Config, MissionKind, Store};
use crate::keymap::{key_name, Keymap, BINDS};
use crate::math::*;
use crate::settings::{Best, PadLayout, PitchSource, Settings};
use crate::sim::{Aircraft, Mission};
use crate::world::Camera;

pub const INK: Col = rgba(0x05, 0x08, 0x07, 1.0);
pub const PANEL: Col = rgba(0x0a, 0x11, 0x0e, 0.94);
pub const LINE: Col = rgba(0x31, 0x47, 0x3a, 1.0);
pub const LINE_HOT: Col = rgba(0x6e, 0x9b, 0x6d, 1.0);
pub const GREEN: Col = rgba(0x9d, 0xff, 0xa7, 1.0);
pub const ACID: Col = rgba(0xd8, 0xff, 0x86, 1.0);
pub const AMBER: Col = rgba(0xff, 0xc0, 0x5b, 1.0);
pub const RED: Col = rgba(0xff, 0x6d, 0x5f, 1.0);
pub const CREAM: Col = rgba(0xec, 0xe8, 0xcd, 1.0);
pub const DIM: Col = rgba(0x7b, 0x95, 0x7e, 1.0);
const HUD: Col = rgba(0xa9, 0xff, 0xb2, 1.0);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Start,
    Resume,
    Restart,
    Menu,
    Mute,
    Controls,
    Quit,
    SensDown,
    SensUp,
    FovDown,
    FovUp,
    Pitch,
    Invert,
    PadLayout,
    Fullscreen,
    Reset,
    /// Choose the mission on the start screen.
    Pick(MissionKind),
    /// Loadout screen: next store for a pylon pair (0 outboard, 1 inboard),
    /// next fuel load, and take off.
    Station(u8),
    Fuel,
    Launch,
    /// Next airframe for the player, and for the bandits.
    Jet,
    Aggressor,
    /// Mission clocks on or off.
    Clock,
    /// Wait for a key for an action (index into keymap::BINDS) and slot.
    Rebind(u8, u8),
    ResetKeys,
}

#[derive(Clone, Copy)]
pub struct Button {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub action: Action,
}

impl Button {
    pub fn hit(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }
}

pub struct Ui<'a> {
    pub cv: &'a mut Canvas,
    pub w: f32,
    pub h: f32,
    pub s: f32,
    pub buttons: Vec<Button>,
    pub selected: usize,
    pub mouse: (f32, f32),
    /// A controller is connected: control lists show its bindings.
    pub pad: bool,
    /// Which controller layout those lists show.
    pub layout: PadLayout,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Win,
    Crash,
    Timeout,
    /// Destroyed in the air by a missile or gunfire; the pilot ejects.
    ShotDown,
}

#[derive(Clone, Copy, Debug)]
pub struct RunResult {
    pub kind: Outcome,
    pub time: f64,
    pub gates: usize,
    pub score: f64,
    pub impact: f64,
    pub mission: MissionKind,
    /// Bandits killed or primary targets destroyed, of the total.
    pub done: usize,
    pub total: usize,
}

/// What the flight overlays show for a combat mission instead of the gate
/// count: the objective tally, the score and the steerpoint cue name.
pub struct Objective {
    pub label: String,
    pub score: f64,
    pub cue: &'static str,
}

pub fn format_time(sec: f64) -> String {
    let s = sec.max(0.0).round() as i64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

pub fn ui_scale(w: f32, h: f32) -> f32 {
    (w / 640.0).min(h / 360.0).floor().clamp(1.0, 4.0)
}

impl<'a> Ui<'a> {
    fn button(&mut self, label: &str, hint: &str, x: f32, y: f32, primary: bool, action: Action) -> f32 {
        let s = self.s;
        let w = Canvas::text_width(label, s) + if hint.is_empty() { 0.0 } else { Canvas::text_width(hint, s.max(2.0) - 1.0) + 8.0 * s } + 16.0 * s;
        let h = 15.0 * s;
        let i = self.buttons.len();
        let hot = self.selected == i || Button { x, y, w, h, action }.hit(self.mouse.0, self.mouse.1);
        let (bg, fg, border) = if primary {
            (if hot { CREAM } else { ACID }, INK, ACID)
        } else {
            (if hot { rgba(0x1a, 0x2a, 0x20, 0.95) } else { rgba(0x0a, 0x11, 0x0e, 0.6) }, if hot { ACID } else { GREEN }, if hot { ACID } else { LINE_HOT })
        };
        self.cv.rect(x, y, w, h, bg);
        self.cv.stroke_rect(x, y, w, h, 1.0, border);
        self.cv.text(label, x + 8.0 * s, y + h / 2.0, s, fg, Align::Left);
        if !hint.is_empty() {
            let hs = s.max(2.0) - 1.0;
            self.cv.text(hint, x + w - 8.0 * s, y + h / 2.0, hs, with_alpha(fg, 0.7), Align::Right);
        }
        self.buttons.push(Button { x, y, w, h, action });
        w
    }

    /// A fixed-width key cap in the small font, for the bindings table.
    fn key_cap(&mut self, label: &str, x: f32, y: f32, w: f32, hot: bool, action: Action) {
        let sm = self.s.max(2.0) - 1.0;
        let h = 11.0 * sm;
        let i = self.buttons.len();
        let focus = hot || i == self.selected || Button { x, y, w, h, action }.hit(self.mouse.0, self.mouse.1);
        let (bg, fg, border) = if focus { (rgba(0x1a, 0x2a, 0x20, 0.95), ACID, ACID) } else { (rgba(0x0a, 0x11, 0x0e, 0.6), GREEN, LINE_HOT) };
        self.cv.rect(x, y, w, h, bg);
        self.cv.stroke_rect(x, y, w, h, 1.0, border);
        self.cv.text(label, x + w * 0.5, y + h * 0.5, sm, fg, Align::Center);
        self.buttons.push(Button { x, y, w, h, action });
    }

    fn frame(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.cv.rect(x, y, w, h, PANEL);
        self.cv.stroke_rect(x, y, w, h, 1.0, LINE_HOT);
        let c = 8.0 * self.s;
        for (cx, cy, dx, dy) in [(x, y, 1.0, 1.0), (x + w, y, -1.0, 1.0), (x, y + h, 1.0, -1.0), (x + w, y + h, -1.0, -1.0)] {
            self.cv.rect(cx.min(cx + dx * c), cy.min(cy + dy * 2.0), c, 2.0, ACID);
            self.cv.rect(cx.min(cx + dx * 2.0), cy.min(cy + dy * c), 2.0, c, ACID);
        }
    }

    // ---------------------------------------------------------- screens
    pub fn start_screen(&mut self, muted: bool, best: &Best, time: f64, pitch: PitchSource, mission: MissionKind, cfg: &Config, clock: bool) {
        let s = self.s;
        let (pw, ph) = ((self.w - 24.0 * s).min(600.0 * s), (self.h - 20.0 * s).min(250.0 * s));
        let (x, y) = ((self.w - pw) / 2.0, (self.h - ph) / 2.0);
        self.frame(x, y, pw, ph);
        let m = 12.0 * s;
        self.cv.text("// TACTICAL AIR COMMAND, 0715 ZULU", x + m, y + m, s.max(2.0) - 1.0, DIM, Align::Left);
        let lamp = (time * 1.6).fract() < 0.7;
        let lx = x + pw - m;
        self.cv.text("FLCS ONLINE", lx, y + m, s.max(2.0) - 1.0, DIM, Align::Right);
        let tw = Canvas::text_width("FLCS ONLINE", s.max(2.0) - 1.0);
        for (i, c) in [GREEN, with_alpha(GREEN, 0.35), RED].iter().enumerate() {
            let c = if i == 0 && !lamp { with_alpha(*c, 0.4) } else { *c };
            self.cv.disc(lx - tw - 8.0 * s - i as f32 * 7.0 * s, y + m, 2.2 * s, c);
        }
        self.cv.line(x + m, y + m + 8.0 * s, x + pw - m, y + m + 8.0 * s, 1.0, LINE);

        let left_w = pw * 0.56;
        let mut yy = y + 34.0 * s;
        self.cv.text("F-16 VIPER, MODERN COCKPIT", x + m, yy, s, AMBER, Align::Left);
        yy += 26.0 * s;
        let ts = s * 4.0;
        self.cv.text("VIPER", x + m, yy, ts, GREEN, Align::Left);
        let vw = Canvas::text_width("VIPER", ts);
        self.cv.text("/", x + m + vw + 4.0 * s, yy, ts, AMBER, Align::Left);
        self.cv.text("RUN", x + m + vw + 4.0 * s + Canvas::text_width("/", ts) + 4.0 * s, yy, ts, CREAM, Align::Left);
        yy += 22.0 * s;
        self.cv.rect(x + m, yy, left_w - m, 1.0, LINE_HOT);
        self.cv.text("COLD RIVER GORGE, SECTOR 07", x + m, yy + 7.0 * s, s.max(2.0) - 1.0, DIM, Align::Left);
        yy += 18.0 * s;
        let (copy, tag) = match mission {
            MissionKind::Gates => ("You are already airborne, twenty clicks from the border, with ten navigation gates strung down a black river gorge. Thread the valley. Stay under the ridgeline. Make the coast before the clock runs out.", "THE JET IS LOADED. THE WEATHER IS PERFECT. THE MOUNTAIN IS NOT."),
            MissionKind::Dogfight => ("Two aggressor jets are coming up the gorge high and fast, and a third is somewhere off the flank. Lock them up, take the shot, and win the turning fight that follows.", "SAME JET. SAME LIMITS. ONLY ONE OF YOU GOES HOME."),
            MissionKind::Airfield => ("Your jet sits on runway A of the valley airbase with the engine at idle. Take off, raise the gear, fly out to the steerpoint past the far end and come back to land: gear down below 300 knots, about 13 degrees of alpha on final, and stop on the runway.", match (self.pad, self.layout) {
                    (false, _) => "GEAR, FLAPS AND WHEEL BRAKES ARE ON G, L AND B.",
                    (true, PadLayout::Combat) => "GEAR AND FLAPS ARE ON THE D-PAD, WHEEL BRAKES ON LT.",
                    (true, PadLayout::Classic) => "GEAR IS ON THE RS CLICK, WHEEL BRAKES ON LT.",
                },
            ),
            MissionKind::Strike => ("An SA-6 battery guards the Cold River bridge. Go in low under its radar, put a bomb on the span and on the SAM, and strafe what moves. The guns along the river shoot back.", "FOUR MK-82S. PICKLE ON THE PIPPER."),
            MissionKind::Free => ("No clock and no score to beat. The gate rings are strung down the gorge, the bridge, the convoy and the SA-6 are on the river, and a pair of bandits is coming. Nobody shoots back, and whatever you shoot down or blow up comes back half a minute later.", "STOP ON THE RUNWAY TO REARM, REFUEL AND REPAIR."),
        };
        yy += self.cv.paragraph(copy, x + m, yy, left_w - m * 1.5, s.max(2.0) - 1.0, 1.7, rgba(0xc7, 0xd6, 0xc2, 1.0));
        yy += 2.0 * s;
        yy += self.cv.paragraph(tag, x + m, yy, left_w - m * 1.5, s.max(2.0) - 1.0, 1.7, CREAM);
        yy += 8.0 * s;
        let bx = x + m;
        // The mission row sits above ENGAGE but comes after it in the
        // keyboard order, so Enter still starts the flight.
        let picks_y = yy;
        yy += 19.0 * s;
        let jets_y = yy;
        yy += 19.0 * s;
        let w1 = self.button("ENGAGE", "ENTER", bx, yy, true, Action::Start);
        let w2 = self.button(if muted { "AUDIO OFF" } else { "AUDIO ON" }, "M", bx + w1 + 6.0 * s, yy, false, Action::Mute);
        let _ = w2;
        yy += 19.0 * s;
        let w3 = self.button("CONTROLS / CONFIG", "K", bx, yy, false, Action::Controls);
        self.button("QUIT", "", bx + w3 + 6.0 * s, yy, false, Action::Quit);
        let mut mx = bx;
        for k in MissionKind::ALL {
            // The chosen mission is the lit button; free flight's name is cut
            // short so the row stays clear of the briefing card.
            let label = if k == MissionKind::Free { "FREE FLT" } else { k.name() };
            mx += self.button(label, "", mx, picks_y, k == mission, Action::Pick(k)) + 4.0 * s;
        }
        let mut jx = bx;
        jx += self.button(&format!("JET: {}", cfg.jet.name()), "", jx, jets_y, false, Action::Jet) + 4.0 * s;
        if matches!(mission, MissionKind::Dogfight | MissionKind::Free) {
            jx += self.button(&format!("BANDITS: {}", cfg.aggressor.name()), "", jx, jets_y, false, Action::Aggressor) + 4.0 * s;
        }
        // Free flight never has a clock.
        if mission != MissionKind::Free {
            self.button(if clock { "CLOCK: ON" } else { "CLOCK: OFF" }, "", jx, jets_y, false, Action::Clock);
        }

        // Briefing card.
        let (bxx, byy, bw) = (x + left_w + 6.0 * s, y + 30.0 * s, pw - left_w - m - 6.0 * s);
        let bh = ph - 52.0 * s;
        self.cv.rect(bxx, byy, bw, bh, rgba(0x05, 0x0b, 0x08, 0.8));
        self.cv.stroke_rect(bxx, byy, bw, bh, 1.0, LINE);
        let sm = s.max(2.0) - 1.0;
        self.cv.text(&format!("MISSION DATA // {}", mission.name()), bxx + 6.0 * s, byy + 8.0 * s, sm, AMBER, Align::Left);
        let airframe = match cfg.jet {
            crate::sim::Type::F16 => ("AIRFRAME", "F-16C BLOCK 50", "F110-GE-129 / FLCS NORM"),
            crate::sim::Type::Mig29 => ("AIRFRAME", "MIG-29A 9.12", "2 X RD-33 / FBW TRAINER FLCS"),
        };
        let rows = match mission {
            MissionKind::Gates => [airframe, ("ROUTE", "10 GATES", "RIVER GORGE, COAST EGRESS"), ("CLOCK", "02:30", "+ 8 SEC PER GATE"), ("HAZARD", "TERRAIN", "STALL, G-LIMIT")],
            MissionKind::Dogfight => [airframe, ("THREAT", "3 BANDITS", if cfg.aggressor == crate::sim::Type::Mig29 { "MIG-29A FULCRUM, AIM-9, GUN" } else { "AGGRESSOR F-16, AIM-9, GUN" }), ("STORES", "2 AIM-9M 2 AIM-120C", "M61A1, 511 ROUNDS"), ("CLOCK", "06:00", "SPLASH ALL THREE")],
            MissionKind::Airfield => [airframe, ("ROUTE", "RWY A, STPT 1, RWY A", "9 KM PAST THE FAR END, 3,000 FT"), ("CLOCK", "10:00", "FULL-STOP LANDING"), ("SCORING", "TOUCHDOWN", "SINK RATE AND AIM POINT")],
            MissionKind::Strike => [airframe, ("TARGETS", "BRIDGE, SA-6 SITE", "CONVOY, DEPOT, ZSU-23-4"), ("STORES", "4 MK-82 2 AIM-9M", "M61A1, 511 ROUNDS"), ("CLOCK", "07:00", "DROP BRIDGE, KILL THE SAM")],
            MissionKind::Free => [airframe, ("ROUTE", "10 GATES", "RIVER GORGE, ON A LOOP"), ("TARGETS", "2 BANDITS, SA-6", "HOLD FIRE, BACK IN 30 SEC"), ("CLOCK", "OFF", "STOP ON RWY A TO REARM")],
        };
        let rows = rows.map(|r| if !clock && r.0 == "CLOCK" { ("CLOCK", "OFF", "NO TIME LIMIT") } else { r });
        let mut ry = byy + 20.0 * s;
        for (k, v1, v2) in rows {
            self.cv.text(k, bxx + 6.0 * s, ry, sm, DIM, Align::Left);
            self.cv.text(v1, bxx + 6.0 * s + 58.0 * sm, ry, sm, CREAM, Align::Left);
            self.cv.text(v2, bxx + 6.0 * s + 58.0 * sm, ry + 9.0 * sm, sm, with_alpha(GREEN, 0.75), Align::Left);
            ry += 22.0 * sm;
            self.cv.rect(bxx + 6.0 * s, ry - 5.0 * sm, bw - 12.0 * s, 1.0, with_alpha(LINE, 0.6));
        }
        ry += 2.0 * s;
        let k1 = match pitch {
            PitchSource::Mouse => ("MOUSE", "PITCH / RUDDER"),
            PitchSource::Keys => ("W S", "PITCH"),
        };
        // Missions with something to shoot list the weapons; the others list
        // what flying the route and landing need.
        let armed = matches!(mission, MissionKind::Dogfight | MissionKind::Strike | MissionKind::Free);
        let keys = match (armed, self.pad, self.layout) {
            (true, true, PadLayout::Combat) => [("LS", "PITCH / ROLL"), ("RS", "LOOK"), ("RT", "FIRE"), ("RB", "LOCK TARGET"), ("LB", "THROTTLE UP"), ("LT", "THROTTLE DOWN"), ("A", "NEXT WEAPON"), ("LS CLICK", "CHAFF / FLARE"), ("X + LS", "RUDDER"), ("B", "SPEEDBRAKE"), ("Y", "CAMERA"), ("START", "PAUSE")],
            (true, true, PadLayout::Classic) => [("LS", "PITCH / ROLL"), ("RS", "LOOK"), ("A", "FIRE"), ("D-PAD DOWN", "LOCK TARGET"), ("RT", "THROTTLE UP"), ("LT", "THROTTLE DOWN"), ("D-PAD UP", "NEXT WEAPON"), ("LS CLICK", "CHAFF / FLARE"), ("LB RB", "RUDDER"), ("B", "SPEEDBRAKE"), ("Y", "CAMERA"), ("START", "PAUSE")],
            (false, true, PadLayout::Combat) => [("LS", "PITCH / ROLL"), ("RS", "LOOK"), ("LB", "THROTTLE UP"), ("LT", "THROTTLE DOWN"), ("X + LS", "RUDDER"), ("B", "SPEEDBRAKE"), ("D-PAD DOWN", "LANDING GEAR"), ("D-PAD UP", "FLAPS"), ("LT GROUND", "WHEEL BRAKES"), ("Y", "CAMERA"), ("START", "PAUSE"), ("BACK", "CONTROLS")],
            (false, true, PadLayout::Classic) => [("LS", "PITCH / ROLL"), ("RS", "LOOK"), ("RT", "THROTTLE UP"), ("LT", "THROTTLE DOWN"), ("LB RB", "RUDDER"), ("B", "SPEEDBRAKE"), ("RS CLICK", "LANDING GEAR"), ("X", "CENTER VIEW"), ("LT GROUND", "WHEEL BRAKES"), ("Y", "CAMERA"), ("START", "PAUSE"), ("BACK", "CONTROLS")],
            (true, false, _) => [k1, ("A D", "ROLL"), ("F L-CLICK", "FIRE"), ("T", "LOCK TARGET"), ("SHIFT", "THROTTLE UP"), ("CTRL", "THROTTLE DOWN"), ("X", "NEXT WEAPON"), ("Z", "CHAFF / FLARE"), ("Q E", "RUDDER"), ("H", "SPEEDBRAKE"), ("V", "CAMERA"), ("P ESC", "PAUSE, MENU")],
            (false, false, _) => [k1, ("A D", "ROLL"), ("SHIFT", "THROTTLE UP"), ("CTRL", "THROTTLE DOWN"), ("Q E", "RUDDER"), ("H", "SPEEDBRAKE"), ("G", "LANDING GEAR"), ("L", "FLAPS"), ("B", "WHEEL BRAKES"), ("V", "CAMERA"), ("ALT+MOUSE", "LOOK"), ("P ESC", "PAUSE, MENU")],
        };
        self.cv.text(if self.pad { "XBOX CONTROLLER" } else { "FLIGHT CONTROLS" }, bxx + 6.0 * s, ry, sm, AMBER, Align::Left);
        ry += 10.0 * sm;
        for (i, (k, v)) in keys.iter().enumerate() {
            let col = i % 2;
            let row = i / 2;
            let kx = bxx + 6.0 * s + col as f32 * (bw - 12.0 * s) / 2.0;
            let ky = ry + row as f32 * 10.0 * sm;
            if ky > byy + bh - 6.0 * sm {
                break;
            }
            self.cv.text(k, kx, ky, sm, ACID, Align::Left);
            self.cv.text(v, kx + Canvas::text_width(k, sm) + 4.0 * sm, ky, sm, with_alpha(CREAM, 0.7), Align::Left);
        }
        // Footer.
        let fy = y + ph - 12.0 * s;
        self.cv.line(x + m, fy - 8.0 * s, x + pw - m, fy - 8.0 * s, 1.0, LINE);
        self.cv.text(&best_line(best), x + m, fy, sm, AMBER, Align::Left);
        self.cv.text("37°14′N 081°06′E  ALT 2,140 FT", x + pw - m, fy, sm, DIM, Align::Right);
    }

    fn status_panel(&mut self, kicker: &str, kicker_col: Col, title: &str, title_col: Col, sub: &str, stats: [(&str, String, Col); 4], actions: &[(&str, &str, Action)], small: &str) {
        let s = self.s;
        let sm = s.max(2.0) - 1.0;
        let (pw, ph) = ((self.w - 24.0 * s).min(380.0 * s), (self.h - 20.0 * s).min(186.0 * s));
        let (x, y) = ((self.w - pw) / 2.0, (self.h - ph) / 2.0);
        self.frame(x, y, pw, ph);
        let m = 14.0 * s;
        let mut yy = y + m;
        self.cv.text(kicker, x + m, yy, sm, kicker_col, Align::Left);
        yy += 20.0 * s;
        self.cv.text(title, x + m, yy, s * 3.0, title_col, Align::Left);
        yy += 18.0 * s;
        yy += self.cv.paragraph(sub, x + m, yy, pw - 2.0 * m, sm, 1.7, rgba(0xc7, 0xd6, 0xc2, 1.0));
        yy += 8.0 * s;
        let cw = (pw - 2.0 * m - 3.0 * 4.0 * s) / 4.0;
        for (i, (k, v, c)) in stats.iter().enumerate() {
            let cx = x + m + i as f32 * (cw + 4.0 * s);
            self.cv.rect(cx, yy, cw, 30.0 * s, rgba(0x05, 0x0b, 0x08, 0.85));
            self.cv.stroke_rect(cx, yy, cw, 30.0 * s, 1.0, LINE);
            self.cv.text(k, cx + 5.0 * s, yy + 8.0 * s, sm, DIM, Align::Left);
            self.cv.text(v, cx + 5.0 * s, yy + 20.0 * s, s, *c, Align::Left);
        }
        yy += 40.0 * s;
        let mut bx = x + m;
        for (i, (label, hint, act)) in actions.iter().enumerate() {
            let w = Canvas::text_width(label, s) + if hint.is_empty() { 0.0 } else { Canvas::text_width(hint, sm) + 8.0 * s } + 16.0 * s;
            if bx + w > x + pw - m {
                bx = x + m;
                yy += 19.0 * s;
            }
            let used = self.button(label, hint, bx, yy, i == 0, *act);
            bx += used + 6.0 * s;
        }
        self.cv.text(small, x + m, y + ph - 12.0 * s, sm, DIM, Align::Left);
    }

    pub fn pause_screen(&mut self, a: &Aircraft, m: &Mission, muted: bool, obj: Option<&Objective>) {
        let cleared = match obj {
            Some(o) => ("OBJECTIVE", o.label.clone(), GREEN),
            None => ("CLEARED", format!("{} / {}", pad(m.passed as f64, 2), pad(m.gates.len() as f64, 2)), GREEN),
        };
        self.status_panel(
            "// FLIGHT SUSPENDED, FLCS HOLD",
            DIM,
            "PAUSED",
            GREEN,
            "Your flight is frozen on the clock. Take a breath, check the route, and re-engage when ready.",
            [
                cleared,
                if m.timed { ("TIME LEFT", format_time(m.time_left), AMBER) } else { ("ELAPSED", format_time(m.elapsed), AMBER) },
                ("AIRSPEED", format!("{} KT", pad(a.airspeed_kt, 3)), CREAM),
                ("ALTITUDE", format!("{} FT", pad(AirData::of(a).baro_ft, 5)), CREAM),
            ],
            &[("RESUME", "P", Action::Resume), ("RESTART", "R", Action::Restart), ("MENU", "", Action::Menu), ("CONFIG", "K", Action::Controls)],
            &format!("ESC RESUME, R RESTART, M AUDIO {}", if muted { "OFF" } else { "ON" }),
        );
    }

    pub fn result_screen(&mut self, r: &RunResult, best: &Best) {
        if r.mission != MissionKind::Gates {
            self.combat_result(r);
            return;
        }
        let best_str = if best.score > 0.0 { pad(best.score, 5) } else { "-----".to_string() };
        match r.kind {
            Outcome::Win => {
                let rank = if r.time < 95.0 { "ACE" } else if r.time < 125.0 { "VIPER" } else if r.time < 155.0 { "QUALIFIED" } else { "CADET" };
                self.status_panel(
                    "// ROUTE COMPLETE, COLD RIVER GORGE",
                    DIM,
                    "COURSE COMPLETE",
                    ACID,
                    &format!("All ten gates cleared. You brought the jet and the terrain back in one piece. The evaluator has logged the run as {rank}."),
                    [("RUN TIME", format_time(r.time), GREEN), ("GATES", format!("{} / 10", r.gates), CREAM), ("SCORE", pad(r.score, 5), AMBER), ("RATING", rank.to_string(), CREAM)],
                    &[("FLY AGAIN", "R", Action::Restart), ("MISSION BOARD", "", Action::Menu)],
                    &format!("{}, R RESTART, ESC MENU", best_line(best)),
                );
            }
            Outcome::Crash | Outcome::ShotDown => self.status_panel(
                "// FLIGHT TERMINATED, AIRFRAME LOST",
                RED,
                "TERRAIN WINS",
                RED,
                &format!("The gorge is unforgiving. The flight recorder shows {} gates cleared before impact.", pad(r.gates as f64, 2)),
                [("GATE", format!("{} / 10", pad(r.gates as f64, 2)), RED), ("ELAPSED", format_time(r.time), CREAM), ("IMPACT", format!("{} KT", pad(r.impact * 1.94384, 3)), RED), ("BEST", best_str, CREAM)],
                &[("REARM & RETRY", "R", Action::Restart), ("MISSION BOARD", "", Action::Menu)],
                "TIP: EASE OFF THE PULL AT HIGH SPEED. LET THE GORGE OPEN BEFORE YOU ROLL IN.",
            ),
            Outcome::Timeout => self.status_panel(
                "// CLOCK EXPIRED, RUN INVALID",
                AMBER,
                "RUN EXPIRED",
                AMBER,
                &format!("The coast is still out there. You cleared {} gates before the clock ran dry.", pad(r.gates as f64, 2)),
                [("GATE", format!("{} / 10", pad(r.gates as f64, 2)), AMBER), ("ELAPSED", format_time(r.time), CREAM), ("SCORE", pad(r.score, 5), CREAM), ("BEST", best_str, CREAM)],
                &[("TRY AGAIN", "R", Action::Restart), ("MISSION BOARD", "", Action::Menu)],
                "THE CLOCK IS A HARD CEILING. GATES ADD 8 SECONDS EACH.",
            ),
        }
    }

    /// Loadout before a DOGFIGHT or STRIKE sortie: the pylon pairs, the fuel,
    /// and what that does to the weight, drag and thrust-to-weight.
    pub fn loadout_screen(&mut self, mission: MissionKind, cfg: &Config) {
        let s = self.s;
        let sm = s.max(2.0) - 1.0;
        let (pw, ph) = ((self.w - 24.0 * s).min(420.0 * s), (self.h - 20.0 * s).min(250.0 * s));
        let (x, y) = ((self.w - pw) / 2.0, (self.h - ph) / 2.0);
        self.frame(x, y, pw, ph);
        let m = 14.0 * s;
        let mut yy = y + m;
        self.cv.text(&format!("// ARMAMENT AND FUEL, {}", mission.name()), x + m, yy, sm, DIM, Align::Left);
        yy += 20.0 * s;
        self.cv.text("LOADOUT", x + m, yy, s * 3.0, GREEN, Align::Left);
        yy += 18.0 * s;
        let name = |st: Option<Store>| match st {
            None => "EMPTY",
            Some(Store::Aim9) => "2 X AIM-9M  IR",
            Some(Store::Aim120) => "2 X AIM-120C  RADAR",
            Some(Store::Mk82) => "2 X MK-82  500 LB",
        };
        let rows: [(&str, String, Option<Action>); 6] = [
            ("STA 1 / 9  WINGTIP", "2 X AIM-9M  IR".to_string(), None),
            ("STA 3 / 7  OUTBOARD", name(cfg.outboard).to_string(), Some(Action::Station(0))),
            ("STA 4 / 6  INBOARD", name(cfg.inboard).to_string(), Some(Action::Station(1))),
            ("INTERNAL FUEL", format!("{} LB  {}%", (cfg.jet.fuel_max() * cfg.fuel * 2.20462).round() as i64, (cfg.fuel * 100.0).round() as i64), Some(Action::Fuel)),
            ("M61A1 GUN", "511 ROUNDS PGU-28".to_string(), None),
            ("ALE-47", "30 CHAFF  30 FLARES".to_string(), None),
        ];
        // LAUNCH comes first in the keyboard order, so Enter takes off.
        let launch_y = yy + 6.0 * 17.0 * s + 14.0 * s;
        let w1 = self.button("LAUNCH", "ENTER", x + m, launch_y, true, Action::Launch);
        self.button("MISSION BOARD", "ESC", x + m + w1 + 6.0 * s, launch_y, false, Action::Menu);
        let col2 = x + m + 112.0 * sm;
        for (label, value, act) in rows {
            self.cv.text(label, x + m, yy + 7.0 * s, sm, DIM, Align::Left);
            match act {
                Some(a) => {
                    self.button(&value, "", col2, yy, false, a);
                }
                None => self.cv.text(&value, col2 + 8.0 * s, yy + 7.0 * s, sm, CREAM, Align::Left),
            }
            yy += 17.0 * s;
        }
        let (mass, drag) = cfg.weight();
        let lb = mass * 2.20462;
        // Full afterburner, 29,000 lbf, against the take-off weight.
        let tw = 29000.0 / lb;
        let stats = format!("GROSS {} LB   DRAG +{} COUNTS   T/W {:.2}", (lb / 10.0).round() as i64 * 10, (drag * 10000.0).round() as i64, tw);
        self.cv.text(&stats, x + m, yy + 4.0 * s, sm, AMBER, Align::Left);
        self.cv.text("CLICK A STORE TO CHANGE IT. HEAVIER AND DRAGGIER MEANS SLOWER AND LESS G.", x + m, y + ph - 12.0 * s, sm, DIM, Align::Left);
    }

    /// Result of a DOGFIGHT or STRIKE sortie.
    fn combat_result(&mut self, r: &RunResult) {
        if r.mission == MissionKind::Airfield && r.kind == Outcome::Win {
            let again: &[(&str, &str, Action)] = &[("FLY AGAIN", "R", Action::Restart), ("MISSION BOARD", "", Action::Menu)];
            self.status_panel(
                "// FULL STOP, RUNWAY A",
                DIM,
                "ON THE GROUND",
                ACID,
                "Wheels up, out to the steerpoint and back down on the same runway. The tower has logged your landing.",
                [("TIME", format_time(r.time), GREEN), ("LEGS", format!("{} / {}", r.done, r.total), CREAM), ("SCORE", pad(r.score, 5), AMBER), ("RATING", if r.score > 4200.0 { "ACE" } else if r.score > 3500.0 { "VIPER" } else { "QUALIFIED" }.to_string(), CREAM)],
                again,
                "R RESTART, ESC MENU",
            );
            return;
        }
        if r.mission == MissionKind::Free {
            // Nothing shoots back in free flight: only a crash ends it.
            self.status_panel(
                "// FREE FLIGHT, AIRFRAME LOST",
                RED,
                "TERRAIN WINS",
                RED,
                &format!("The gorge is unforgiving. You flew {} and took down {} bandits and {} ground targets.", format_time(r.time), r.done, r.total),
                [("FLOWN", format_time(r.time), CREAM), ("KILLS", r.done.to_string(), GREEN), ("TARGETS", r.total.to_string(), GREEN), ("SCORE", pad(r.score, 5), AMBER)],
                &[("FLY AGAIN", "R", Action::Restart), ("MISSION BOARD", "", Action::Menu)],
                "R RESTART, ESC MENU",
            );
            return;
        }
        let fight = r.mission == MissionKind::Dogfight;
        let (what, unit) = if fight { ("KILLS", "bandits") } else { ("TARGETS", "primary targets") };
        let tally = format!("{} / {}", r.done, r.total);
        let again: &[(&str, &str, Action)] = &[("FLY AGAIN", "R", Action::Restart), ("MISSION BOARD", "", Action::Menu)];
        match r.kind {
            Outcome::Win => {
                let rank = match (fight, r.time) {
                    (true, t) if t < 100.0 => "ACE",
                    (true, t) if t < 170.0 => "VIPER",
                    (false, t) if t < 200.0 => "ACE",
                    (false, t) if t < 300.0 => "VIPER",
                    _ => "QUALIFIED",
                };
                let (kicker, title, text) = if fight {
                    ("// SPLASH ALL BANDITS, COLD RIVER", "SKY CLEARED", format!("All three aggressors are down and you are still flying. The evaluator has logged the fight as {rank}."))
                } else {
                    ("// TARGETS DESTROYED, COLD RIVER", "STRIKE COMPLETE", format!("The bridge is in the river and the SA-6 battery is burning. The evaluator has logged the strike as {rank}."))
                };
                self.status_panel(kicker, DIM, title, ACID, &text, [("TIME", format_time(r.time), GREEN), (what, tally, CREAM), ("SCORE", pad(r.score, 5), AMBER), ("RATING", rank.to_string(), CREAM)], again, "R RESTART, ESC MENU");
            }
            Outcome::ShotDown => self.status_panel(
                "// AIRFRAME LOST, PILOT EJECTED",
                RED,
                "SHOT DOWN",
                RED,
                &format!("The jet came apart around you and the seat did its job. You took {} {unit} with you.", r.done),
                [(what, tally, RED), ("ELAPSED", format_time(r.time), CREAM), ("SCORE", pad(r.score, 5), CREAM), ("SPEED", format!("{} KT", pad(r.impact * 1.94384, 3)), CREAM)],
                again,
                if fight { "TIP: WATCH THE RWR. BREAK INTO A MISSILE, NOT AWAY FROM IT." } else { "TIP: STAY LOW IN THE GORGE. THE SA-6 CANNOT SEE YOU BELOW THE RIDGES." },
            ),
            Outcome::Crash => self.status_panel(
                "// FLIGHT TERMINATED, AIRFRAME LOST",
                RED,
                "TERRAIN WINS",
                RED,
                &format!("The gorge is unforgiving. The flight recorder shows {} {unit} down before impact.", r.done),
                [(what, tally, RED), ("ELAPSED", format_time(r.time), CREAM), ("IMPACT", format!("{} KT", pad(r.impact * 1.94384, 3)), RED), ("SCORE", pad(r.score, 5), CREAM)],
                again,
                "TIP: A DIVE ON A TARGET NEEDS ROOM TO PULL OUT. RELEASE HIGH ENOUGH.",
            ),
            Outcome::Timeout => self.status_panel(
                "// CLOCK EXPIRED, BINGO FUEL",
                AMBER,
                "TIME EXPIRED",
                AMBER,
                &format!("The tanker will not wait. You return with {} of {} {unit} done.", r.done, r.total),
                [(what, tally, AMBER), ("ELAPSED", format_time(r.time), CREAM), ("SCORE", pad(r.score, 5), CREAM), ("MISSION", r.mission.name().to_string(), CREAM)],
                again,
                "X CYCLES THE WEAPON, T LOCKS THE TARGET NEAREST THE NOSE.",
            ),
        }
    }

    /// Left drawer with the control reference and live settings.
    pub fn controls_drawer(&mut self, st: &Settings, paused_flight: bool, waiting: Option<(usize, usize)>) {
        let s = self.s;
        let sm = s.max(2.0) - 1.0;
        let w = (200.0 * s).min(self.w * 0.94);
        self.cv.rect(0.0, 0.0, w, self.h, rgba(0x09, 0x10, 0x12, 0.97));
        self.cv.rect(w - 1.0, 0.0, 1.0, self.h, rgba(0x78, 0x8e, 0x7b, 1.0));
        let m = 10.0 * s;
        let mut y = m + 4.0 * s;
        self.cv.text("CONTROLS / CONFIG", m, y, s, CREAM, Align::Left);
        self.button("CLOSE", "K", w - m - 50.0 * s, y - 7.5 * s, false, Action::Controls);
        y += 16.0 * s;
        let heading = match (paused_flight, self.pad) {
            (true, _) => "FLIGHT PAUSED",
            (false, true) => "XBOX CONTROLLER",
            (false, false) => "FLIGHT CONTROLS",
        };
        self.cv.text(heading, m, y, sm, rgba(0xbe, 0xd5, 0x8f, 1.0), Align::Left);
        y += 12.0 * s;
        let pitch_rows: &[(&str, &str)] = match (self.pad, st.pitch, st.invert_pitch) {
            (true, _, false) => &[("LS", "Pitch and roll")],
            (true, _, true) => &[("LS", "Pitch (inverted) and roll")],
            (false, PitchSource::Mouse, false) => &[("MOUSE Y", "Pitch")],
            (false, PitchSource::Mouse, true) => &[("MOUSE Y", "Pitch (inverted)")],
            (false, PitchSource::Keys, false) => &[("S / DOWN", "Nose up"), ("W / UP", "Nose down")],
            (false, PitchSource::Keys, true) => &[("W / UP", "Nose up"), ("S / DOWN", "Nose down")],
        };
        let rest: &[(&str, &str)] = match (self.pad, st.pad_layout) {
            (true, PadLayout::Combat) => &[
                ("RS", "Look around"),
                ("RT", "Fire, hold for gun"),
                ("RB", "Lock target, again for next"),
                ("RB HOLD", "Drop lock"),
                ("LB", "Throttle up, afterburner at the stop"),
                ("LT", "Throttle down"),
                ("LT GROUND", "Wheel brakes"),
                ("X + LS", "Rudder"),
                ("LS CLICK", "Chaff and flare"),
                ("A", "Next weapon"),
                ("B", "Speedbrake"),
                ("Y", "Cockpit / chase camera"),
                ("D-PAD L R", "Left / right MFD page"),
                ("D-PAD DOWN", "Landing gear"),
                ("D-PAD UP", "Flaps"),
                ("START", "Freeze flight"),
                ("B FROZEN", "Pause menu"),
                ("BACK", "Controls and settings"),
            ],
            (true, PadLayout::Classic) => &[
                ("RS", "Look around"),
                ("A", "Fire, hold for gun"),
                ("D-PAD DOWN", "Lock target, again for next"),
                ("RT", "Throttle up, afterburner at the stop"),
                ("LT", "Throttle down"),
                ("LT GROUND", "Wheel brakes"),
                ("LB RB", "Rudder left / right"),
                ("LS CLICK", "Chaff and flare"),
                ("D-PAD UP", "Next weapon"),
                ("B", "Speedbrake"),
                ("X", "Center view"),
                ("RS CLICK", "Landing gear"),
                ("Y", "Cockpit / chase camera"),
                ("D-PAD L R", "Left / right MFD page"),
                ("START", "Freeze flight"),
                ("B FROZEN", "Pause menu"),
                ("BACK", "Controls and settings"),
            ],
            (false, _) => &[("MOUSE X", "Rudder"), ("ALT+MOUSE", "Look around"), ("R-CLICK", "Center the mouse stick"), ("L-CLICK", "Fire"), ("P", "Freeze flight"), ("ESC", "Pause menu"), ("M", "Sound on / off"), ("F11", "Fullscreen"), ("KEYS", "Change them at right")],
        };
        for &(k, v) in pitch_rows.iter().chain(rest.iter()) {
            self.cv.text(k, m, y, sm, ACID, Align::Left);
            self.cv.text(v, m + 72.0 * sm, y, sm, rgba(0xb3, 0xc3, 0xb7, 1.0), Align::Left);
            y += 11.0 * sm;
            self.cv.rect(m, y - 5.5 * sm, w - 2.0 * m, 1.0, rgba(0x25, 0x35, 0x2e, 1.0));
        }
        y += 6.0 * s;
        let slider = |ui: &mut Ui, y: f32, label: &str, value: String, t: f64, down: Action, up: Action| {
            ui.cv.text(label, m, y, sm, CREAM, Align::Left);
            ui.cv.text(&value, w - m, y, sm, rgba(0xc9, 0xe9, 0xa0, 1.0), Align::Right);
            let by = y + 9.0 * s;
            ui.button("-", "", m, by, false, down);
            let bw = 15.0 * s;
            let (tx, tw) = (m + bw + 4.0 * s, w - 2.0 * m - 2.0 * bw - 8.0 * s);
            ui.cv.rect(tx, by + 7.0 * s, tw, 2.0, LINE_HOT);
            ui.cv.rect(tx + tw * t as f32 - 2.0 * s, by + 3.0 * s, 4.0 * s, 9.0 * s, ACID);
            ui.button("+", "", w - m - bw + 1.0 * s, by, false, up);
        };
        slider(self, y, "MOUSE SENSITIVITY", format!("{:.1}X", st.sensitivity), (st.sensitivity - 0.5) / 4.5, Action::SensDown, Action::SensUp);
        y += 30.0 * s;
        slider(self, y, "COCKPIT FIELD OF VIEW", format!("{}°", st.fov.round()), (st.fov - 45.0) / 27.0, Action::FovDown, Action::FovUp);
        y += 30.0 * s;
        self.button(if st.pitch == PitchSource::Mouse { "PITCH INPUT: MOUSE" } else { "PITCH INPUT: W / S" }, "", m, y, false, Action::Pitch);
        y += 18.0 * s;
        self.button(if st.invert_pitch { "[X] INVERT PITCH" } else { "[ ] INVERT PITCH" }, "", m, y, false, Action::Invert);
        y += 18.0 * s;
        self.button(if st.pad_layout == PadLayout::Combat { "PAD LAYOUT: COMBAT" } else { "PAD LAYOUT: CLASSIC" }, "", m, y, false, Action::PadLayout);
        y += 18.0 * s;
        self.button(if st.fullscreen { "[X] FULLSCREEN" } else { "[ ] FULLSCREEN" }, "F11", m, y, false, Action::Fullscreen);
        y += 18.0 * s;
        self.cv.paragraph("Only the selected input pitches the jet. Invert pitch flips the mouse, W / S and the controller.", m, y, w - 2.0 * m, sm, 1.6, rgba(0x91, 0xa1, 0x8f, 1.0));
        y += 16.0 * s;
        self.button("RESTORE DEFAULTS", "", m, y, false, Action::Reset);
        self.cv.paragraph(if self.pad { "Press K or BACK to close. Settings are saved for this user." } else { "Press K to close. Settings are saved for this user." }, m, self.h - 16.0 * s, w - 2.0 * m, sm, 1.6, rgba(0x91, 0xa1, 0x8f, 1.0));
        self.key_bindings(&st.keys, w, waiting);
    }

    /// Key bindings beside the drawer: every flight action with its key and
    /// its alternate. Clicking a key waits for the next key press.
    fn key_bindings(&mut self, keys: &Keymap, x0: f32, waiting: Option<(usize, usize)>) {
        let s = self.s;
        let sm = s.max(2.0) - 1.0;
        let m = 8.0 * s;
        let row = 13.0 * sm;
        let w = (240.0 * s).min(self.w - x0 - 4.0);
        if w < 120.0 * s {
            return;
        }
        let h = (BINDS.len() as f32 + 3.6) * row + 2.0 * m;
        let (x, y0) = (x0 + 4.0, (12.0 * s).min((self.h - h).max(0.0)));
        self.cv.rect(x, y0, w, h, rgba(0x09, 0x10, 0x12, 0.95));
        self.cv.stroke_rect(x, y0, w, h, 1.0, LINE);
        let mut y = y0 + m + 4.0 * s;
        self.cv.text("KEY BINDINGS", x + m, y, s, CREAM, Align::Left);
        y += 12.0 * s;
        let c1 = x + m + 108.0 * sm;
        let cw = (w - 2.0 * m - 108.0 * sm - 4.0 * sm) * 0.5;
        for (i, &b) in BINDS.iter().enumerate() {
            self.cv.text(b.label(), x + m, y + 5.5 * sm, sm, rgba(0xb3, 0xc3, 0xb7, 1.0), Align::Left);
            for slot in 0..2 {
                let label = if waiting == Some((i, slot)) {
                    "PRESS KEY".to_string()
                } else {
                    keys.get(b)[slot].map(key_name).unwrap_or_else(|| "-".to_string())
                };
                self.key_cap(&label, c1 + slot as f32 * (cw + 4.0 * sm), y, cw, waiting == Some((i, slot)), Action::Rebind(i as u8, slot as u8));
            }
            y += row;
        }
        y += 4.0 * s;
        self.button("RESET KEYS", "", x + m, y, false, Action::ResetKeys);
        self.cv.text("BACKSPACE CLEARS, ESC CANCELS", x + w - m, y + 7.0 * s, sm, DIM, Align::Right);
    }
}

pub fn best_line(best: &Best) -> String {
    if best.score <= 0.0 && best.time <= 0.0 {
        "NO QUALIFYING RUN ON FILE".to_string()
    } else if best.time > 0.0 {
        format!("BEST {} PTS // {}", pad(best.score, 5), format_time(best.time))
    } else {
        format!("BEST {} PTS", pad(best.score, 5))
    }
}

// ================================================================ flight overlays
pub struct Warn {
    pub text: &'static str,
    pub color: Col,
}

pub fn warnings(a: &Aircraft, m: &Mission) -> Vec<Warn> {
    let mut w = Vec::new();
    if pull_up(a) {
        w.push(Warn { text: "PULL UP", color: RED });
    }
    if a.departure > 0.4 {
        w.push(Warn { text: "DEPARTURE", color: RED });
    }
    if stall(a) {
        w.push(Warn { text: "STALL", color: RED });
    }
    if a.gear < 0.95 && !a.on_ground && a.agl < 300.0 && a.vs < -2.0 && a.airspeed_kt < 220.0 {
        w.push(Warn { text: "GEAR UP", color: RED });
    }
    if a.gear > 0.0 && a.airspeed_kt > crate::sim::spec::GEAR_LIMIT_KT {
        w.push(Warn { text: "GEAR SPEED", color: AMBER });
    }
    // The wing buzzing towards its flutter boundary, or past the placard.
    if a.structure.amplitude > 0.03 || a.structure.margin > 0.97 {
        w.push(Warn { text: "FLUTTER", color: RED });
    } else if a.structure.margin > 0.87 {
        w.push(Warn { text: "OVERSPEED", color: AMBER });
    }
    if a.structure.damage > 0.25 {
        w.push(Warn { text: "STRUCT DAMAGE", color: AMBER });
    }
    if a.g_smooth > 9.15 {
        w.push(Warn { text: "G LIMIT", color: AMBER });
    }
    if a.fuel < 400.0 {
        w.push(Warn { text: "BINGO", color: AMBER });
    }
    if m.timed && m.time_left < 20.0 {
        w.push(Warn { text: "TIME CRITICAL", color: AMBER });
    }
    w
}

pub fn pull_up(a: &Aircraft) -> bool {
    // With the gear down a gentle sink toward the ground is a landing.
    if a.on_ground || (a.gear > 0.9 && a.vs > -8.0) {
        return false;
    }
    let clearance = a.agl.min(if a.clearance > 0.0 { a.clearance } else { a.agl });
    let sink = -a.vs;
    let tti = if sink > 1.0 { clearance / sink } else { 99.0 };
    clearance < 260.0 && sink > 7.0 && (tti < 6.2 || clearance < 95.0)
}

/// Stalled, within a degree or so of the critical AoA for the present
/// condition, or too slow for the wing to hold much more than 1 g at this
/// weight and altitude.
pub fn stall(a: &Aircraft) -> bool {
    if a.on_ground {
        return false;
    }
    a.stall > 0.05 || a.alpha_deg.abs() > a.alpha_crit - 1.2 || a.g_avail < 1.25
}

fn gate_geometry(a: &Aircraft, m: &Mission) -> Option<(f64, f64, f64)> {
    let gate = m.gates.get(m.gate_index)?;
    let rel = gate.pos - a.pos;
    let bearing = rel.x.atan2(-rel.z) * RAD;
    let diff = wrap180(bearing - a.heading);
    let elevation = rel.y.atan2((rel.x * rel.x + rel.z * rel.z).sqrt()) * RAD;
    Some((rel.len(), diff, elevation))
}

/// Live flight-control positions, so a resting mouse offset is never hidden.
#[derive(Clone, Copy)]
pub struct StickCue {
    pub pitch: f64,
    pub yaw: f64,
    pub roll: f64,
    pub source: PitchSource,
    pub pad: bool,
    pub brake: bool,
}

/// Small stick box with its top-left corner at x, y: the dot moves the way
/// the mouse does (down is a pull), the bar below shows roll.
fn stick_cue(cv: &mut Canvas, x: f32, y: f32, s: f32, c: &StickCue) {
    let sm = (s - 1.0).max(1.0);
    let b = 22.0 * s;
    let (cx, cy, travel) = (x + b / 2.0, y + b / 2.0, b / 2.0 - 3.0 * s);
    cv.rect(x, y, b, b, rgba(4, 10, 6, 0.45));
    cv.stroke_rect(x, y, b, b, 1.0, with_alpha(HUD, 0.35));
    cv.line(cx, y + 2.0 * s, cx, y + b - 2.0 * s, 1.0, with_alpha(HUD, 0.16));
    cv.line(x + 2.0 * s, cy, x + b - 2.0 * s, cy, 1.0, with_alpha(HUD, 0.16));
    let (px, py) = (cx + clamp(c.yaw, -1.0, 1.0) as f32 * travel, cy + clamp(c.pitch, -1.0, 1.0) as f32 * travel);
    cv.rect(px - 1.5 * s, py - 1.5 * s, 3.0 * s, 3.0 * s, HUD);
    let ry = y + b + 4.0 * s;
    cv.rect(x, ry, b, 1.0, with_alpha(HUD, 0.3));
    cv.rect(cx + clamp(c.roll, -1.0, 1.0) as f32 * travel - 1.0 * s, ry - 2.0 * s, 2.0 * s, 4.0 * s, HUD);
    let label = match (c.pad, c.source) {
        (true, PitchSource::Mouse) => "PAD + MOUSE",
        (true, PitchSource::Keys) => "PAD + W S",
        (false, PitchSource::Mouse) => "PITCH MOUSE",
        (false, PitchSource::Keys) => "PITCH W S",
    };
    cv.text(label, x - 4.0 * s, y + 4.0 * s, sm, with_alpha(HUD, 0.55), Align::Right);
    if c.brake {
        cv.text("SPD BRK", x - 4.0 * s, y + b - 4.0 * s, sm, AMBER, Align::Right);
    }
}

/// Mission data at the canopy edges, outside the HUD combiner.
#[allow(clippy::too_many_arguments)]
pub fn cockpit_overlay(cv: &mut Canvas, w: f32, h: f32, a: &Aircraft, m: &Mission, toast: &str, toast_t: f64, cue: &StickCue, obj: Option<&Objective>) {
    let s = ui_scale(w, h);
    let sm = (s - 1.0).max(1.0);
    let (lx, rx) = (w * 0.026, w * 0.974);
    let gate_n = (m.gate_index + 1).min(m.gates.len());
    let title = obj.map(|o| o.label.clone()).unwrap_or_else(|| format!("GATE {}/{}", pad(gate_n as f64, 2), pad(m.gates.len() as f64, 2)));
    cv.text(&title, lx, h * 0.058, s, with_alpha(HUD, 0.78), Align::Left);
    let score = obj.map_or(m.final_score(), |o| o.score);
    if let Some((dist, diff, el)) = gate_geometry(a, m) {
        cv.text(&format!("{:.1} KM", dist / 1000.0), lx, h * 0.058 + 11.0 * s, sm, with_alpha(HUD, 0.58), Align::Left);
        cv.text(&format!("{} {}°  {}{}°", if diff >= 0.0 { "R" } else { "L" }, pad(diff.abs(), 2), if el >= 0.0 { "+" } else { "-" }, pad(el.abs(), 2)), lx, h * 0.058 + 20.0 * s, sm, with_alpha(HUD, 0.45), Align::Left);
    }
    air_panel(cv, lx, h * 0.058 + 34.0 * s, s, a);
    // Without a clock the corner counts the time flown instead.
    let clk = if m.timed { format!("CLK {}", format_time(m.time_left)) } else { format!("ET {}", format_time(m.elapsed)) };
    cv.text(&clk, rx, h * 0.058, s, if m.timed && m.time_left < 20.0 { RED } else { with_alpha(HUD, 0.82) }, Align::Right);
    cv.text(&format!("SCR {}", pad(score, 5)), rx, h * 0.058 + 11.0 * s, sm, with_alpha(HUD, 0.58), Align::Right);
    cv.text(&format!("{} KT", pad(a.airspeed_kt, 3)), rx, h * 0.058 + 20.0 * s, sm, with_alpha(HUD, 0.45), Align::Right);
    let keys = if cue.pad { "BACK  CONTROLS    RS  LOOK    Y  CAMERA" } else { "K  CONTROLS / CONFIG    ALT + MOUSE  LOOK    C  CENTER    V  CHASE" };
    cv.text(keys, 12.0 * sm, h - 10.0 * sm, sm, rgba(0xbf, 0xd0, 0xbf, 0.8), Align::Left);
    stick_cue(cv, rx - 22.0 * s, h - 20.0 * sm - 30.0 * s, s, cue);
    // Above the combiner, which the default head pitch lifts high on screen.
    toast_line(cv, w, 11.0 * s, s, toast, toast_t);
}

fn toast_line(cv: &mut Canvas, w: f32, y: f32, s: f32, toast: &str, t: f64) {
    if t > 0.0 && !toast.is_empty() {
        let alpha = clamp(t, 0.0, 1.0) as f32;
        let tw = Canvas::text_width(toast, s);
        cv.rect(w * 0.5 - tw * 0.5 - 6.0 * s, y - 7.0 * s, tw + 12.0 * s, 14.0 * s, rgba(4, 10, 6, 0.55 * alpha));
        cv.text(toast, w * 0.5, y, s, with_alpha(rgba(0xd8, 0xff, 0x86, 1.0), 0.62 + alpha * 0.36), Align::Center);
    }
}

fn project(cam: &Camera, w: f32, h: f32, p: V3) -> Option<(f32, f32)> {
    let rel = p - cam.eye;
    let z = rel.dot(cam.forward);
    if z <= 0.25 {
        return None;
    }
    let tan = (cam.fov / 2.0).tan();
    let aspect = (w / h) as f64;
    let nx = rel.dot(cam.right) / (z * tan * aspect);
    let ny = rel.dot(cam.up) / (z * tan);
    Some((((nx * 0.5 + 0.5) * w as f64) as f32, ((1.0 - (ny * 0.5 + 0.5)) * h as f64) as f32))
}

#[allow(clippy::too_many_arguments)]
pub fn chase_overlay(cv: &mut Canvas, w: f32, h: f32, a: &Aircraft, m: &Mission, cam: &Camera, toast: &str, toast_t: f64, time: f64, flying: bool, cue: &StickCue, obj: Option<&Objective>) {
    let score = obj.map_or(m.final_score(), |o| o.score);
    let cue_name = obj.map_or("GATE", |o| o.cue);
    let s = ui_scale(w, h);
    let sm = (s - 1.0).max(1.0);
    let edge = 12.0 * s;
    let (top_h, bottom_h) = (44.0 * s, 40.0 * s);
    cv.rect_grad(0.0, 0.0, w, top_h, rgba(4, 10, 6, 0.82), rgba(4, 10, 6, 0.0));
    cv.rect_grad(0.0, h - bottom_h, w, bottom_h, rgba(4, 10, 6, 0.0), rgba(4, 10, 6, 0.78));
    cv.rect(edge, top_h - 4.0 * s, w - 2.0 * edge, 1.0, with_alpha(HUD, 0.16));

    cv.text(&format!("{}  //  CHASE VIEW", a.kind.name()), edge, 10.0 * s, sm, HUD, Align::Left);
    cv.text("COLD RIVER  /  SECTOR 07", edge, 19.0 * s, sm, with_alpha(HUD, 0.48), Align::Left);
    let title = obj.map(|o| o.label.clone()).unwrap_or_else(|| format!("GATE {} / {}", pad((m.gate_index + 1).min(m.gates.len()) as f64, 2), pad(m.gates.len() as f64, 2)));
    cv.text(&title, w * 0.5, 9.0 * s, sm, AMBER, Align::Center);
    let clk = if m.timed { m.time_left } else { m.elapsed };
    cv.text(&format_time(clk), w * 0.5, 22.0 * s, s * 2.0, if m.timed && m.time_left < 20.0 { RED } else { HUD }, Align::Center);
    cv.text(&format!("{} KT", pad(a.airspeed_kt, 3)), w - edge, 10.0 * s, s, HUD, Align::Right);
    cv.text(&format!("ALT {} FT", pad(AirData::of(a).baro_ft, 5)), w - edge, 20.0 * s, sm, with_alpha(HUD, 0.68), Align::Right);
    cv.text(&format!("AOA {:.0}  GAVL {:.1}", a.alpha_deg, a.g_avail), w - edge, 28.0 * s, sm, with_alpha(HUD, 0.48), Align::Right);
    air_panel(cv, edge, top_h + 8.0 * s, s, a);
    cv.text(&format!("SCR {}", pad(score, 5)), w - edge, 36.0 * s, sm, with_alpha(HUD, 0.48), Align::Right);

    // Gate cue: diamond on screen, or an edge arrow toward it.
    if let Some(gate) = m.gates.get(m.gate_index) {
        let rel = gate.pos - cam.eye;
        let dist = rel.len();
        let p = project(cam, w, h, gate.pos);
        let (mx, mt, mb) = (30.0 * s, 56.0 * s, 52.0 * s);
        match p {
            Some((px, py)) if px > mx && px < w - mx && py > mt && py < h - mb => {
                let size = 5.0 * s + clamp(1.0 - dist / 5000.0, 0.0, 1.0) as f32 * 4.0 * s;
                cv.polyline(&[[px, py - size], [px + size, py], [px, py + size], [px - size, py]], 1.5, true, AMBER);
                cv.circle(px, py, size * 1.85, 1.0, with_alpha(AMBER, 0.3));
                cv.text(&format!("{cue_name} {}  {:.1} KM", pad((m.gate_index + 1) as f64, 2), dist / 1000.0), px, py + size * 2.6, sm, AMBER, Align::Center);
            }
            _ => {
                let tan = (cam.fov / 2.0).tan();
                let aspect = (w / h) as f64;
                let fz = rel.dot(cam.forward);
                let mut nx = rel.dot(cam.right) / (fz.abs() * tan * aspect).max(0.05);
                let mut ny = rel.dot(cam.up) / (fz.abs() * tan).max(0.05);
                if fz <= 0.0 {
                    nx = -nx;
                    ny = -ny;
                }
                let mag = (nx * nx + ny * ny).sqrt().max(1e-6);
                let (nx, ny) = ((nx / mag) as f32, (ny / mag) as f32);
                let (cx, cy, rx, ry) = (w * 0.5, h * 0.48, w * 0.39, h * 0.31);
                let (x, y) = (cx + nx.clamp(-1.0, 1.0) * rx, cy - ny.clamp(-1.0, 1.0) * ry);
                let ang = (-ny).atan2(nx);
                let (c, sn) = (ang.cos(), ang.sin());
                let rot = |px: f32, py: f32| [x + px * c - py * sn, y + px * sn + py * c];
                cv.tri(rot(0.0, 0.0), rot(-8.0 * s, -4.5 * s), rot(-8.0 * s, 4.5 * s), AMBER);
                cv.text(&format!("{}  {:.1} KM", pad((m.gate_index + 1) as f64, 2), dist / 1000.0), x, y + 11.0 * s, sm, AMBER, Align::Center);
            }
        }
        if let Some((_, diff, el)) = gate_geometry(a, m) {
            cv.text(&format!("TGT {} {}°   EL {}{}°", if diff >= 0.0 { "R" } else { "L" }, pad(diff.abs(), 2), if el >= 0.0 { "+" } else { "-" }, pad(el.abs(), 2)), edge, h - 34.0 * s, sm, AMBER, Align::Left);
        }
    }

    toast_line(cv, w, top_h + 12.0 * s, s, toast, toast_t);
    let warns = warnings(a, m);
    if !warns.is_empty() && flying && ((time * 4.5).floor() as i64) % 2 == 0 {
        let y = h * 0.60;
        cv.text(warns[0].text, w * 0.5, y, s * 2.0, warns[0].color, Align::Center);
        if let Some(w2) = warns.get(1) {
            cv.text(w2.text, w * 0.5, y + 16.0 * s, s, w2.color, Align::Center);
        }
    }
    cv.text(if cue.pad { "X CENTER  //  R STICK ORBIT" } else { "C CENTER  //  ALT + MOUSE ORBIT" }, edge, h - 24.0 * s, sm, with_alpha(HUD, 0.68), Align::Left);
    cv.text(&format!("FUEL {} LB", pad(a.fuel * 2.20462, 4)), edge, h - 14.0 * s, sm, if a.fuel < 500.0 { RED } else { with_alpha(HUD, 0.48) }, Align::Left);
    if a.gear > 0.0 || a.flaps > 0.0 {
        let gear = if a.gear >= 1.0 { "GEAR DN" } else if a.gear > 0.0 { "GEAR TRANSIT" } else { "GEAR UP" };
        let text = format!("{gear}  FLAPS {}{}", (a.flaps * crate::sim::spec::FLAP_MAX).round() as i64, if a.wheel_brake > 0.1 { "  BRAKES" } else { "" });
        cv.text(&text, edge, h - 44.0 * s, sm, if a.gear > 0.0 && a.gear < 1.0 { AMBER } else { with_alpha(HUD, 0.6) }, Align::Left);
    }
    cv.text(if cue.pad { "Y  COCKPIT VIEW" } else { "V  COCKPIT VIEW" }, w - edge, h - 24.0 * s, sm, with_alpha(HUD, 0.68), Align::Right);
    let hint = match (cue.pad, cue.source) {
        (true, _) => "L STICK  PITCH / ROLL  ,  LB RB  RUDDER",
        (false, PitchSource::Mouse) => "A / D  ROLL  ,  MOUSE PITCH / YAW",
        (false, PitchSource::Keys) => "A / D  ROLL  ,  W / S  PITCH  ,  MOUSE YAW",
    };
    cv.text(hint, w - edge, h - 14.0 * s, sm, with_alpha(HUD, 0.48), Align::Right);
    stick_cue(cv, w - edge - 22.0 * s, h - bottom_h - 34.0 * s, s, cue);
}

/// Horizontal meter from `lo` to `hi`, filled from `from` to `value`, with
/// tick lines through the bar and amber memory carets under it.
#[allow(clippy::too_many_arguments)]
fn bar_meter(cv: &mut Canvas, x: f32, y: f32, w: f32, h: f32, lo: f64, hi: f64, from: f64, value: f64, fill: Col, ticks: &[(f64, Col)], carets: &[f64]) {
    let px = |v: f64| (x + clamp((v - lo) / (hi - lo), 0.0, 1.0) as f32 * w).round();
    cv.rect(x, y, w, h, rgba(4, 10, 6, 0.7));
    let (a, b) = (px(from.min(value)), px(from.max(value)));
    cv.rect(a, y, (b - a).max(1.0), h, with_alpha(fill, 0.7));
    cv.stroke_rect(x, y, w, h, 1.0, with_alpha(HUD, 0.35));
    for &(v, c) in ticks {
        cv.rect(px(v), y - 2.0, 1.0, h + 4.0, c);
    }
    for &v in carets {
        let p = px(v);
        cv.tri([p, y + h + 1.0], [p - h * 0.6, y + h * 1.8 + 1.0], [p + h * 0.6, y + h * 1.8 + 1.0], AMBER);
    }
    cv.rect(px(value) - 1.0, y - 3.0, 2.0, h + 6.0, CREAM);
}

/// Air-data block: a G meter with the sortie's peak memories, a Mach meter
/// marked with the limit Mach at this pressure level (800 KCAS low down,
/// Mach 2.05 upstairs), and the air the jet is flying through.
fn air_panel(cv: &mut Canvas, x: f32, y: f32, s: f32, a: &Aircraft) {
    let ad = AirData::of(a);
    let u = (s - 1.0).max(1.0);
    let (w, line) = (158.0 * u, 9.0 * u);
    let rows = [
        format!("KCAS {}  TAS {}  GS {}", pad(ad.kcas, 3), pad(ad.ktas, 3), pad(ad.gs_kt, 3)),
        format!("BARO {} FT  QNH {:.2}", pad(ad.baro_ft, 5), ad.qnh_inhg),
        format!("PA {}  DA {} FT", ad.pressure_alt_ft.round() as i64, ad.density_alt_ft.round() as i64),
        format!("OAT {:+.0}°C  TAT {:+.0}°C", ad.oat_c, ad.tat_c),
        format!("RHO {:.3} ({:.0}%)  Q {:.0}KPA", ad.atm.rho, ad.sigma * 100.0, ad.qbar / 1000.0),
        // Gross weight and the drag the stores, gear and flaps add, in counts.
        format!("GW {}  DRAG +{}  T/W {:.2}", (a.mass * 2.20462 / 10.0).round() as i64 * 10, ((a.stores_drag + crate::sim::spec::GEAR_DRAG * a.gear + crate::sim::spec::FLAP_CD * a.flaps) * 10000.0).round() as i64, a.thrust_n / (a.mass * crate::sim::G0)),
    ];
    let rows_y = y + 34.0 * u;
    cv.rect(x - 4.0 * u, y - 5.0 * u, w + 8.0 * u, rows_y - y + rows.len() as f32 * line + 2.0 * u, rgba(4, 10, 6, 0.45));
    let (bx, bh) = (x + 40.0 * u, 5.0 * u);
    let bw = w - 40.0 * u;
    let g_col = if ad.g > 9.0 || ad.g < -3.0 { RED } else if ad.g > 7.0 || ad.g < -1.0 { AMBER } else { GREEN };
    cv.text(&format!("G {:+.1}", ad.g), x, y + bh * 0.5, u, g_col, Align::Left);
    let g_ticks = [(-3.0, RED), (0.0, DIM), (1.0, with_alpha(HUD, 0.8)), (5.0, DIM), (9.0, RED)];
    bar_meter(cv, bx, y, bw, bh, -3.0, 10.0, 1.0, ad.g, g_col, &g_ticks, &[ad.g_max, ad.g_min]);
    let y2 = y + 16.0 * u;
    let m_col = if ad.overspeed() { RED } else if ad.mach > ad.vmax_mach - 0.05 { AMBER } else { GREEN };
    cv.text(&format!("M {:.2}", ad.mach), x, y2 + bh * 0.5, u, m_col, Align::Left);
    bar_meter(cv, bx, y2, bw, bh, 0.0, 2.2, 0.0, ad.mach, m_col, &[(1.0, with_alpha(HUD, 0.8)), (ad.vmax_mach, RED)], &[]);
    for (i, r) in rows.iter().enumerate() {
        cv.text(r, x, rows_y + i as f32 * line, u, with_alpha(HUD, 0.7), Align::Left);
    }
}
