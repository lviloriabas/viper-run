//! Menu screens, the controls drawer and the flight overlays, drawn with the
//! pixel font. Screens return clickable buttons for mouse and keyboard use.
use crate::air::AirData;
use crate::canvas::{rgba, with_alpha, Align, Canvas, Col};
use crate::cockpit::pad;
use crate::combat::{Config, MissionKind, Store};
use crate::keymap::{key_name, Keymap, BINDS};
use crate::math::*;
use crate::settings::{Best, PadLayout, PitchSource, Settings, FOV_MAX, FOV_MIN};
use crate::scenario;
use crate::sim::{Aircraft, Mission};
use crate::targets;
use crate::terrain;
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
    School,
    NetTab(crate::multiplayer::Panel), NetFilter, NetPage(bool),
    Online, NetEdit(crate::multiplayer::Field), NetMode, NetCapacity, NetPublic,
    NetHost, NetJoin, NetPasteJoin, NetBrowse, NetNearby, NetHosting, NetHelp, NetHelpClose, NetRoom(u8), NetNearbyRoom(u8), NetCopy(bool), NetCopyManual, NetCopySetup, NetReady, NetLaunch, NetLeave, NetResume,
    VoiceMode, VoiceMute, VoiceDeafen, VoiceInput(bool), VoiceOutput(bool), VoiceThreshold(bool), VoicePeer(u64), VoicePeerGain(u64),
    Lesson(crate::tutorial::Lesson),
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
    PadSetup,
    PadExpo(bool),
    PadDeadLeft(bool),
    PadDeadRight(bool),
    PadRudderClick,
    PadFineAim,
    PadCalibrate,
    PadReset,
    Fullscreen,
    Performance,
    Reset,
    /// Choose the mission on the start screen.
    Pick(MissionKind),
    /// Loadout screen: next store for a station pair (0 tips, 1 stations 2
    /// and 8, 2 outboard, 3 inboard), next fuel load, and take off.
    Station(u8),
    Fuel,
    Launch,
    /// Next airframe for the player, and for the bandits.
    Jet,
    Aggressor,
    /// Threat setup: next prepared layout, and one more bandit, SA-6
    /// battery or ZSU-23-4 (wrapping to none).
    Layout,
    Bandits,
    Sams,
    Guns,
    /// Mission clocks on or off.
    Clock,
    /// Free flight's auto refill on or off.
    Refill,
    /// Wait for a key for an action (index into keymap::BINDS) and slot.
    Rebind(u8, u8),
    ResetKeys,
}

impl Action {
    pub fn pad_setting(self) -> bool {
        matches!(self, Self::PadSetup | Self::PadExpo(_) | Self::PadDeadLeft(_) | Self::PadDeadRight(_) | Self::PadRudderClick | Self::PadFineAim | Self::PadCalibrate | Self::PadReset)
    }
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

/// Choose the closest button in a direction, favouring the same row/column.
pub fn navigation(buttons: &[Button], selected: usize, dx: f32, dy: f32) -> Option<usize> {
    let a = buttons.get(selected)?;
    let centre = |b: &Button| (b.x + b.w * 0.5, b.y + b.h * 0.5);
    let (x, y) = centre(a);
    buttons.iter().enumerate().filter(|(i, _)| *i != selected).filter_map(|(i, b)| {
        let (bx, by) = centre(b);
        let along = (bx - x) * dx + (by - y) * dy;
        let across = ((bx - x) * dy - (by - y) * dx).abs();
        (along > 1.0).then_some((i, along + across * 3.0))
    }).min_by(|a, b| a.1.total_cmp(&b.1)).map(|x| x.0)
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
    pub click_rudder: bool,
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
    /// What brought the jet down in the air, as the damage names it.
    pub cause: &'static str,
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
    // A title bar can remove a few pixels from a nominal 720p/1080p window.
    // Keep that from shrinking every overlay by an entire pixel-font step.
    ((w / 640.0).min(h / 360.0) + 0.08).floor().clamp(1.0, 4.0)
}

impl<'a> Ui<'a> {
    /// A shortcut as the active input shows it: the keyboard key, or the
    /// controller button that does the same from anywhere on the screen
    /// (none if there is none). Enter and {A} only ever act on the
    /// highlighted button, so they are never a button's shortcut: the
    /// legend at the foot of the screen names them once.
    fn hint<'b>(&self, key: &'b str) -> &'b str {
        if !self.pad {
            return key;
        }
        match key {
            "SPACE" | "P" | "R" => "{START}",
            "ESC" => "{B}",
            "K" => "{BACK}",
            _ => "",
        }
    }

    /// The prompt legend along the foot of a menu: move and select, which
    /// act on the highlighted button, then the screen's own shortcuts.
    fn legend(&mut self, shortcuts: &[(&str, &str)]) {
        let s = self.s;
        let sm = s.max(2.0) - 1.0;
        let mut items: Vec<(&str, &str)> = if self.pad { vec![("{DPAD}", "MOVE"), ("{A}", "SELECT")] } else { vec![("ARROWS", "MOVE"), ("ENTER", "SELECT")] };
        items.extend(shortcuts.iter().map(|&(k, v)| (self.hint(k), v)).filter(|(k, _)| !k.is_empty()));
        let gap = 10.0 * s;
        let widths: Vec<f32> = items.iter().map(|(k, v)| Canvas::text_width(k, sm) + 4.0 * sm + Canvas::text_width(v, sm)).collect();
        let total = widths.iter().sum::<f32>() + gap * (items.len() as f32 - 1.0);
        let (mut x, y) = ((self.w - total) / 2.0, self.h - 6.0 * s);
        self.cv.rect(x - 6.0 * s, y - 6.0 * s, total + 12.0 * s, 12.0 * s, rgba(0x05, 0x08, 0x07, 0.7));
        for ((k, v), w) in items.iter().zip(widths) {
            self.cv.text(k, x, y, sm, ACID, Align::Left);
            self.cv.text(v, x + Canvas::text_width(k, sm) + 4.0 * sm, y, sm, with_alpha(CREAM, 0.75), Align::Left);
            x += w + gap;
        }
    }

    /// Width `button` gives a label and its shortcut.
    fn button_width(&self, label: &str, hint: &str) -> f32 {
        let s = self.s;
        let pad_hint = match hint { "SPACE" | "P" | "R" => "{START}", "ESC" => "{B}", "K" => "{BACK}", _ => hint };
        let reserve = Canvas::text_width(hint, s.max(2.0) - 1.0).max(Canvas::text_width(pad_hint, s.max(2.0) - 1.0));
        Canvas::text_width(label, s) + if hint.is_empty() { 0.0 } else { reserve + 8.0 * s } + 16.0 * s
    }

    fn button(&mut self, label: &str, hint: &str, x: f32, y: f32, primary: bool, action: Action) -> f32 {
        let w = self.button_width(label, hint);
        let hint = self.hint(hint);
        let s = self.s;
        let h = 15.0 * s;
        let i = self.buttons.len();
        let hot = self.selected == i || Button { x, y, w, h, action }.hit(self.mouse.0, self.mouse.1);
        let (bg, fg, border) = if primary {
            (if hot { CREAM } else { ACID }, INK, ACID)
        } else {
            (if hot { rgba(0x1a, 0x2a, 0x20, 0.95) } else { rgba(0x0a, 0x11, 0x0e, 0.6) }, if hot { ACID } else { GREEN }, if hot { ACID } else { LINE_HOT })
        };
        self.cv.layout_item(crate::menu_layout::Kind::Button,label,[x,y,w,h]);
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

    fn button_row(&mut self, items: &[(&str,&str,Action,bool)], x:f32, y:f32, width:f32) -> f32 {
        let mut bx=x; let mut by=y;
        for &(label,hint,action,primary) in items {
            let w=self.button_width(label,hint);
            if bx>x && bx+w>x+width {bx=x;by+=19.0*self.s;}
            bx+=self.button(label,hint,bx,by,primary,action)+6.0*self.s;
        }
        by+15.0*self.s
    }

    /// A fixed-width key cap in the small font, for the bindings table.
    fn key_cap(&mut self, label: &str, x: f32, y: f32, w: f32, hot: bool, action: Action) {
        let sm = self.s.max(2.0) - 1.0;
        let h = 9.0 * sm;
        let i = self.buttons.len();
        let focus = hot || i == self.selected || Button { x, y, w, h, action }.hit(self.mouse.0, self.mouse.1);
        self.cv.layout_item(crate::menu_layout::Kind::Button,label,[x,y,w,h]);
        let (bg, fg, border) = if focus { (rgba(0x1a, 0x2a, 0x20, 0.95), ACID, ACID) } else { (rgba(0x0a, 0x11, 0x0e, 0.6), GREEN, LINE_HOT) };
        self.cv.rect(x, y, w, h, bg);
        self.cv.stroke_rect(x, y, w, h, 1.0, border);
        self.cv.text(label, x + w * 0.5, y + h * 0.5, sm, fg, Align::Center);
        self.buttons.push(Button { x, y, w, h, action });
    }

    fn frame(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.cv.layout_item(crate::menu_layout::Kind::Panel,"frame",[x,y,w,h]);
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
        let (pw, ph) = ((self.w - 24.0 * s).min(600.0 * s), (self.h - 32.0 * s).min(290.0 * s));
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
        self.cv.text(&format!("{} // COCKPIT", cfg.jet.name()), x + m, yy, s, AMBER, Align::Left);
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
            MissionKind::Gates => ("Thread ten gates down the river gorge and make the coast before the clock runs out.", "THE JET IS LOADED. THE WEATHER IS PERFECT. THE MOUNTAIN IS NOT."),
            MissionKind::Dogfight => ("Aggressor jets are inbound. Find them, shoot them down and come home.", "SAME JET. SAME LIMITS. ONLY ONE OF YOU GOES HOME."),
            MissionKind::Airfield => ("Take off from the valley airbase, fly out to the steerpoint and bring the jet back for a clean landing.", "THE RUNWAY IS LONG. THE APPROACH IS NOT FORGIVING."),
            MissionKind::Strike => ("Drop the Cold River bridge and the SAM site that guards it. Expect fire from the ground.", "FOUR BOMBS. MAKE THEM COUNT."),
            MissionKind::Free => ("Fly the valley at your own pace. The gates, targets and bandits are there for practice, and nothing shoots back.", "NO CLOCK. NO SCORE. JUST THE VALLEY."),
        };
        yy += self.cv.paragraph(copy, x + m, yy, left_w - m * 1.5, s.max(2.0) - 1.0, 1.7, rgba(0xc7, 0xd6, 0xc2, 1.0));
        yy += 2.0 * s;
        yy += self.cv.paragraph(tag, x + m, yy, left_w - m * 1.5, s.max(2.0) - 1.0, 1.7, CREAM);
        yy += 8.0 * s;
        let bx = x + m;
        // The mission row sits above ENGAGE but comes after it in the
        // keyboard order, so ENGAGE is highlighted first. The jet, the
        // bandits and the clock are set on each mission's setup screen.
        let picks_y = yy;
        yy += 19.0 * s;
        let w1 = self.button("ENGAGE", "SPACE", bx, yy, true, Action::Start);
        self.button(if muted { "AUDIO OFF" } else { "AUDIO ON" }, "M", bx + w1 + 6.0 * s, yy, false, Action::Mute);
        yy += 19.0 * s;
        self.button_row(&[("CONTROLS / CONFIG","K",Action::Controls,false),
            ("FLIGHT SCHOOL","",Action::School,false),("ONLINE","",Action::Online,false),
            ("QUIT","",Action::Quit,false)],bx,yy,left_w-m-6.0*s);
        let mut mx = bx;
        for k in MissionKind::ALL {
            // The chosen mission is the lit button; free flight's name is cut
            // short so the row stays clear of the briefing card.
            let label = if k == MissionKind::Free { "FREE FLT" } else { k.name() };
            mx += self.button(label, "", mx, picks_y, k == mission, Action::Pick(k)) + 4.0 * s;
        }
        self.legend(&[("SPACE", "ENGAGE"), ("K", "CONTROLS")]);

        // Briefing card.
        let (bxx, byy, bw) = (x + left_w + 6.0 * s, y + 30.0 * s, pw - left_w - m - 6.0 * s);
        let bh = ph - 52.0 * s;
        self.cv.layout_item(crate::menu_layout::Kind::Panel,"MISSION DATA",[bxx,byy,bw,bh]);
        self.cv.rect(bxx, byy, bw, bh, rgba(0x05, 0x0b, 0x08, 0.8));
        self.cv.stroke_rect(bxx, byy, bw, bh, 1.0, LINE);
        let sm = s.max(2.0) - 1.0;
        self.cv.text(&format!("MISSION DATA // {}", mission.name()), bxx + 6.0 * s, byy + 8.0 * s, sm, AMBER, Align::Left);
        let airframe = match cfg.jet {
            crate::sim::Type::F16 => ("AIRFRAME", crate::sim::Type::F16.full_name(), "LOCKHEED MARTIN, F110-GE-129"),
            crate::sim::Type::Mig29 => ("AIRFRAME", crate::sim::Type::Mig29.full_name(), "MIKOYAN, 2 X KLIMOV RD-33"),
            crate::sim::Type::F18 => ("AIRFRAME", crate::sim::Type::F18.full_name(), "MCDONNELL DOUGLAS, 2 X F404-GE-402"),
            crate::sim::Type::F18E|crate::sim::Type::EA18G => ("AIRFRAME", cfg.jet.full_name(), "BOEING, 2 X F414-GE-400"),
            crate::sim::Type::Su27 => ("AIRFRAME", crate::sim::Type::Su27.full_name(), "SUKHOI, 2 X AL-31F"),
        };
        let bandits = format!("{} BANDITS", cfg.threats.bandits);
        let heaters = match (cfg.threats.mixed, cfg.aggressor) {
            (true, _) => "AIM-9 + R-73",
            (false, crate::sim::Type::Mig29 | crate::sim::Type::Su27) => "R-73",
            (false, crate::sim::Type::F16 | crate::sim::Type::F18 | crate::sim::Type::F18E | crate::sim::Type::EA18G) => "AIM-9",
        };
        let fleet = format!("{}, {heaters}, GUN", cfg.fleet_name());
        let stores = cfg.stores_summary();
        let gun = crate::weapons::gun_of(cfg.jet);
        let gun = format!("{}, {} ROUNDS", gun.name, gun.rounds);
        let rows = match mission {
            MissionKind::Gates => [airframe, ("ROUTE", "10 GATES", "RIVER GORGE, COAST EGRESS"), ("CLOCK", "02:30", "+ 8 SEC PER GATE"), ("HAZARD", "TERRAIN", "STALL, G-LIMIT")],
            MissionKind::Dogfight => [airframe, ("THREAT", &bandits, &fleet), ("STORES", &stores, &gun), ("CLOCK", "06:00", "SPLASH ALL THREE")],
            MissionKind::Airfield => [airframe, ("ROUTE", "RWY A, STPT 1, RWY A", "9 KM PAST THE FAR END, 3,000 FT"), ("CLOCK", "10:00", "FULL-STOP LANDING"), ("SCORING", "TOUCHDOWN", "SINK RATE AND AIM POINT")],
            MissionKind::Strike => [airframe, ("TARGETS", "BRIDGE, SA-6 SITE", "CONVOY, DEPOT, ZSU-23-4"), ("STORES", &stores, &gun), ("CLOCK", "07:00", "DROP BRIDGE, KILL THE SAM")],
            MissionKind::Free => [airframe, ("ROUTE", "10 GATES", "RIVER GORGE, ON A LOOP"), ("TARGETS", "2 BANDITS, SA-6", "HOLD FIRE, BACK IN 30 SEC"), ("CLOCK", "OFF", "STOP ON RWY A TO REARM")],
        };
        let rows = rows.map(|r| if !clock && r.0 == "CLOCK" { ("CLOCK", "OFF", "NO TIME LIMIT") } else { r });
        let mut ry = byy + 20.0 * s;
        for (k, v1, v2) in rows {
            self.cv.text(k, bxx + 6.0 * s, ry, sm, DIM, Align::Left);
            let vx=bxx+6.0*s+58.0*sm;let vw=bxx+bw-6.0*s-vx;
            let h1=self.cv.paragraph(v1,vx,ry-3.5*sm,vw,sm,1.3,CREAM);
            let h2=self.cv.paragraph(v2,vx,ry-3.5*sm+h1,vw,sm,1.3,with_alpha(GREEN,0.75));
            ry += (h1+h2+4.0*sm).max(22.0*sm);
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
        let mut keys = match (armed, self.pad, self.layout) {
            (true, true, PadLayout::Combat) => [("{LS}", "PITCH / ROLL"), ("{RS}", "LOOK"), ("{RT}", "FIRE"), ("{RB}", "LOCK TARGET"), ("{LB}", "THROTTLE UP"), ("{LT}", "THROTTLE DOWN"), ("{A}", "NEXT WEAPON"), ("{L3}", "CHAFF / FLARE"), ("{X} + {LS}", "RUDDER"), ("{B}", "AIRBRAKE"), ("{Y}", "CAMERA"), ("{START}", "PAUSE")],
            (true, true, PadLayout::Classic) => [("{LS}", "PITCH / ROLL"), ("{RS}", "LOOK"), ("{A}", "FIRE"), ("{DD}", "LOCK TARGET"), ("{RT}", "THROTTLE UP"), ("{LT}", "THROTTLE DOWN"), ("{DU}", "NEXT WEAPON"), ("{L3}", "CHAFF / FLARE"), ("{LB} {RB}", "RUDDER"), ("{B}", "AIRBRAKE"), ("{Y}", "CAMERA"), ("{START}", "PAUSE")],
            (false, true, PadLayout::Combat) => [("{LS}", "PITCH / ROLL"), ("{RS}", "LOOK"), ("{LB}", "THROTTLE UP"), ("{LT}", "THROTTLE DOWN"), ("{X} + {LS}", "RUDDER"), ("{B}", "AIRBRAKE"), ("{DD}", "LANDING GEAR"), ("{DU}", "FLAPS"), ("{LT} GROUND", "WHEEL BRAKES"), ("{Y}", "CAMERA"), ("{START}", "PAUSE"), ("{BACK}", "CONTROLS")],
            (false, true, PadLayout::Classic) => [("{LS}", "PITCH / ROLL"), ("{RS}", "LOOK"), ("{RT}", "THROTTLE UP"), ("{LT}", "THROTTLE DOWN"), ("{LB} {RB}", "RUDDER"), ("{B}", "AIRBRAKE"), ("{X}", "LANDING GEAR"), ("{R3} HOLD", "ZOOM"), ("{LT} GROUND", "WHEEL BRAKES"), ("{Y}", "CAMERA"), ("{START}", "PAUSE"), ("{BACK}", "CONTROLS")],
            (true, false, _) => [k1, ("A D", "ROLL"), ("F L-CLICK", "FIRE"), ("T", "LOCK TARGET"), ("SHIFT", "THROTTLE UP"), ("CTRL", "THROTTLE DOWN"), ("X", "NEXT WEAPON"), ("Z", "CHAFF / FLARE"), ("Q E", "RUDDER"), ("H", "AIRBRAKE"), ("V", "CAMERA"), ("ESC", "CONFIG")],
            (false, false, _) => [k1, ("A D", "ROLL"), ("SHIFT", "THROTTLE UP"), ("CTRL", "THROTTLE DOWN"), ("Q E", "RUDDER"), ("H", "AIRBRAKE"), ("G", "LANDING GEAR"), ("L", "FLAPS"), ("B", "WHEEL BRAKES"), ("V", "CAMERA"), ("ALT+MOUSE", "LOOK"), ("ESC", "CONFIG")],
        };
        if self.pad && self.click_rudder {
            for row in &mut keys {
                if row.1 == "RUDDER" { row.0 = "{X} / {L3} + {LS}"; }
                if row.1 == "CHAFF / FLARE" { row.0 = "{X} + {L3}"; }
            }
        }
        self.cv.text(if self.pad { "XBOX CONTROLLER" } else { "FLIGHT CONTROLS" }, bxx + 6.0 * s, ry, sm, AMBER, Align::Left);
        ry += 10.0 * sm;
        let col_w=(bw-16.0*s)/2.0;
        for pair in keys.chunks(2) {
            let mut height=10.0*sm;
            for (col,&(k,v)) in pair.iter().enumerate() {
                let kx=bxx+6.0*s+col as f32*(col_w+4.0*s);
                let kw=Canvas::text_width(k,sm)+4.0*sm;
                self.cv.text(k,kx,ry,sm,ACID,Align::Left);
                if kw+Canvas::text_width(v,sm)<=col_w {
                    self.cv.text(v,kx+kw,ry,sm,with_alpha(CREAM,0.7),Align::Left);
                }else{
                    let used=self.cv.paragraph(v,kx,ry+6.0*sm,col_w,sm,1.3,with_alpha(CREAM,0.7));
                    height=height.max(used+12.0*sm);
                }
            }
            ry+=height;
        }
        // Footer.
        let fy = y + ph - 12.0 * s;
        self.cv.line(x + m, fy - 8.0 * s, x + pw - m, fy - 8.0 * s, 1.0, LINE);
        self.cv.text(&best_line(best), x + m, fy, sm, AMBER, Align::Left);
        self.cv.text("37°14′N 081°06′E  ALT 2,140 FT", x + pw - m, fy, sm, DIM, Align::Right);
    }

    pub fn school_screen(&mut self, completed: u8, jet: crate::sim::Type) {
        let s = self.s;
        let (pw, ph) = ((self.w - 24.0*s).min(570.0*s), (self.h - 32.0*s).min(295.0*s));
        let (x, y) = ((self.w-pw)/2.0, (self.h-ph)/2.0);
        self.frame(x, y, pw, ph);
        self.cv.text("FLIGHT SCHOOL", x+14.0*s, y+18.0*s, 2.0*s, GREEN, Align::Left);
        self.cv.text("NO CLOCK. NO RETURN FIRE. OBJECTIVES CHECK YOUR FLIGHT.", x+14.0*s, y+38.0*s, (s-1.0).max(1.0), DIM, Align::Left);
        let mut yy = y+58.0*s;
        for l in crate::tutorial::Lesson::ALL {
            let passed = completed & (1 << l.index()) != 0;
            self.button(l.name(), "", x+14.0*s, yy, !passed, Action::Lesson(l));
            self.cv.paragraph(l.brief(), x+155.0*s, yy+2.0*s, pw-170.0*s, (s-1.0).max(1.0), 1.35, CREAM);
            if passed { self.cv.text("PASSED", x+14.0*s, yy+23.0*s, (s-1.0).max(1.0), ACID, Align::Left); }
            yy += 39.0*s;
        }
        let w = self.button(&format!("AIRFRAME: {}", jet.name()), "", x+14.0*s, y+ph-26.0*s, false, Action::Jet);
        self.button("MAIN MENU", "ESC", x+20.0*s+w, y+ph-26.0*s, false, Action::Menu);
        self.legend(&[("ESC", "BACK"), ("K", "CONTROLS")]);
    }

    pub fn training_result(&mut self, t: &crate::tutorial::Training, outcome: Outcome) {
        let passed = outcome == Outcome::Win && t.done();
        self.status_panel("// FLIGHT SCHOOL", DIM, if passed { "LESSON PASSED" } else { "TRY AGAIN" },
            if passed { ACID } else { AMBER }, t.lesson.brief(),
            [("LESSON", t.lesson.name().to_string(), CREAM), ("STEPS", format!("{}/{}", t.stage, t.lesson.steps()), GREEN),
             ("TIME", format_time(t.elapsed), CREAM), ("PROGRESS", if passed { "SAVED" } else { "RETRY" }.to_string(), AMBER)],
            &[("REPEAT", "R", Action::Lesson(t.lesson)), ("FLIGHT SCHOOL", "", Action::School), ("MAIN MENU", "ESC", Action::Menu)],
            "ESC CONFIGURATION DURING FLIGHT INCLUDES RESTART.");
    }

    fn network_field(&mut self, label:&str, value:&str, field:crate::multiplayer::Field, editing:bool, enabled:bool, x:f32,y:f32,w:f32) {
        let s=self.s;let sm=(s-1.0).max(1.0);let h=22.0*s;
        let action=Action::NetEdit(field);
        let hot=enabled&&(editing||self.selected==self.buttons.len()||Button{x,y,w,h,action}.hit(self.mouse.0,self.mouse.1));
        self.cv.layout_item(if enabled{crate::menu_layout::Kind::Button}else{crate::menu_layout::Kind::Panel},label,[x,y,w,h]);
        self.cv.rect(x,y,w,h,INK);self.cv.stroke_rect(x,y,w,h,1.0,if hot{ACID}else{LINE});
        self.cv.text(label,x+5.0*s,y+5.0*s,sm,DIM,Align::Left);
        let count=((w-12.0*s)/(6.0*sm)).max(5.0)as usize;
        let shown=if value.len()>count{format!("...{}",&value[value.len()-count+3..])}else{value.to_string()};
        self.cv.text(&format!("{}{}",shown,if editing{"_"}else{""}),x+5.0*s,y+15.0*s,sm,if hot{ACID}else{CREAM},Align::Left);
        if enabled {self.buttons.push(Button{x,y,w,h,action});}
    }
    pub fn online_controls_button(&mut self) {
        self.button("ONLINE / VOICE", "", 12.0*self.s, self.h-30.0*self.s, false, Action::Online);
    }
    fn server_browser(&mut self,mp:&crate::multiplayer::Multiplayer,jet:crate::sim::Type) {
        use crate::multiplayer::{Field,Panel};
        let s=self.s;let sm=(s-1.0).max(1.0);
        let pw=(self.w-20.0*s).min(610.0*s);let ph=320.0*s;
        let (x,y)=((self.w-pw)/2.0,(self.h-ph)/2.0);
        let left=x+12.0*s;let width=pw-24.0*s;
        self.frame(x,y,pw,ph);
        self.cv.text("MULTIPLAYER",left,y+16.0*s,2.0*s,GREEN,Align::Left);
        let menu=&mp.menu;
        let mut bx=left;
        for (label,panel) in [("CREATE GAME",Panel::Create),("JOIN FRIEND",Panel::Invite),("NEARBY",Panel::Nearby),("ADVANCED",Panel::Browser)] {
            bx+=self.button(label,"",bx,y+32.0*s,menu.panel==panel,Action::NetTab(panel))+5.0*s;
        }
        let half=(width-8.0*s)/2.0;
        self.network_field("CALLSIGN",&menu.name,Field::Name,menu.editing==Some(Field::Name),true,left,y+57.0*s,half);
        self.button(&format!("JET: {}",["F-16C","MIG-29S","F/A-18C","SU-27SK"][jet.index()]),"",left+half+8.0*s,y+60.0*s,false,Action::Jet);
        match menu.panel {
            Panel::Browser => {
                self.network_field("RELAY SERVER (HOST OR HOST:PORT)",&menu.server,Field::Server,menu.editing==Some(Field::Server),true,left,y+84.0*s,width);
                let mut bx=left;
                bx+=self.button("REFRESH","",bx,y+112.0*s,false,Action::NetBrowse)+5.0*s;
                self.button(menu.filter.map_or("ALL MODES",|m|m.name()),"",bx,y+112.0*s,false,Action::NetFilter);
                self.cv.text("ROOM / MODE",left,y+143.0*s,sm,DIM,Align::Left);
                self.cv.text("PILOTS",left+width-62.0*s,y+143.0*s,sm,DIM,Align::Left);
                if let Some(browser)=&menu.browser {
                    for(i,room)in browser.rooms.iter().enumerate() {
                        let yy=y+(151.0+i as f32*18.0)*s;
                        let label=format!("{} / {}",room.title,room.mode.name());
                        self.key_cap(&label,left,yy,width-70.0*s,false,Action::NetRoom(i as u8));
                        self.cv.text(&format!("{}/{} {}",room.players,room.capacity,if room.players>=room.capacity{"FULL"}else{"JOIN"}),left+width-62.0*s,yy+6.0*s,sm,if room.players>=room.capacity{DIM}else{ACID},Align::Left);
                    }
                    let bx=left+self.button("PREV","",left,y+264.0*s,false,Action::NetPage(false))+5.0*s;
                    let bx=bx+self.button("NEXT","",bx,y+264.0*s,false,Action::NetPage(true))+8.0*s;
                    self.cv.text(&format!("PAGE {} / {}",browser.page+1,browser.pages),bx,y+272.0*s,sm,DIM,Align::Left);
                    if browser.rooms.is_empty() {
                        self.cv.paragraph(&browser.status(),left,y+172.0*s,width,sm,1.5,CREAM);
                    }
                } else {
                    self.cv.paragraph("Optional: browse an external relay. Enter its address and refresh. To play without a separate server, choose CREATE GAME or NEARBY.",left,y+163.0*s,width,sm,1.6,CREAM);
                    self.cv.paragraph("An external relay helps when a router or Internet provider blocks hosting. Players only make outgoing connections.",left,y+211.0*s,width,sm,1.6,DIM);
                }
            }
            Panel::Nearby => {
                self.cv.paragraph("Same Wi-Fi or local network? Your friend's public game appears here automatically.",left,y+86.0*s,width,sm,1.5,CREAM);
                let mut bx=left;
                bx+=self.button("SEARCH AGAIN","",bx,y+112.0*s,false,Action::NetNearby)+5.0*s;
                self.button(menu.filter.map_or("ALL MODES",|m|m.name()),"",bx,y+112.0*s,false,Action::NetFilter);
                if let Some(nearby)=&menu.nearby {
                    for(i,room)in nearby.rooms.iter().enumerate() {
                        let yy=y+(151.0+i as f32*18.0)*s;
                        self.key_cap(&format!("{} / {}",room.title,room.mode.name()),left,yy,width-70.0*s,false,Action::NetNearbyRoom(i as u8));
                        self.cv.text(&format!("{}/{} {}",room.players,room.capacity,if room.players>=room.capacity{"FULL"}else{"JOIN"}),left+width-62.0*s,yy+6.0*s,sm,ACID,Align::Left);
                    }
                    if nearby.rooms.is_empty() {
                        self.cv.paragraph("Looking for games... The host selects CREATE GAME first. If nothing appears, ask for a LAN invite and use JOIN FRIEND.",left,y+154.0*s,width,sm,1.6,CREAM);
                        self.button("CONNECTION HELP","",left,y+222.0*s,false,Action::NetHelp);
                    }
                    let bx=left+self.button("PREV","",left,y+264.0*s,false,Action::NetPage(false))+5.0*s;
                    let bx=bx+self.button("NEXT","",bx,y+264.0*s,false,Action::NetPage(true))+8.0*s;
                    self.cv.text(&format!("PAGE {} / {}",nearby.page+1,nearby.pages),bx,y+272.0*s,sm,DIM,Align::Left);
                }
            }
            Panel::Create => {
                self.button(if menu.external_server{"USE EXTERNAL RELAY"}else{"HOST ON THIS PC"},"CHANGE",left,y+84.0*s,false,Action::NetHosting);
                if menu.external_server {
                    self.network_field("EXTERNAL RELAY ADDRESS",&menu.server,Field::Server,menu.editing==Some(Field::Server),true,left,y+112.0*s,width);
                } else {
                    self.cv.paragraph("The server starts inside the game. Keep this game open while your friends play.",left,y+115.0*s,width,sm,1.5,CREAM);
                }
                let mut bx=left;
                bx+=self.button(menu.rules.mode.name(),"",bx,y+148.0*s,false,Action::NetMode)+5.0*s;
                bx+=self.button(&format!("SLOTS {}",menu.rules.capacity),"",bx,y+148.0*s,false,Action::NetCapacity)+5.0*s;
                self.button(if menu.public{"PUBLIC"}else{"INVITE ONLY"},"",bx,y+148.0*s,false,Action::NetPublic);
                self.cv.paragraph(menu.rules.mode.brief(),left,y+183.0*s,width,sm,1.6,CREAM);
                self.cv.paragraph(if menu.external_server{"Friends can join through an invitation or the external server browser."}else{"Same network: your friend opens NEARBY. Internet: share the Internet invite when router setup finishes. Windows may ask to allow hosting."},left,y+218.0*s,width,sm,1.5,DIM);
                let bx=left+self.button("CREATE GAME","",left,y+264.0*s,true,Action::NetHost)+8.0*s;
                self.button("CONNECTION HELP","",bx,y+264.0*s,false,Action::NetHelp);
            }
            Panel::Invite => {
                self.network_field("INVITATION (CTRL+V TO PASTE)",&menu.invite,Field::Invite,menu.editing==Some(Field::Invite),true,left,y+84.0*s,width);
                self.button("PASTE AND JOIN","",left,y+117.0*s,true,Action::NetPasteJoin);
                self.cv.paragraph("Copy your friend's invitation, then select PASTE AND JOIN. You can also paste into the field and press Enter to connect.",left,y+155.0*s,width,sm,1.6,CREAM);
                self.cv.paragraph("Use a LAN invite on the same network, or an Internet invite from another network. The invitation includes everything needed.",left,y+205.0*s,width,sm,1.6,DIM);
                let bx=left+self.button("JOIN ROOM","",left,y+264.0*s,false,Action::NetJoin)+8.0*s;
                self.button("CONNECTION HELP","",bx,y+264.0*s,false,Action::NetHelp);
            }
        }
        let max=(width/(6.0*sm))as usize;
        self.cv.text(&menu.status.chars().take(max).collect::<String>(),left,y+296.0*s,sm,AMBER,Align::Left);
        self.button("MAIN MENU","ESC",left,y+303.0*s,false,Action::Menu);
        self.legend(&[("ESC","BACK")]);
    }
    pub fn network_screen(&mut self,mp:&crate::multiplayer::Multiplayer,jet:crate::sim::Type,keys:&crate::keymap::Keymap) {
        use crate::multiplayer::Field;
        if mp.menu.help {
            if let Some(server)=&mp.server {
                let s=self.s;let sm=(s-1.0).max(1.0);let pw=(self.w-20.0*s).min(610.0*s);let ph=330.0*s;
                let(x,y)=((self.w-pw)/2.0,(self.h-ph)/2.0);let left=x+12.0*s;let width=pw-24.0*s;
                self.frame(x,y,pw,ph);
                self.cv.text("HOST INTERNET SETUP",left,y+16.0*s,2.0*s,GREEN,Align::Left);
                self.cv.text(&format!("HOST PC: {}   TCP PORT: {}",server.lan.ip(),server.lan.port()),left,y+44.0*s,sm,ACID,Align::Left);
                let router=server.router_address.map_or("OPEN YOUR ROUTER SETTINGS".into(),|a|format!("ROUTER: {}",a.ip()));
                self.cv.text(&router,left,y+58.0*s,sm,DIM,Align::Left);
                self.cv.text(server.internet_status(),left,y+73.0*s,sm,AMBER,Align::Left);
                self.cv.paragraph(&format!("1. If automatic setup fails, open Port Forwarding / Virtual Server in your router. Forward external TCP port {} to {}, internal port {}. No UDP forwarding is needed.",server.lan.port(),server.lan.ip(),server.lan.port()),left,y+95.0*s,width,sm,1.4,CREAM);
                self.cv.paragraph("2. Save/apply the router rule. Allow Viper Run incoming connections on the Windows private network if asked. The guest does not open ports.",left,y+134.0*s,width,sm,1.4,CREAM);
                self.cv.paragraph("3. Enter the router's WAN / Internet public IPv4 or your DNS name below. Include :port only if you chose a different external port. Copy the generated invitation and send it to your friend.",left,y+166.0*s,width,sm,1.4,CREAM);
                self.network_field("PUBLIC IPV4 / DNS (OPTIONAL :EXTERNAL PORT)",&mp.menu.public_address,Field::PublicAddress,mp.menu.editing==Some(Field::PublicAddress),true,left,y+201.0*s,width);
                let bx=left+self.button("COPY INTERNET INVITE","",left,y+232.0*s,true,Action::NetCopyManual)+8.0*s;
                self.button("COPY ROUTER STEPS","",bx,y+232.0*s,false,Action::NetCopySetup);
                self.cv.paragraph("If the WAN address is private or starts with 100.64 through 100.127, your connection may use double NAT / CGNAT. This router rule alone will not work. Try the other PC as host, ask your provider for a public IPv4, or use an external relay.",left,y+258.0*s,width,sm,1.4,DIM);
                let max=(width/(6.0*sm))as usize;
                self.cv.text(&mp.menu.status.chars().take(max).collect::<String>(),left,y+299.0*s,sm,AMBER,Align::Left);
                self.button("BACK TO LOBBY","ESC",left,y+313.0*s,false,Action::NetHelpClose);
                return;
            }
            let s=self.s;let sm=(s-1.0).max(1.0);let pw=(self.w-20.0*s).min(610.0*s);let ph=300.0*s;
            let(x,y)=((self.w-pw)/2.0,(self.h-ph)/2.0);let left=x+12.0*s;let width=pw-24.0*s;
            self.frame(x,y,pw,ph);
            self.cv.text("CONNECT WITH A FRIEND",left,y+16.0*s,2.0*s,GREEN,Align::Left);
            for(i,(label,text))in[
                ("SAME NETWORK","One pilot creates a public game. The other opens NEARBY and selects JOIN. If discovery is blocked, use COPY LAN INVITE and PASTE AND JOIN. Allow the host game on your private network when Windows asks."),
                ("DIFFERENT NETWORKS","The host automatically asks the router for a temporary Internet connection. When available, use COPY INTERNET INVITE. The host keeps the game open; the guest needs no server setup."),
                ("IF INTERNET SETUP FAILS","Try the other PC as host. Some routers and Internet providers (CGNAT) block incoming connections. An accessible external relay can be selected under CREATE GAME, CHANGE; ADVANCED browses its rooms."),
                ("READY TO FLY","After connecting, the guest selects READY. The host selects LAUNCH. Microphones start off; select a voice mode in the lobby to talk."),
            ].iter().enumerate() {
                let yy=y+(52.0+i as f32*54.0)*s;
                self.cv.text(label,left,yy,sm,ACID,Align::Left);
                self.cv.paragraph(text,left,yy+12.0*s,width,sm,1.4,CREAM);
            }
            self.button("BACK TO GAME","ESC",left,y+276.0*s,true,Action::NetHelpClose);
            return;
        }
        if mp.session.is_none() { self.server_browser(mp,jet); return; }
        let s=self.s;let sm=(s-1.0).max(1.0);
        let(pw,ph)=((self.w-20.0*s).min(610.0*s),(self.h-30.0*s).min(320.0*s));
        let(x,y)=((self.w-pw)/2.0,(self.h-ph)/2.0);let cw=(pw-38.0*s)*0.49;let rx=x+cw+24.0*s;
        self.frame(x,y,pw,ph);
        self.cv.text("MULTIPLAYER",x+12.0*s,y+16.0*s,2.0*s,GREEN,Align::Left);
        self.cv.text("ROOM LOBBY. READY UP, THEN THE HOST LAUNCHES.",x+12.0*s,y+34.0*s,sm,DIM,Align::Left);
        let menu=&mp.menu;
        let session=mp.session.as_ref().unwrap();
        self.cv.text(if session.host{"YOUR ROOM"}else{"JOINED ROOM"},x+12.0*s,y+53.0*s,s,ACID,Align::Left);
        self.network_field("INVITATION", &mp.invitation(false).unwrap_or_default(),Field::Invite,false,false,x+12.0*s,y+67.0*s,cw);
        let ready=session.members.iter().filter(|m|m.ready).count();
        self.cv.text(&format!("PILOTS {} / {}, READY {}",session.members.len(),session.rules.capacity,ready),x+12.0*s,y+103.0*s,sm,CREAM,Align::Left);
        self.cv.paragraph(if let Some(server)=&mp.server{server.internet_status()}else if session.host{"Wait for at least two pilots to be ready, then launch."}else{"Select READY when you want to fly. The host launches."},x+12.0*s,y+118.0*s,cw,sm,1.4,DIM);
        if let Some(server)=&mp.server {
            let bx=x+12.0*s+self.button("COPY LAN INVITE","",x+12.0*s,y+149.0*s,false,Action::NetCopy(false))+5.0*s;
            if server.internet_address().is_some() {self.button("COPY INTERNET INVITE","",bx,y+149.0*s,false,Action::NetCopy(true));}
            else {self.button(if server.checking_internet(){"CONNECTING..."}else{"INTERNET HELP"},"",bx,y+149.0*s,false,Action::NetHelp);}
        } else {self.button("COPY INVITE","",x+12.0*s,y+149.0*s,false,Action::NetCopy(false));}
        self.cv.text(session.rules.mode.name(),x+12.0*s,y+174.0*s,s,AMBER,Align::Left);
        if mp.server.is_some() {self.button("CONNECTION HELP / PORTS","",x+12.0*s,y+188.0*s,false,Action::NetHelp);}
        else {self.cv.paragraph(&session.status,x+12.0*s,y+190.0*s,cw,sm,1.2,CREAM);}
        self.cv.text(&format!("AIRFRAME: {}",jet.name()),rx,y+49.0*s,sm,AMBER,Align::Left);
        if !mp.active(){self.button("CHANGE JET","",rx,y+57.0*s,false,Action::Jet);}
        else {self.cv.text("AIRFRAME FIXED FOR THIS SORTIE",rx,y+65.0*s,sm,DIM,Align::Left);}
        if let Some(session)=&mp.session{
            self.cv.text("PILOTS / VOICE",rx,y+82.0*s,sm,DIM,Align::Left);
            for(i,m)in session.members.iter().enumerate(){
                let yy=y+(97.0+i as f32*13.0)*s;
                let color=if (m.id==session.id&&mp.voice.transmitting())||mp.voice.peer_talking(m.id){ACID}else{CREAM};
                let score=mp.battle.as_ref().and_then(|b|b.scores.iter().find(|r|r.0==m.id));
                let name=m.name.chars().take(10).collect::<String>();
                let text=if let Some((_,_,kills,deaths))=score{format!("{name} K{kills}/D{deaths}")}else{format!("{} {} {}",if m.ready{"READY"}else{"WAIT"},name,["F16","M29","F18","S27","F18E","E18G"][m.jet as usize])};
                self.cv.text(&text,rx,yy,sm,color,Align::Left);
                if m.id!=session.id{
                    let bx=x+pw-103.0*s;
                    self.key_cap(if menu.muted.contains(&m.id){"UNMUTE"}else{"MUTE"},bx,yy-4.0*sm,50.0*s,false,Action::VoicePeer(m.id));
                    self.key_cap(&format!("{:.0}%",menu.gains.get(&m.id).copied().unwrap_or(1.0)*100.0),bx+54.0*s,yy-4.0*sm,38.0*s,false,Action::VoicePeerGain(m.id));
                }
            }
        }
        let vs=mp.settings();let voice_label=match vs.mode{crate::voice::VoiceMode::Disabled=>"MIC OFF",crate::voice::VoiceMode::PushToTalk=>"PUSH TO TALK",crate::voice::VoiceMode::VoiceActivation=>"VOICE DETECTION"};
        let mut bx=x+12.0*s;let vy=y+213.0*s;
        for(label,act)in[(voice_label,Action::VoiceMode),(if vs.muted{"UNMUTE MIC"}else{"MUTE MIC"},Action::VoiceMute),(if vs.deafened{"UNDEAFEN"}else{"DEAFEN"},Action::VoiceDeafen)]{
            bx+=self.button(label,"",bx,vy,false,act)+5.0*s;
        }
        for(i,(label,value,down,up))in[
            ("MIC GAIN",format!("{:.0}%",vs.input_gain*100.0),Action::VoiceInput(false),Action::VoiceInput(true)),
            ("VOICE VOLUME",format!("{:.0}%",vs.output_gain*100.0),Action::VoiceOutput(false),Action::VoiceOutput(true)),
            ("DETECT RMS",format!("{:.3}",vs.activation_threshold),Action::VoiceThreshold(false),Action::VoiceThreshold(true)),
        ].into_iter().enumerate(){
            let xx=x+12.0*s+i as f32*(pw-24.0*s)/3.0;
            self.cv.text(&format!("{label}: {value}"),xx,y+241.0*s,sm,DIM,Align::Left);
            self.key_cap("-",xx,y+247.0*s,25.0*s,false,down);self.key_cap("+",xx+30.0*s,y+247.0*s,25.0*s,false,up);
        }
        let talk=keys.get(crate::keymap::Bind::PushToTalk).iter().flatten().map(|&k|crate::keymap::key_name(k)).collect::<Vec<_>>().join("/");
        self.cv.text(&format!("MIC STARTS OFF. SELECT A MODE TO ENABLE. TALK: {} / PAD VIEW.",if talk.is_empty(){"UNBOUND"}else{&talk}),x+12.0*s,y+270.0*s,sm,DIM,Align::Left);
        self.cv.text(&format!("MIC LEVEL {:.3}",mp.voice.input_level()),x+pw-145.0*s,y+217.0*s,sm,DIM,Align::Left);
        self.cv.rect(x+pw-145.0*s,y+226.0*s,125.0*s,3.0*s,LINE);
        self.cv.rect(x+pw-145.0*s,y+226.0*s,125.0*s*(mp.voice.input_level()*8.0).min(1.0),3.0*s,ACID);
        let mut bx=x+12.0*s;let by=y+283.0*s;
        if let Some(session)=&mp.session{
            if session.rules.started{bx+=self.button("RESUME","",bx,by,true,Action::NetResume)+5.0*s;}
            else{bx+=self.button("READY / WAIT","",bx,by,true,Action::NetReady)+5.0*s;
                if session.host{bx+=self.button("LAUNCH","",bx,by,false,Action::NetLaunch)+5.0*s;}}
            bx+=self.button("LEAVE ROOM","",bx,by,false,Action::NetLeave)+5.0*s;
        }
        self.button("MAIN MENU",if mp.active(){""}else{"ESC"},bx,by,false,Action::Menu);
        let status=mp.voice.capture_error().unwrap_or(&menu.status);
        let max=((pw-24.0*s)/(6.0*sm))as usize;
        self.cv.text(&status.chars().take(max).collect::<String>(),x+12.0*s,y+310.0*s,sm,AMBER,Align::Left);
        self.legend(if mp.active(){&[("ESC","RESUME")]}else{&[("ESC","BACK")]});
    }

    fn status_panel(&mut self, kicker: &str, kicker_col: Col, title: &str, title_col: Col, sub: &str, stats: [(&str, String, Col); 4], actions: &[(&str, &str, Action)], small: &str) {
        let s = self.s;
        let sm = s.max(2.0) - 1.0;
        let pw=(self.w-24.0*s).min(380.0*s);
        let content_w=pw-28.0*s;
        let sub_h=Canvas::paragraph_height(sub,content_w,sm,1.7);
        let note_h=Canvas::paragraph_height(small,content_w,sm,1.4);
        let mut rows=1;let mut row_w=0.0;
        for (label,hint,_) in actions {let w=self.button_width(label,hint);if row_w>0.0&&row_w+w>content_w {rows+=1;row_w=0.0;}row_w+=w+6.0*s;}
        let needed=135.0*s+sub_h+note_h+(rows-1)as f32*19.0*s;
        let ph=needed.max(186.0*s).min(self.h-24.0*s);
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
            self.cv.layout_item(crate::menu_layout::Kind::Panel,k,[cx,yy,cw,30.0*s]);
            self.cv.rect(cx, yy, cw, 30.0 * s, rgba(0x05, 0x0b, 0x08, 0.85));
            self.cv.stroke_rect(cx, yy, cw, 30.0 * s, 1.0, LINE);
            self.cv.text(k, cx + 5.0 * s, yy + 8.0 * s, sm, DIM, Align::Left);
            self.cv.text(v, cx + 5.0 * s, yy + 20.0 * s, s, *c, Align::Left);
        }
        yy += 40.0 * s;
        let items:Vec<_>=actions.iter().enumerate().map(|(i,&(label,hint,action))|(label,hint,action,i==0)).collect();
        let end=self.button_row(&items,x+m,yy,pw-2.0*m);
        self.cv.paragraph(small,x+m,(end+8.0*s).max(y+ph-12.0*s-note_h),pw-2.0*m,sm,1.4,DIM);
        let keys: Vec<(&str, &str)> = actions.iter().map(|&(label, hint, _)| (hint, label)).collect();
        self.legend(&keys);
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
                    &[("FLY AGAIN", "R", Action::Restart), ("MAIN MENU", "ESC", Action::Menu)],
                    &best_line(best),
                );
            }
            Outcome::Crash | Outcome::ShotDown => self.status_panel(
                "// FLIGHT TERMINATED, AIRFRAME LOST",
                RED,
                "TERRAIN WINS",
                RED,
                &format!("The gorge is unforgiving. The flight recorder shows {} gates cleared before impact.", pad(r.gates as f64, 2)),
                [("GATE", format!("{} / 10", pad(r.gates as f64, 2)), RED), ("ELAPSED", format_time(r.time), CREAM), ("IMPACT", format!("{} KT", pad(r.impact * 1.94384, 3)), RED), ("BEST", best_str, CREAM)],
                &[("REARM & RETRY", "R", Action::Restart), ("MAIN MENU", "ESC", Action::Menu)],
                "TIP: EASE OFF THE PULL AT HIGH SPEED. LET THE GORGE OPEN BEFORE YOU ROLL IN.",
            ),
            Outcome::Timeout => self.status_panel(
                "// CLOCK EXPIRED, RUN INVALID",
                AMBER,
                "RUN EXPIRED",
                AMBER,
                &format!("The coast is still out there. You cleared {} gates before the clock ran dry.", pad(r.gates as f64, 2)),
                [("GATE", format!("{} / 10", pad(r.gates as f64, 2)), AMBER), ("ELAPSED", format_time(r.time), CREAM), ("SCORE", pad(r.score, 5), CREAM), ("BEST", best_str, CREAM)],
                &[("TRY AGAIN", "R", Action::Restart), ("MAIN MENU", "ESC", Action::Menu)],
                "THE CLOCK IS A HARD CEILING. GATES ADD 8 SECONDS EACH.",
            ),
        }
    }

    /// Setup before every sortie: the airframe, the stores on the pylon
    /// pairs (missions with something to shoot), the fuel and the clock, and
    /// what that does to the weight, drag and thrust-to-weight.
    pub fn loadout_screen(&mut self, mission: MissionKind, cfg: &Config, clock: bool) {
        let s = self.s;
        let sm = s.max(2.0) - 1.0;
        // The fighting missions add the threat setup and its map on the right.
        let threats = matches!(mission, MissionKind::Dogfight | MissionKind::Strike | MissionKind::Free);
        let armed = threats;
        let store_w = Store::ALL.iter().map(|st| Canvas::text_width(st.pair(), s) + 16.0 * s).fold(0.0, f32::max);
        let left = if threats { (30.0 * s + 120.0 * sm + store_w).max(280.0 * s) } else { 300.0 * s };
        // Without the stores and the threat map the panel fits its few rows.
        let tall = if armed { 280.0 } else { 172.0 };
        let (pw, ph) = ((self.w - 24.0 * s).min(if threats { 800.0 * s } else { left }), (self.h - 26.0 * s).min(tall * s));
        let (x, y) = ((self.w - pw) / 2.0, (self.h - ph) / 2.0 - 4.0 * s);
        self.frame(x, y, pw, ph);
        let m = 14.0 * s;
        let mut yy = y + m;
        self.cv.text(&format!("// {}, {}", cfg.jet.name(), mission.name()), x + m, yy, sm, DIM, Align::Left);
        yy += 20.0 * s;
        self.cv.text(if armed { "LOADOUT" } else { "SETUP" }, x + m, yy, s * 3.0, GREEN, Align::Left);
        yy += 18.0 * s;
        let name = |st: Option<Store>| st.map_or("EMPTY", |s| s.pair());
        let mig = cfg.jet.russian();
        let gun = crate::weapons::gun_of(cfg.jet);
        let (tip, out, inb, ammo) = if mig { ("PYLON 1 / 6  OUTER", "PYLON 2 / 5  MIDDLE", "PYLON 3 / 4  INNER", "OFZ 30 MM") } else { ("STA 1 / 9  WINGTIP", "STA 3 / 7", "STA 4 / 6", "PGU-28") };
        // The MiG's gauges read kilograms, the F-16's pounds.
        let (unit, per_kg) = if mig { ("KG", 1.0) } else { ("LB", 2.20462) };
        let fuel_kg = cfg.jet.fuel_max() * cfg.fuel;
        let fuel = if cfg.fuel_is_min() { format!("{} {unit}  MINIMUM", (fuel_kg * per_kg).round() as i64) } else { format!("{} {unit}  {}%", (fuel_kg * per_kg).round() as i64, (cfg.fuel * 100.0).round() as i64) };
        let mut rows: Vec<(&str, String, Option<Action>)> = vec![("AIRFRAME", cfg.jet.name().to_string(), Some(Action::Jet))];
        if armed {
            rows.push((tip, cfg.tip.pair().to_string(), Some(Action::Station(0))));
            // The MiG has no stations 2 and 8.
            if crate::combat::has_pair(cfg.jet, 1) {
                rows.push(("STA 2 / 8", name(cfg.outer).to_string(), Some(Action::Station(1))));
            }
            rows.push((out, name(cfg.outboard).to_string(), Some(Action::Station(2))));
            rows.push((inb, name(cfg.inboard).to_string(), Some(Action::Station(3))));
        }
        rows.push(("INTERNAL FUEL", fuel, Some(Action::Fuel)));
        if armed {
            rows.push((gun.name, format!("{} ROUNDS {ammo}", gun.rounds), None));
            rows.push((crate::weapons::dispenser_of(cfg.jet), format!("{} CHAFF  {} FLARES", crate::weapons::CHAFF, crate::weapons::FLARES), None));
        }
        // Free flight never has a clock; instead its weapons can refill.
        if mission != MissionKind::Free {
            rows.push(("MISSION CLOCK", if clock { format!("ON, {}", format_time(mission.time())) } else { "OFF".to_string() }, Some(Action::Clock)));
        } else {
            rows.push(("AUTO REFILL", if cfg.refill { "ON" } else { "OFF" }.to_string(), Some(Action::Refill)));
        }
        // LAUNCH comes first in the keyboard order, so it is highlighted
        // first and Enter takes off.
        let launch_y = yy + rows.len() as f32 * 17.0 * s + 14.0 * s;
        let w1 = self.button("LAUNCH", "SPACE", x + m, launch_y, true, Action::Launch);
        self.button("MAIN MENU", "ESC", x + m + w1 + 6.0 * s, launch_y, false, Action::Menu);
        let col2 = x + m + 120.0 * sm;
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
        // Full afterburner against the take-off weight.
        let tw = cfg.jet.max_thrust_lbf() / (mass * 2.20462);
        let stats = format!("GROSS {} {unit}   DRAG +{} COUNTS   T/W {:.2}", (mass * per_kg / 10.0).round() as i64 * 10, (drag * 10000.0).round() as i64, tw);
        self.cv.text(&stats, x + m, yy + 4.0 * s, sm, AMBER, Align::Left);
        let note = if armed { "HEAVIER AND DRAGGIER MEANS SLOWER AND LESS G." } else { "LESS FUEL IS LIGHTER AND QUICKER, AND SOONER EMPTY." };
        let nh=Canvas::paragraph_height(note,pw-2.0*m,sm,1.4);
        self.cv.paragraph(note,x+m,y+ph-8.0*s-nh,pw-2.0*m,sm,1.4,DIM);
        // After LAUNCH and the stores in the keyboard order.
        if threats && pw > left + 200.0 * s {
            self.threat_setup(x + left, y + m, pw - left - m, ph - 2.0 * m, mission, cfg);
        }
        self.legend(&[("SPACE", "LAUNCH"), ("ESC", "MAIN MENU")]);
    }

    /// Threat setup beside the loadout: the prepared layout, the number and
    /// type of bandits, the SA-6 batteries and ZSU-23-4s, and a tactical map
    /// of what that puts in the valley.
    fn threat_setup(&mut self, x: f32, y: f32, w: f32, h: f32, mission: MissionKind, cfg: &Config) {
        let s = self.s;
        let sm = s.max(2.0) - 1.0;
        let t = &cfg.threats;
        let mut yy = y;
        self.cv.text("// OPPOSITION", x, yy, sm, DIM, Align::Left);
        yy += 20.0 * s;
        self.cv.text("THREATS", x, yy, s * 3.0, RED, Align::Left);
        yy += 14.0 * s;
        let col2 = x + 64.0 * sm;
        let rows: [(&str, String, Action); 5] = [
            ("LAYOUT", t.layout.name().to_string(), Action::Layout),
            ("BANDITS", if t.bandits == 0 { "NONE".to_string() } else { t.bandits.to_string() }, Action::Bandits),
            ("TYPE", cfg.fleet_name().to_string(), Action::Aggressor),
            ("SA-6 SITES", if t.sams == 0 { "NONE".to_string() } else { t.sams.to_string() }, Action::Sams),
            ("ZSU-23-4", if t.guns == 0 { "NONE".to_string() } else { t.guns.to_string() }, Action::Guns),
        ];
        let rows_top = yy;
        // The map starts past the widest setting, the mixed fleet, so long
        // names never run under it and the map holds still as they change.
        let widest = rows.iter().map(|(_, v, _)| v.as_str()).chain(["F-16C + MIG-29S"]).map(|v| Canvas::text_width(v, s) + 16.0 * s).fold(90.0 * s, f32::max);
        for (label, value, act) in rows {
            self.cv.text(label, x, yy + 7.0 * s, sm, DIM, Align::Left);
            self.button(&value, "", col2, yy, false, act);
            yy += 17.0 * s;
        }
        let brief_w = col2 - x + widest;
        yy += 4.0 * s;
        self.cv.paragraph(t.layout.brief(), x, yy, brief_w, sm, 1.6, rgba(0xb3, 0xc3, 0xb7, 1.0));
        // The map takes the rest of the column, right of the setup rows.
        let mx = x + brief_w + 8.0 * s;
        let mw = (x + w - mx).max(40.0 * s);
        self.tactical_map(mx, rows_top - 34.0 * s, mw, h - (rows_top - 34.0 * s - y), mission, cfg);
    }

    /// Tactical map of the Cold River valley, north up: the ground shaded by
    /// height, the river, the gate route and the runway, the jet's start,
    /// and the opposition the setup deploys, with the SA-6 and ZSU-23-4
    /// engagement rings and the patrol circuits.
    fn tactical_map(&mut self, x: f32, y: f32, w: f32, h: f32, mission: MissionKind, cfg: &Config) {
        let s = self.s;
        let sm = s.max(2.0) - 1.0;
        let m = Mission::new();
        let d = scenario::deploy(mission, cfg.aggressor, &cfg.threats, &m);
        // A window from past the farthest bandits (north, -z, at the top) to
        // just behind the start.
        let (z0, z1) = (m.start_pos.z - 40000.0, m.start_pos.z + 3000.0);
        let span_z = z1 - z0;
        let span_x = span_z * (w / h) as f64;
        let cx = terrain::river_x((z0 + z1) * 0.5);
        let to = |p: V3| -> [f32; 2] { [x + (((p.x - cx) / span_x + 0.5) as f32) * w, y + (((p.z - z0) / span_z) as f32) * h] };
        let px = |metres: f64| (metres / span_z) as f32 * h;
        self.cv.layout_item(crate::menu_layout::Kind::Panel,"tactical map",[x,y,w,h]);
        self.cv.set_clip(Some(vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]]));
        // Ground: cells shaded by height, lit from the north-west.
        let (nx, nz) = (((w / h) * 72.0).round().max(8.0) as usize, 72usize);
        let (cw, ch) = (w / nx as f32, h / nz as f32);
        for j in 0..nz {
            for i in 0..nx {
                let wx = cx + ((i as f64 + 0.5) / nx as f64 - 0.5) * span_x;
                let wz = z0 + (j as f64 + 0.5) / nz as f64 * span_z;
                let e = terrain::floor(wx, wz);
                let slope = e - terrain::floor(wx - 250.0, wz - 250.0);
                let t = clamp((e - 500.0) / 2000.0, 0.0, 1.0) as f32;
                let lit = clamp(1.0 + slope / 900.0, 0.85, 1.15) as f32;
                let (r, g, b) = if t < 0.5 { (0.16 + 0.5 * t, 0.24 + 0.24 * t, 0.14 + 0.22 * t) } else { (0.41 + 0.5 * (t - 0.5), 0.36 + 0.56 * (t - 0.5), 0.25 + 0.7 * (t - 0.5)) };
                let c = |v: f32| ((v * lit).min(1.0) * 255.0) as u8;
                self.cv.rect(x + i as f32 * cw, y + j as f32 * ch, cw + 0.5, ch + 0.5, rgba(c(r), c(g), c(b), 1.0));
            }
        }
        // River, route and runway.
        let river: Vec<[f32; 2]> = (0..=60).map(|k| to(scenario::river(z0 + k as f64 / 60.0 * span_z))).collect();
        self.cv.polyline(&river, 2.0, false, rgba(0x5a, 0x9c, 0xd8, 0.9));
        let route: Vec<[f32; 2]> = m.gates.iter().map(|g| to(g.pos)).collect();
        self.cv.polyline(&route, 1.0, false, with_alpha(GREEN, 0.35));
        let (ax, az) = terrain::runway_point(-terrain::RUNWAY_HALF_LENGTH, 0.0);
        let (bx, bz) = terrain::runway_point(terrain::RUNWAY_HALF_LENGTH, 0.0);
        let (ra, rb) = (to(v3(ax, 0.0, az)), to(v3(bx, 0.0, bz)));
        self.cv.line(ra[0], ra[1], rb[0], rb[1], 3.0, rgba(0x2a, 0x2c, 0x2e, 1.0));
        // Engagement rings first, under the markers.
        for site in &d.sites {
            let p = to(site.pos);
            match site.kind {
                targets::Kind::Launcher => self.cv.circle(p[0], p[1], px(scenario::SAM_RANGE), 1.0, with_alpha(RED, 0.55)),
                targets::Kind::Zsu => self.cv.circle(p[0], p[1], px(scenario::GUN_RANGE).max(3.0), 1.0, with_alpha(AMBER, 0.6)),
                _ => {}
            }
        }
        let mut sam_labels: Vec<(f32, f32)> = Vec::new();
        for site in &d.sites {
            let p = to(site.pos);
            match site.kind {
                targets::Kind::Launcher => {
                    self.cv.tri([p[0], p[1] - 4.0 * s], [p[0] - 3.5 * s, p[1] + 3.0 * s], [p[0] + 3.5 * s, p[1] + 3.0 * s], RED);
                    let lx = (p[0] + 5.0 * s).min(x + w - Canvas::text_width("SA-6", sm) - 2.0 * s).max(x + 2.0 * s);
                    let ly = p[1].clamp(y + 5.0 * s, y + h - 5.0 * s);
                    if !sam_labels.iter().any(|&(sx, sy)| (sx-lx).abs() < 32.0 * sm && (sy-ly).abs() < 10.0 * sm) {
                        self.cv.text("SA-6", lx, ly, sm, RED, Align::Left);
                        sam_labels.push((lx, ly));
                    }
                }
                targets::Kind::Zsu => self.cv.disc(p[0], p[1], 2.0 * s, AMBER),
                targets::Kind::Radar => {}
                _ => self.cv.rect(p[0] - 1.5 * s, p[1] - 1.5 * s, 3.0 * s, 3.0 * s, CREAM),
            }
        }
        // Bandits: on patrol, their circuit; otherwise where they start and
        // the way they head.
        for b in &d.bandits {
            let letter = if b.kind == crate::sim::Type::Mig29 { "M" } else { "F" };
            let p = match b.patrol {
                Some((radius, _, _, at)) => {
                    let c = to(scenario::river(scenario::PATROL_CENTRE_Z));
                    self.cv.circle(c[0], c[1], px(radius), 1.0, with_alpha(RED, 0.4));
                    let ang = at as f64 * TAU / 8.0;
                    to(scenario::river(scenario::PATROL_CENTRE_Z) + v3(ang.sin() * radius, 0.0, -ang.cos() * radius))
                }
                None => {
                    let p = to(b.pos);
                    let (dx, dz) = (b.heading.sin(), -b.heading.cos());
                    self.cv.line(p[0], p[1], p[0] + (dx * 9.0) as f32 * s, p[1] + (dz * 9.0) as f32 * s, 1.0, RED);
                    p
                }
            };
            let r = 3.5 * s;
            self.cv.quad([[p[0], p[1] - r], [p[0] + r, p[1]], [p[0], p[1] + r], [p[0] - r, p[1]]], RED);
            self.cv.text(letter, p[0], p[1] - r - 5.0 * s, sm, RED, Align::Center);
        }
        // The jet's start, pointing down the route.
        let p = to(m.start_pos);
        let f = v3(m.start_vel.x, 0.0, m.start_vel.z).norm();
        let (fx, fz) = (f.x as f32, f.z as f32);
        let r = 5.0 * s;
        self.cv.tri([p[0] + fx * r, p[1] + fz * r], [p[0] - fx * r * 0.6 - fz * r * 0.6, p[1] - fz * r * 0.6 + fx * r * 0.6], [p[0] - fx * r * 0.6 + fz * r * 0.6, p[1] - fz * r * 0.6 - fx * r * 0.6], GREEN);
        self.cv.set_clip(None);
        self.cv.stroke_rect(x, y, w, h, 1.0, LINE_HOT);
        self.cv.text("N", x + w - 6.0 * s, y + 7.0 * s, sm, CREAM, Align::Center);
        let scale = px(10000.0);
        self.cv.line(x + 6.0 * s, y + h - 6.0 * s, x + 6.0 * s + scale, y + h - 6.0 * s, 1.0, CREAM);
        self.cv.text("10 KM", x + 6.0 * s, y + h - 12.0 * s, sm, CREAM, Align::Left);
    }

    /// Result of a DOGFIGHT or STRIKE sortie.
    fn combat_result(&mut self, r: &RunResult) {
        if r.mission == MissionKind::Airfield && r.kind == Outcome::Win {
            let again: &[(&str, &str, Action)] = &[("FLY AGAIN", "R", Action::Restart), ("MAIN MENU", "ESC", Action::Menu)];
            self.status_panel(
                "// FULL STOP, RUNWAY A",
                DIM,
                "ON THE GROUND",
                ACID,
                "Wheels up, out to the steerpoint and back down on the same runway. The tower has logged your landing.",
                [("TIME", format_time(r.time), GREEN), ("LEGS", format!("{} / {}", r.done, r.total), CREAM), ("SCORE", pad(r.score, 5), AMBER), ("RATING", if r.score > 4200.0 { "ACE" } else if r.score > 3500.0 { "VIPER" } else { "QUALIFIED" }.to_string(), CREAM)],
                again,
                "",
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
                &[("FLY AGAIN", "R", Action::Restart), ("MAIN MENU", "ESC", Action::Menu)],
                "",
            );
            return;
        }
        let fight = r.mission == MissionKind::Dogfight;
        let (what, unit) = if fight { ("KILLS", "bandits") } else { ("TARGETS", "primary targets") };
        let tally = format!("{} / {}", r.done, r.total);
        let again: &[(&str, &str, Action)] = &[("FLY AGAIN", "R", Action::Restart), ("MAIN MENU", "ESC", Action::Menu)];
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
                    ("// SPLASH ALL BANDITS, COLD RIVER", "SKY CLEARED", format!("Every aggressor is down and you are still flying. The evaluator has logged the fight as {rank}."))
                } else {
                    ("// TARGETS DESTROYED, COLD RIVER", "STRIKE COMPLETE", format!("The bridge is in the river and the SA-6 battery is burning. The evaluator has logged the strike as {rank}."))
                };
                self.status_panel(kicker, DIM, title, ACID, &text, [("TIME", format_time(r.time), GREEN), (what, tally, CREAM), ("SCORE", pad(r.score, 5), AMBER), ("RATING", rank.to_string(), CREAM)], again, "");
            }
            Outcome::ShotDown => self.status_panel(
                &if r.cause == "PILOT KILLED" { "// PILOT KILLED IN THE COCKPIT".to_string() } else if r.cause == "EJECTED" { "// PILOT EJECTED, JET LOST".to_string() } else { format!("// {}, PILOT EJECTED", if r.cause.is_empty() { "AIRFRAME LOST" } else { r.cause }) },
                RED,
                if r.cause == "PILOT KILLED" { "KILLED" } else if r.cause == "EJECTED" { "EJECTED" } else { "SHOT DOWN" },
                RED,
                &if r.cause == "PILOT KILLED" {
                    format!("The rounds came through the canopy, and the jet flew on without you. You took {} {unit} with you.", r.done)
                } else if r.cause == "EJECTED" {
                    format!("You pulled the handle, the seat fired and the chute let you down. You took {} {unit} with you.", r.done)
                } else {
                    format!("The jet came apart around you and the seat did its job. You took {} {unit} with you.", r.done)
                },
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
    pub fn controls_drawer(&mut self, st: &Settings, paused_flight: bool, waiting: Option<(usize, usize)>, online: bool) {
        let s = self.s;
        let sm = s.max(2.0) - 1.0;
        let w = (200.0 * s).min(self.w * 0.94);
        self.cv.layout_item(crate::menu_layout::Kind::Panel,"controls",[0.0,0.0,w,self.h]);
        self.cv.rect(0.0, 0.0, w, self.h, rgba(0x09, 0x10, 0x12, 0.97));
        self.cv.rect(w - 1.0, 0.0, 1.0, self.h, rgba(0x78, 0x8e, 0x7b, 1.0));
        let m = 10.0 * s;
        let mut y = m + 4.0 * s;
        self.cv.text("CONTROLS", m, y, s, CREAM, Align::Left);
        let cw = self.button_width("CLOSE", "ESC");
        self.button("CLOSE", "ESC", w - m - cw, y - 7.5 * s, false, Action::Controls);
        y += 16.0 * s;
        let heading = match (paused_flight, self.pad) {
            (true, _) => if online{"LOCAL CONTROLS PAUSED"}else{"FLIGHT PAUSED"},
            (false, true) => "XBOX CONTROLLER",
            (false, false) => "FLIGHT CONTROLS",
        };
        self.cv.text(heading, m, y, sm, rgba(0xbe, 0xd5, 0x8f, 1.0), Align::Left);
        y += 12.0 * s;
        let pitch_rows: &[(&str, &str)] = match (self.pad, st.pitch, st.invert_pitch) {
            (true, _, false) => &[("{LS}", "Pitch and roll")],
            (true, _, true) => &[("{LS}", "Pitch (inverted) and roll")],
            (false, PitchSource::Mouse, false) => &[("MOUSE Y", "Pitch")],
            (false, PitchSource::Mouse, true) => &[("MOUSE Y", "Pitch (inverted)")],
            (false, PitchSource::Keys, false) => &[("S / DOWN", "Nose up"), ("W / UP", "Nose down")],
            (false, PitchSource::Keys, true) => &[("W / UP", "Nose up"), ("S / DOWN", "Nose down")],
        };
        let rest: &[(&str, &str)] = match (self.pad, st.pad_layout) {
            (true, PadLayout::Combat) => &[
                ("{RS}", "Look around"),
                ("{RT}", "Fire, hold for gun"),
                ("{RB}", "Lock target, again for next"),
                ("{RB} HOLD", "Drop lock"),
                ("{R3} HOLD", "Zoom"),
                ("{LB}", "Throttle up, afterburner at the stop"),
                ("{LT}", "Throttle down"),
                ("{LT} GROUND", "Wheel brakes"),
                ("{X} + {LS}", "Rudder"),
                ("{L3}", "Chaff and flare"),
                ("{L3} + {R3}", "Limiters off / on"),
                ("{A}", "Next weapon"),
                ("{A} HOLD", "Jettison the selected store"),
                ("{LB} + {RB} HOLD", "Eject"),
                ("{B}", "Airbrake"),
                ("{Y}", "Cockpit / chase camera"),
                ("{DLR}", "Left / right MFD page"),
                ("{DL} MIG-29", "Radar / IRST"),
                ("{DD}", "Landing gear"),
                ("{DU}", "Flaps"),
                ("{START}", "Freeze flight"),
                ("{B} FROZEN", "Release mouse"),
                ("{BACK}", "Controls and settings"),
            ],
            (true, PadLayout::Classic) => &[
                ("{RS}", "Look around"),
                ("{A}", "Fire, hold for gun"),
                ("{DD}", "Lock target, again for next"),
                ("{RT}", "Throttle up, afterburner at the stop"),
                ("{LT}", "Throttle down"),
                ("{LT} GROUND", "Wheel brakes"),
                ("{LB} {RB}", "Rudder left / right"),
                ("{L3}", "Chaff and flare"),
                ("{L3} + {R3}", "Limiters off / on"),
                ("{DU}", "Next weapon"),
                ("{DU} HOLD", "Jettison the selected store"),
                ("{LB} + {RB} HOLD", "Eject"),
                ("{B}", "Airbrake"),
                ("{X}", "Landing gear"),
                ("{R3} HOLD", "Zoom"),
                ("{Y}", "Cockpit / chase camera"),
                ("{DLR}", "Left / right MFD page"),
                ("{DL} MIG-29", "Radar / IRST"),
                ("{START}", "Freeze flight"),
                ("{B} FROZEN", "Release mouse"),
                ("{BACK}", "Controls and settings"),
            ],
            (false, _) => &[("MOUSE X", "Rudder"), ("ALT+MOUSE", "Look around"), ("R-CLICK", "Zoom, hold"), ("M-CLICK", "Center the mouse stick"), ("L-CLICK", "Fire"), ("ALT+O", "Limiters off / on"), ("P", "Freeze flight"), ("ESC", "Configuration, pause flight"), ("M", "Sound on / off"), ("F11", "Fullscreen"), ("F12", "Save screenshot"), ("KEYS", "Change them at right")],
        };
        // The controller map is long: like the key bindings, it gets its own
        // panel beside the drawer, which then holds only the settings.
        let rows: Vec<(&str, &str)> = pitch_rows.iter().chain(rest.iter()).copied().map(|row| {
            if self.pad && st.click_rudder() {
                match row {
                    ("{X} + {LS}", _) => ("{X} / {L3} HOLD", "Rudder with left stick"),
                    ("{L3}", "Chaff and flare") => ("{X} + {L3}", "Chaff and flare"),
                    _ => row,
                }
            } else { row }
        }).collect();
        if self.pad { self.pad_map(&rows, w); }
        y += 6.0 * s;
        let slider = |ui: &mut Ui, y: f32, label: &str, value: String, t: f64, down: Action, up: Action| {
            ui.cv.text(label, m, y, sm, CREAM, Align::Left);
            ui.cv.text(&value, w - m, y, sm, rgba(0xc9, 0xe9, 0xa0, 1.0), Align::Right);
            let by = y + 9.0 * s;
            ui.button("-", "", m, by, false, down);
            let bw = ui.button_width("-", "");
            let (tx, tw) = (m + bw + 4.0 * s, w - 2.0 * m - 2.0 * bw - 8.0 * s);
            ui.cv.rect(tx, by + 7.0 * s, tw, 2.0, LINE_HOT);
            ui.cv.rect(tx + tw * t as f32 - 2.0 * s, by + 3.0 * s, 4.0 * s, 9.0 * s, ACID);
            ui.button("+", "", w - m - bw, by, false, up);
        };
        if self.pad {
            slider(self, y, "STICK RESPONSE (EXPO)", format!("{:.0}%", st.expo * 100.0), st.expo, Action::SensDown, Action::SensUp);
        } else {
            slider(self, y, "MOUSE SENSITIVITY", format!("{:.1}X", st.sensitivity), (st.sensitivity - 0.5) / 4.5, Action::SensDown, Action::SensUp);
        }
        y += 30.0 * s;
        slider(self, y, "COCKPIT FIELD OF VIEW", format!("{}°", st.fov.round()), (st.fov - FOV_MIN) / (FOV_MAX - FOV_MIN), Action::FovDown, Action::FovUp);
        y += 30.0 * s;
        {
            self.button(if st.pitch == PitchSource::Mouse { "PITCH INPUT: MOUSE" } else { "PITCH INPUT: W / S" }, "", m, y, false, Action::Pitch);
            y += 18.0 * s;
        }
        self.button(if st.invert_pitch { "[X] INVERT PITCH" } else { "[ ] INVERT PITCH" }, "", m, y, false, Action::Invert);
        y += 18.0 * s;
        {
            self.button(if st.pad_layout == PadLayout::Combat { "PAD LAYOUT: COMBAT" } else { "PAD LAYOUT: CLASSIC" }, "", m, y, false, Action::PadLayout);
            y += 18.0 * s;
        }
        self.button("CONTROLLER SETUP", "", m, y, false, Action::PadSetup);
        y += 18.0 * s;
        self.button(if st.fullscreen { "[X] FULLSCREEN" } else { "[ ] FULLSCREEN" }, "F11", m, y, false, Action::Fullscreen);
        y += 18.0 * s;
        self.button(if st.performance_overlay { "[X] PERFORMANCE OVERLAY" } else { "[ ] PERFORMANCE OVERLAY" }, "", m, y, false, Action::Performance);
        y += 18.0 * s;

        let note = "Mouse and controller work together. Invert pitch applies to both. Use the input map on the right.";
        y += self.cv.paragraph(note, m, y, w - 2.0 * m, sm, 1.6, rgba(0x91, 0xa1, 0x8f, 1.0)) + 2.0 * s;
        if paused_flight {
            let width=self.button(if online{"LEAVE ROOM"}else{"RESTART"}, "", m, y, false, if online{Action::NetLeave}else{Action::Restart});
            self.button("MAIN MENU", "", m+width+6.0*s, y, false, Action::Menu);
            y += 18.0 * s;
        }
        self.button("RESTORE DEFAULTS", "", m, y, false, Action::Reset);
        y += 15.0 * s + 8.0 * s;
        self.cv.paragraph(if self.pad { "Press {BACK} or {B} to close. Settings are saved for this user." } else { "Press Esc or K to close. Settings are saved for this user." }, m, y, w - 2.0 * m, sm, 1.6, rgba(0x91, 0xa1, 0x8f, 1.0));
        if !self.pad {
            self.key_bindings(&st.keys, w, waiting);
        }
    }

    /// Dedicated setup page, available with either mouse or controller.
    pub fn pad_setup(&mut self, st: &Settings, pad: &crate::gamepad::Gamepad, now: f64) {
        let s = self.s;
        let sm = (s - 1.0).max(1.0);
        let w = (604.0 * s).min(self.w - 16.0 * s);
        let x = (self.w - w) * 0.5;
        let y = 10.0 * s;
        let h = self.h - 20.0 * s;
        self.cv.layout_item(crate::menu_layout::Kind::Panel, "controller setup", [x, y, w, h]);
        self.cv.rect(x, y, w, h, rgba(0x09, 0x10, 0x12, 0.98));
        self.cv.stroke_rect(x, y, w, h, 1.0, LINE);
        let left = x + 12.0 * s;
        let right = x + w * 0.52;
        self.cv.text("CONTROLLER SETUP", left, y + 14.0 * s, s, CREAM, Align::Left);
        self.cv.text(if pad.raw.connected { "CONTROLLER CONNECTED" } else { "CONNECT A CONTROLLER TO CALIBRATE" }, left, y + 30.0 * s, sm, if pad.raw.connected { GREEN } else { AMBER }, Align::Left);
        let slider = |ui: &mut Ui, row: f32, label: &str, value: f64, range: f64, down: Action, up: Action| {
            let width = w * 0.45;
            ui.cv.text(label, left, row, sm, CREAM, Align::Left);
            ui.cv.text(&format!("{:.0}%", value * 100.0), left + width, row, sm, GREEN, Align::Right);
            ui.button("-", "", left, row + 7.0 * s, false, down);
            let bw = ui.button_width("+", "");
            let bx = left + bw + 5.0 * s;
            let bar = width - 2.0 * bw - 10.0 * s;
            ui.cv.rect(bx, row + 14.0 * s, bar, 1.0, LINE_HOT);
            ui.cv.rect(bx + bar * (value / range) as f32 - s, row + 10.0 * s, 2.0 * s, 9.0 * s, ACID);
            ui.button("+", "", left + width - bw, row + 7.0 * s, false, up);
        };
        if !pad.calibrating() {
            slider(self, y + 49.0 * s, "LEFT DEAD ZONE", st.pad_dead_left, crate::settings::DEAD_ZONE_MAX, Action::PadDeadLeft(false), Action::PadDeadLeft(true));
            slider(self, y + 80.0 * s, "RIGHT DEAD ZONE", st.pad_dead_right, crate::settings::DEAD_ZONE_MAX, Action::PadDeadRight(false), Action::PadDeadRight(true));
            slider(self, y + 111.0 * s, "CENTER SOFTNESS (EXPO)", st.expo, 1.0, Action::PadExpo(false), Action::PadExpo(true));
            self.button(if st.pad_fine_aim { "[X] FINE AIM WITH ZOOM" } else { "[ ] FINE AIM WITH ZOOM" }, "", left, y + 140.0 * s, false, Action::PadFineAim);
            self.button(if st.pad_rudder_click { "[X] X OR L3 FOR RUDDER" } else { "[ ] X OR L3 FOR RUDDER" }, "", left, y + 162.0 * s, false, Action::PadRudderClick);
        } else {
            self.cv.paragraph("LET GO OF BOTH STICKS. Calibration measures each stick separately. Keep the controller still until it finishes.", left, y + 65.0 * s, w * 0.45, sm, 1.6, AMBER);
        }
        self.cv.paragraph(if st.pad_layout == PadLayout::Classic {
            "L3 rudder applies to COMBAT layout. CLASSIC keeps LB / RB rudder and L3 countermeasures."
        } else if st.click_rudder() {
            "Hold X or click and hold L3, then move left / right for rudder. X + L3 releases chaff / flare."
        } else {
            "Hold X and move left / right for rudder. Enable L3 above to use either button. L3 releases chaff / flare."
        }, left, y + 190.0 * s, w * 0.45, sm, 1.4, DIM);

        self.cv.text("LIVE INPUT: LEFT / RIGHT", right, y + 49.0 * s, sm, CREAM, Align::Left);
        for (i, (raw, filtered, dead)) in [(pad.raw.left, st.flight_stick(pad.state.left, 0.0), st.pad_dead_left), (pad.raw.right, (crate::settings::stick_expo(pad.state.right.0, 0.5), crate::settings::stick_expo(pad.state.right.1, 0.5)), st.pad_dead_right)].into_iter().enumerate() {
            let (cx, cy, r) = (right + (46.0 + i as f32 * 124.0) * s, y + 102.0 * s, 34.0 * s);
            self.cv.circle(cx, cy, r, 1.0, LINE_HOT);
            self.cv.circle(cx, cy, r * dead as f32, 1.0, AMBER);
            self.cv.line(cx - r, cy, cx + r, cy, 1.0, LINE);
            self.cv.line(cx, cy - r, cx, cy + r, 1.0, LINE);
            self.cv.rect(cx + raw.0 as f32 * r - s, cy - raw.1 as f32 * r - s, 2.0 * s, 2.0 * s, AMBER);
            self.cv.circle(cx + filtered.0 as f32 * r, cy - filtered.1 as f32 * r, 2.0 * s, s, GREEN);
        }
        self.cv.text("AMBER: RAW / DEAD ZONE", right, y + 146.0 * s, sm, AMBER, Align::Left);
        self.cv.text("GREEN: FLIGHT / LOOK OUTPUT", right, y + 159.0 * s, sm, GREEN, Align::Left);
        self.cv.text(&format!("RAW DRIFT  L {:.1}%  R {:.1}%", pad.raw.left.0.hypot(pad.raw.left.1) * 100.0, pad.raw.right.0.hypot(pad.raw.right.1) * 100.0), right, y + 176.0 * s, sm, CREAM, Align::Left);
        self.cv.paragraph("Lower dead zone until small movements register. If the green dot moves at rest, raise it or calibrate. More softness gives finer corrections near center. Full travel keeps full authority.", right, y + 194.0 * s, w * 0.44, sm, 1.4, DIM);
        self.button(if pad.calibrating() { "CANCEL CALIBRATION" } else { "AUTO CALIBRATE DRIFT" }, "", left, y + 245.0 * s, false, Action::PadCalibrate);
        self.cv.paragraph(&pad.calibration_message(now), left, y + 270.0 * s, w - 24.0 * s, sm, 1.4, AMBER);
        self.button("BACK", "ESC", left, y + h - 26.0 * s, false, Action::PadSetup);
        if !pad.calibrating() {
            let bw = self.button_width("RESET CONTROLLER", "");
            self.button("RESET CONTROLLER", "", x + w - 12.0 * s - bw, y + h - 26.0 * s, false, Action::PadReset);
        }
    }

    /// The controller map beside the drawer, in the same frame as the key
    /// bindings. False when the screen is too narrow for it.
    fn pad_map(&mut self, rows: &[(&str, &str)], x0: f32) -> bool {
        let s = self.s;
        let sm = s.max(2.0) - 1.0;
        let m = 8.0 * s;
        let w = (340.0 * s).min(self.w - x0 - 4.0);
        if w < 120.0 * s { return false; }
        let (x, y0, h) = (x0 + 4.0, 12.0 * s, self.h - 24.0 * s);
        self.cv.layout_item(crate::menu_layout::Kind::Panel,"input map",[x,y0,w,h]);
        self.cv.rect(x, y0, w, h, rgba(0x09, 0x10, 0x12, 0.95));
        self.cv.stroke_rect(x, y0, w, h, 1.0, LINE);
        let key_w = rows.iter().map(|&(k, _)| Canvas::text_width(k, sm)).fold(72.0 * sm, f32::max) + 8.0 * sm;
        let mut y = y0 + m + 4.0 * s;
        self.cv.text("INPUT MAP: CONTROLLER", x + m, y, sm, CREAM, Align::Left);
        y += 16.0 * sm;
        for &(k, v) in rows {
            self.cv.text(k, x + m, y, sm, ACID, Align::Left);
            let used = self.cv.paragraph(v, x + m + key_w, y, w - 2.0 * m - key_w, sm, 1.25, rgba(0xb3, 0xc3, 0xb7, 1.0));
            y += used.max(11.0 * sm);
        }
        true
    }

    /// Key bindings beside the drawer: every flight action with its key and
    /// its alternate. Clicking a key waits for the next key press.
    fn key_bindings(&mut self, keys: &Keymap, x0: f32, waiting: Option<(usize, usize)>) {
        let s = self.s;
        let sm = s.max(2.0) - 1.0;
        let m = 8.0 * s;
        let row = (13.0 * sm).min(((self.h - 68.0 * s - 35.0 * sm) / BINDS.len() as f32).max(10.0 * sm));
        let w = (340.0 * s).min(self.w - x0 - 4.0);
        if w < 120.0 * s {
            return;
        }
        let h = self.h - 24.0 * s;
        let (x, y0) = (x0 + 4.0, 12.0 * s);
        self.cv.layout_item(crate::menu_layout::Kind::Panel,"input map",[x,y0,w,h]);
        self.cv.rect(x, y0, w, h, rgba(0x09, 0x10, 0x12, 0.95));
        self.cv.stroke_rect(x, y0, w, h, 1.0, LINE);
        let mut y = y0 + m + 4.0 * s;
        self.cv.text("INPUT MAP: KEYBOARD", x + m, y, sm, CREAM, Align::Left);
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
        self.cv.text("BACKSPACE CLEARS, ESC CANCELS", x + m, y + 23.0 * s, sm, DIM, Align::Left);
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
    // Past the limits the overridden computer no longer holds.
    if let Some(text) = a.flcs.danger {
        w.push(Warn { text, color: RED });
    }
    if a.gear < 0.95 && !a.on_ground && a.agl < 300.0 && a.vs < -2.0 && a.airspeed_kt < 220.0 {
        w.push(Warn { text: "GEAR UP", color: RED });
    }
    if a.gear > 0.0 && a.airspeed_kt > a.kind.gear_limit_kt() {
        w.push(Warn { text: "GEAR SPEED", color: AMBER });
    }
    if a.gear_broken.iter().any(|&b| b) {
        w.push(Warn { text: "GEAR BROKEN", color: RED });
    } else if a.gear_damage >= crate::sim::spec::GEAR_JAM {
        w.push(Warn { text: "GEAR JAMMED", color: RED });
    } else if a.gear_damage > 0.1 {
        w.push(Warn { text: "GEAR DAMAGED", color: AMBER });
    }
    if a.flap_damage > 0.1 {
        w.push(Warn { text: "FLAPS DAMAGED", color: AMBER });
    }
    // Past the placard, on the way to the flutter boundary.
    if a.structure.margin > 0.87 {
        w.push(Warn { text: "OVERSPEED", color: AMBER });
    }
    // Battle damage, worst first.
    let b = &a.battle;
    if b.fire {
        w.push(Warn { text: "FIRE", color: RED });
    }
    if b.thrust() == 0.0 {
        w.push(Warn { text: "ENGINE OUT", color: RED });
    } else if b.engine >= 0.45 && a.kind.engine_count()>1 {
        w.push(Warn { text: "ONE ENGINE OUT", color: AMBER });
    } else if b.engine > 0.0 {
        w.push(Warn { text: "ENGINE DAMAGE", color: AMBER });
    }
    if b.controls > 0.0 {
        w.push(Warn { text: "HYDRAULICS", color: AMBER });
    }
    if b.wing > 0.0 {
        w.push(Warn { text: "WING DAMAGE", color: AMBER });
    }
    if b.leak > 0.0 {
        w.push(Warn { text: "FUEL LEAK", color: AMBER });
    }
    if a.structure.damage > 0.25 || a.battle.airframe > 0.0 {
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
    pub layout: PadLayout,
    pub click_rudder: bool,
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
        cv.text("AIR BRK", x - 4.0 * s, y + b - 4.0 * s, sm, AMBER, Align::Right);
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
    let keys = if cue.pad { "{BACK} CONTROLS    {RS} LOOK    {Y} CAMERA" } else { "ESC  CONFIG    ALT + MOUSE  LOOK    C  CENTER    V  CHASE" };
    cv.text(keys, 12.0 * sm, h - 10.0 * sm, sm, rgba(0xbf, 0xd0, 0xbf, 0.8), Align::Left);
    stick_cue(cv, rx - 22.0 * s, h - 20.0 * sm - 30.0 * s, s, cue);
    // Above the combiner, which the default head pitch lifts high on screen.
    toast_line(cv, w, 11.0 * s, s, toast, toast_t);
    limiter_line(cv, w, 26.0 * s, s, a);
}

/// The flight computer holding the jet to its limits against the stick:
/// steady red, so a pull that goes no further reads as protection.
/// With the limiters overridden it says so in amber, and in red what the
/// jet is doing past them.
fn limiter_line(cv: &mut Canvas, w: f32, y: f32, s: f32, a: &Aircraft) {
    let name = if a.kind.russian() { "SOS" } else { "FLCS" };
    if a.flcs.limits_off && !a.crashed {
        let (label, col) = match a.flcs.danger {
            Some(text) => (format!("{text} // {name} LIMITERS OFF"), RED),
            None => (format!("{name} LIMITERS OFF"), AMBER),
        };
        let tw = Canvas::text_width(&label, s);
        cv.rect(w * 0.5 - tw * 0.5 - 6.0 * s, y - 7.0 * s, tw + 12.0 * s, 14.0 * s, rgba(4, 10, 6, 0.55));
        cv.text(&label, w * 0.5, y, s, col, Align::Center);
        return;
    }
    if let Some(text) = a.flcs.limit.filter(|_| !a.crashed) {
        let label = format!("{name} {text}");
        let tw = Canvas::text_width(&label, s);
        cv.rect(w * 0.5 - tw * 0.5 - 6.0 * s, y - 7.0 * s, tw + 12.0 * s, 14.0 * s, rgba(4, 10, 6, 0.55));
        cv.text(&label, w * 0.5, y, s, RED, Align::Center);
    }
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

    cv.text(&format!("{}  //  CHASE VIEW", a.kind.full_name()), edge, 10.0 * s, sm, HUD, Align::Left);
    cv.text("COLD RIVER  /  SECTOR 07", edge, 19.0 * s, sm, with_alpha(HUD, 0.48), Align::Left);
    let title = obj.map(|o| o.label.clone()).unwrap_or_else(|| format!("GATE {} / {}", pad((m.gate_index + 1).min(m.gates.len()) as f64, 2), pad(m.gates.len() as f64, 2)));
    cv.text(&title, w * 0.5, 9.0 * s, sm, AMBER, Align::Center);
    let clk = if m.timed { m.time_left } else { m.elapsed };
    cv.text(&format_time(clk), w * 0.5, 22.0 * s, s * 2.0, if m.timed && m.time_left < 20.0 { RED } else { HUD }, Align::Center);
    cv.text(&format!("{} KT", pad(a.airspeed_kt, 3)), w - edge, 10.0 * s, s, HUD, Align::Right);
    cv.text(&format!("ALT {} FT", format!("{:.0}", AirData::of(a).baro_ft)), w - edge, 20.0 * s, sm, with_alpha(HUD, 0.68), Align::Right);
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
    limiter_line(cv, w, top_h + 27.0 * s, s, a);
    let warns = warnings(a, m);
    if !warns.is_empty() && flying && ((time * 4.5).floor() as i64) % 2 == 0 {
        let y = h * 0.60;
        cv.text(warns[0].text, w * 0.5, y, s * 2.0, warns[0].color, Align::Center);
        if let Some(w2) = warns.get(1) {
            cv.text(w2.text, w * 0.5, y + 16.0 * s, s, w2.color, Align::Center);
        }
    }
    cv.text(if cue.pad { "{RS} ORBIT" } else { "C CENTER  //  ALT + MOUSE ORBIT" }, edge, h - 24.0 * s, sm, with_alpha(HUD, 0.68), Align::Left);
    cv.text(&format!("FUEL {} {}",pad(a.fuel * if a.kind.russian(){1.0}else{2.20462},4),if a.kind.russian(){"KG"}else{"LB"}), edge, h - 14.0 * s, sm, if a.fuel < 500.0 { RED } else { with_alpha(HUD, 0.48) }, Align::Left);
    if a.gear > 0.0 || a.flaps > 0.0 {
        let gear = if a.gear >= 1.0 { "GEAR DN" } else if a.gear > 0.0 { "GEAR TRANSIT" } else { "GEAR UP" };
        let text = format!("{gear}  FLAPS {}{}", (a.flaps * a.kind.flap_angle()).round() as i64, if a.wheel_brake > 0.1 { "  BRAKES" } else { "" });
        cv.text(&text, edge, h - 44.0 * s, sm, if a.gear > 0.0 && a.gear < 1.0 { AMBER } else { with_alpha(HUD, 0.6) }, Align::Left);
    }
    cv.text(if cue.pad { "{Y} COCKPIT VIEW" } else { "V  COCKPIT VIEW" }, w - edge, h - 24.0 * s, sm, with_alpha(HUD, 0.68), Align::Right);
    let hint = if cue.pad {
        if cue.layout == PadLayout::Combat && cue.click_rudder { "{LS} PITCH / ROLL  ,  HOLD {X} / {L3} RUDDER" }
        else if cue.layout == PadLayout::Combat { "{LS} PITCH / ROLL  ,  {X} + {LS} RUDDER" }
        else { "{LS} PITCH / ROLL  ,  {LB} {RB} RUDDER" }
    } else if cue.source == PitchSource::Mouse { "A / D  ROLL  ,  MOUSE PITCH / YAW" }
    else { "A / D  ROLL  ,  W / S  PITCH  ,  MOUSE YAW" };
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
        format!("BARO {} FT  QNH {:.2}", format!("{:.0}", ad.baro_ft), ad.qnh_inhg),
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

#[cfg(test)]
mod menu_geometry_tests {
    use super::*;
    #[test]
    fn window_borders_do_not_halve_the_overlay_font() {
        assert_eq!(ui_scale(1280.0, 701.0), 2.0);
        assert_eq!(ui_scale(1920.0, 1061.0), 3.0);
        assert_eq!(ui_scale(800.0, 600.0), 1.0);
        assert_eq!(ui_scale(960.0, 540.0), 1.0);
    }

    #[test]
    fn directional_navigation_keeps_rows_and_columns() {
        let buttons = [
            Button { x: 10.0, y: 10.0, w: 100.0, h: 20.0, action: Action::Start },
            Button { x: 120.0, y: 10.0, w: 80.0, h: 20.0, action: Action::Mute },
            Button { x: 10.0, y: 40.0, w: 100.0, h: 20.0, action: Action::Controls },
            Button { x: 120.0, y: 40.0, w: 80.0, h: 20.0, action: Action::Quit },
        ];
        assert_eq!(navigation(&buttons, 0, 1.0, 0.0), Some(1));
        assert_eq!(navigation(&buttons, 0, 0.0, 1.0), Some(2));
        assert_eq!(navigation(&buttons, 3, 0.0, -1.0), Some(1));
        assert_eq!(navigation(&buttons, 3, -1.0, 0.0), Some(2));
        assert_eq!(navigation(&buttons, 0, 0.0, -1.0), None);
    }
}


pub fn training_overlay(cv: &mut Canvas, w: f32, h: f32, t: &crate::tutorial::Training, settings: &Settings, pad: bool) {
    let s = ui_scale(w,h); let sm = (s-1.0).max(1.0);
    let pw = (w-24.0*s).min(410.0*s);
    let (x,y) = ((w-pw)/2.0, h-80.0*s);
    cv.layout_item(crate::menu_layout::Kind::Panel,"training instruction",[x,y,pw,66.0*s]);
    cv.rect(x,y,pw,66.0*s,PANEL); cv.stroke_rect(x,y,pw,66.0*s,1.0,LINE_HOT);
    let (title, detail, _) = t.instruction();
    cv.text(&format!("{}  {}/{}: {}", t.lesson.name(), t.stage+1, t.lesson.steps(), title),x+8.0*s,y+9.0*s,sm,ACID,Align::Left);
    cv.paragraph(detail,x+8.0*s,y+20.0*s,pw-16.0*s,sm,1.3,CREAM);
    cv.text(&t.hint(&settings.keys,pad,settings.pad_layout,settings.pitch,settings.click_rudder()),x+8.0*s,y+51.0*s,sm,AMBER,Align::Left);
    cv.rect(x+8.0*s,y+60.0*s,pw-16.0*s,2.0*s,LINE);
    cv.rect(x+8.0*s,y+60.0*s,(pw-16.0*s)*(t.held/t.duration().max(0.01)).min(1.0) as f32,2.0*s,ACID);
}


pub fn network_overlay(cv:&mut Canvas,w:f32,h:f32,mp:&crate::multiplayer::Multiplayer,c:&crate::combat::Combat,a:&Aircraft,cam:&Camera){
    let s=ui_scale(w,h);let sm=(s-1.0).max(1.0);
    let Some(session)=&mp.session else{return};
    let vs=mp.settings();
    let text=if vs.deafened{"VOICE DEAFENED"}else if vs.muted{"MIC MUTED"}else if mp.voice.transmitting(){"TRANSMITTING"}else if vs.mode==crate::voice::VoiceMode::Disabled{"MIC OFF"}else{"VOICE READY"};
    cv.text(&format!("ONLINE {} PILOTS, {}  ESC CONFIG / VOICE",session.members.len(),text),w*0.5,12.0*s,sm,if mp.voice.transmitting(){ACID}else{GREEN},Align::Center);
    if let Some(b)=&mp.battle{
        let mut behind=0usize;
        for(pos,label)in b.labels(session,c){
            let r=pos-cam.eye;let depth=r.dot(cam.forward);
            let focal=h as f64*0.5/(cam.fov*0.5).tan();
            let side=r.dot(cam.right);let up=r.dot(cam.up);
            let px=(w as f64*0.5+side*focal/depth.abs().max(1.0)).clamp(40.0*s as f64,w as f64-40.0*s as f64)as f32;
            let py=(h as f64*0.5-up*focal/depth.abs().max(1.0)).clamp(34.0*s as f64,h as f64-100.0*s as f64)as f32;
            let color=rgba(0x72,0xdf,0xff,0.95);
            if depth>0.0 {
                cv.line(px-4.0*s,py,px,py-4.0*s,1.0,color);cv.line(px,py-4.0*s,px+4.0*s,py,1.0,color);
                cv.line(px+4.0*s,py,px,py+4.0*s,1.0,color);cv.line(px,py+4.0*s,px-4.0*s,py,1.0,color);
                cv.text(&format!("{} {:.1}KM",label,(pos-a.pos).len()/1000.0),px,py+11.0*s,sm,color,Align::Center);
            } else {
                cv.text(&format!("{} BEHIND, {:.1}KM",label,(pos-a.pos).len()/1000.0),w*0.5,h-88.0*s-behind as f32*11.0*sm,sm,color,Align::Center);behind+=1;
            }
        }
        if c.player_dead&&!b.finished {
            cv.text(if session.rules.mode.pvp(){"AIRCRAFT LOST. RESPAWN PENDING."}else{"AIRCRAFT LOST. WATCHING THE SORTIE."},w*0.5,h*0.42,s,AMBER,Align::Center);
        }
        if b.finished{
            cv.rect(w*0.16,h*0.35,w*0.68,65.0*s,PANEL);
            cv.text("SORTIE ENDED",w*0.5,h*0.35+14.0*s,2.0*s,ACID,Align::Center);
            cv.paragraph(&b.objective,w*0.18,h*0.35+32.0*s,w*0.64,sm,1.5,CREAM);
            cv.text("ESC CONFIG, ONLINE / VOICE TO LEAVE OR RETURN",w*0.5,h*0.35+54.0*s,sm,DIM,Align::Center);
        }
    }
}
