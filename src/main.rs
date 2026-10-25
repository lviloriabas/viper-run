//! VIPER RUN: a small retro F-16 cockpit flight game.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod aero;
mod aero_mig29;
mod aero_hornet;
mod aero_su27;
mod ai;
mod air;
mod airframe;
mod audio;
mod canvas;
mod capture;
mod combat;
mod cockpit;
mod damage;
mod eject;
mod font;
mod fx;
mod gamepad;
mod gfx;
mod icon;
mod keymap;
mod math;
mod mesh;
mod mig29;
mod hornet;
mod su27;
mod physio;
mod performance;
mod png;
mod props;
mod scenario;
mod scenery;
mod settings;
mod sim;
mod structure;
mod symbology;
mod targets;
mod terrain;
mod texture;
mod ui;
mod tutorial;
mod net;
mod voice;
mod operation;
mod multiplayer;
mod ui_check;
mod menu_layout;
mod vapor;
mod weapons;
mod warnings;
mod world;

use audio::{Audio, Params, Wave};
use canvas::{rgba, Canvas};
use cockpit::{Cockpit, Hands, Page};
use combat::{Combat, MissionKind, Mode};
use gamepad::Gamepad;
use keymap::Bind;
use math::*;
use miniquad::*;
use physio::Pilot;
use settings::{Best, PadLayout, PitchSource, Settings};
use sim::{gate_crossed, Aircraft, Input, Mission, FIXED_DT, GATE_BONUS};
use std::collections::HashSet;
use ui::{Action, Button, Objective, Outcome, RunResult, Ui};
use world::{Camera, World};

/// Default cockpit head pitch: 14 degrees down keeps the HUD, the panel
/// and a good slice of sky in view; the stick and throttle heads show at
/// the bottom corners and the hands mostly below the frame.
const LOOK_PITCH: f64 = -14.0 * DEG;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    Attract,
    School,
    Network,
    Flying,
    /// Everything stops, but the camera can still look around and switch
    /// views, for screenshots.
    Frozen,
    Paused,
    Transition,
    Result,
    /// Stores and fuel before a combat sortie.
    Loadout,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum View {
    Cockpit,
    Chase,
}

/// Command-line switches, mostly for testing and screenshots.
#[derive(Default)]
struct Opts {
    autostart: bool,
    tutorial: Option<tutorial::Lesson>,
    host:Option<String>, join:Option<String>, directory:Option<String>, callsign:Option<String>, online_mode:Option<net::GameMode>,
    ready:bool, launch_ready:bool, invite_file:Option<String>, net_report:Option<String>, exit_after:Option<f64>, future_ui_check:bool,

    audio_demo: Option<String>,
    no_mirrors: bool,
    ui_check: bool,
    layout_check: bool,
    layout_failed: bool,
    layout_scene: Option<String>,
    chase: bool,
    shot: Option<String>,
    frames: u32,
    look: Option<(f64, f64)>,
    sim_seconds: f64,
    input: Input,
    screen: Option<String>,
    /// Pilot state for screenshots: retinal oxygen, redout, G-LOC seconds left.
    vision: Option<[f64; 3]>,
    /// Start speed as a Mach number, for screenshots of transonic effects.
    mach: Option<f64>,
    /// Parts broken off the jet at the start, to show it coming apart.
    breakup: Option<sim::Breakup>,
    broken_gear: bool,
    controls: bool,
    performance_overlay: Option<bool>,
    /// Show the controller prompts, for screenshots without a pad.
    pad: bool,
    fov: Option<f64>,
    /// Chase camera distance scale, for close-ups of the airframe.
    zoom: Option<f64>,
    bench: Option<f64>,
    size: Option<(i32, i32)>,
    mission: Option<MissionKind>,
    /// Hold the trigger, and lock the nearest target, through a scripted run.
    fire: bool,
    /// --input eject: pull the handle at once.
    eject: bool,
    lock: bool,
    /// Track with the MiG's IRST instead of the radar.
    irst: bool,
    /// Fly the scripted run with the AI pilot toward the steerpoint.
    auto: bool,
    weapon: Option<Mode>,
    /// Airframes for the player and the bandits.
    jet: Option<sim::Type>,
    bandits: Option<sim::Type>,
}

fn parse_opts() -> Opts {
    let mut o = Opts { frames: 3, ..Default::default() };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let next = |i: &mut usize| -> String {
            *i += 1;
            args.get(*i).cloned().unwrap_or_default()
        };
        match args[i].as_str() {
            "--host" => o.host=Some(next(&mut i)),
            "--join" => o.join=Some(next(&mut i)),
            "--directory" => o.directory=Some(next(&mut i)),
            "--callsign" => o.callsign=Some(next(&mut i)),
            "--online-mode" => o.online_mode=net::GameMode::parse(&next(&mut i)),
            "--ready" => o.ready=true,
            "--launch-when-ready" => o.launch_ready=true,
            "--invite-file" => o.invite_file=Some(next(&mut i)),
            "--net-report" => o.net_report=Some(next(&mut i)),
            "--exit-after" => o.exit_after=next(&mut i).parse::<f64>().ok().filter(|t|t.is_finite()&&*t>0.0),
            "--future-ui-check" => o.future_ui_check=true,
            "--autostart" => o.autostart = true,
            "--tutorial" => { o.tutorial = tutorial::Lesson::parse(&next(&mut i)); o.autostart = o.tutorial.is_some(); },
            "--audio-demo" => o.audio_demo = Some(next(&mut i)),
            "--no-mirrors" => o.no_mirrors = true,
            "--ui-check" => o.ui_check = true,
            "--layout-check" => o.layout_check = true,
            "--layout-scene" => o.layout_scene = Some(next(&mut i)),
            "--chase" => o.chase = true,
            "--break-gear" => o.broken_gear = true,
            "--shot" => o.shot = Some(next(&mut i)),
            "--frames" => o.frames = next(&mut i).parse().unwrap_or(3),
            "--look" => {
                let v = next(&mut i);
                let mut it = v.split(',').filter_map(|s| s.parse::<f64>().ok());
                if let (Some(a), Some(b)) = (it.next(), it.next()) {
                    o.look = Some((a, b));
                }
            }
            "--sim" => o.sim_seconds = next(&mut i).parse().unwrap_or(0.0),
            "--input" => {
                for k in next(&mut i).split(',') {
                    match k {
                        "pull" => o.input.pitch_up = true,
                        "push" => o.input.pitch_down = true,
                        "left" => o.input.roll_left = true,
                        "right" => o.input.roll_right = true,
                        "yawleft" => o.input.yaw_left = true,
                        "yawright" => o.input.yaw_right = true,
                        "ab" => {
                            o.input.afterburner = true;
                            o.input.throttle_up = true;
                        }
                        "throttle" => o.input.throttle_up = true,
                        "brake" => o.input.airbrake = true,
                        "nolimit" => o.input.limiters_off = true,
                        "gear" => o.input.gear_down = true,
                        "flaps" => o.input.flaps = true,
                        "wheelbrake" => o.input.wheel_brake = true,
                        "fire" => o.fire = true,
                        "eject" => o.eject = true,
                        "lock" => o.lock = true,
                        "irst" => o.irst = true,
                        "auto" => o.auto = true,
                        _ => {}
                    }
                }
            }
            "--screen" => o.screen = Some(next(&mut i)),
            "--mach" => o.mach = next(&mut i).parse().ok(),
            "--break" => {
                let s = next(&mut i);
                o.breakup = Some(sim::Breakup { wing: [s == "left", s == "right"], tail: s == "tail" });
            }
            "--vision" => {
                let v: Vec<f64> = next(&mut i).split(',').filter_map(|s| s.parse().ok()).collect();
                o.vision = Some([v.first().copied().unwrap_or(1.0), v.get(1).copied().unwrap_or(0.0), v.get(2).copied().unwrap_or(0.0)]);
            }
            "--mission" => o.mission = MissionKind::parse(&next(&mut i)),
            "--jet" | "--bandits" => {
                let t = sim::Type::parse(&next(&mut i)).unwrap_or_default();
                if args[i - 1] == "--jet" {
                    o.jet = Some(t);
                } else {
                    o.bandits = Some(t);
                }
            }
            "--weapon" => {
                o.weapon = match next(&mut i).as_str() {
                    "nav" => Some(Mode::Nav),
                    "gun" => Some(Mode::Gun),
                    "strf" => Some(Mode::Strf),
                    "aim9" | "srm" => Some(Mode::Srm),
                    "aim120" | "mrm" => Some(Mode::Mrm),
                    "mk82" | "ccip" => Some(Mode::Ccip),
                    _ => None,
                }
            }
            "--controls" => o.controls = true,
            "--performance" => o.performance_overlay = Some(true),
            "--no-performance" => o.performance_overlay = Some(false),
            "--pad" => o.pad = true,
            "--fov" => o.fov = next(&mut i).parse().ok(),
            "--zoom" => o.zoom = next(&mut i).parse().ok(),
            "--bench" => o.bench = next(&mut i).parse().ok(),
            "--size" => {
                let v = next(&mut i);
                if let Some((w, h)) = v.split_once('x') {
                    o.size = w.parse().ok().zip(h.parse().ok());
                }
            }
            "--help" | "-h" => {
                println!("viper-run [--autostart] [--chase] [--shot FILE.png] [--frames N] [--look YAW,PITCH] [--sim SECONDS] [--input pull,left,right,yawleft,yawright,throttle,ab,brake,nolimit,fire,lock,irst,auto] [--screen pause|freeze|win|crash|timeout|shotdown|menu] [--vision RETINA,RED,GLOC] [--mach M] [--break left|right|tail] [--controls] [--pad] [--performance|--no-performance] [--zoom K] [--mission gates|dogfight|strike|airfield|free] [--jet f16|mig29|f18|su27] [--bandits f16|mig29|f18|su27] [--weapon nav|gun|aim9|aim120|mk82]");
                println!("Flight school: --tutorial controls|flight|weapons|takeoff|landing");
                println!("Online: --host ADDRESS:PORT | --join INVITATION [--directory ADDRESS:PORT] [--callsign NAME] [--online-mode dogfight|teams|combat|strike|escort|defense] [--ready] [--launch-when-ready]");
                println!("Layout QA: --layout-check --shot FILE.png [--layout-scene NAME]");
                println!("QA: --future-ui-check --screen menu --shot FILE.png --frames 200; --invite-file FILE --net-report FILE --exit-after SECONDS");
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }
    assert!(!o.layout_check || o.shot.is_some(),"--layout-check requires --shot FILE.png");
    assert!(o.layout_scene.is_none() || o.layout_check,"--layout-scene requires --layout-check");
    o
}

struct App {
    ctx: Box<dyn RenderingBackend>,
    world: World,
    cockpit: Cockpit,
    overlay: Canvas,
    atlas_canvas: Canvas,
    scene: gfx::Target,
    /// The rear-view mirrors' picture, copied into the cockpit atlas.
    mirror: gfx::Target,
    output: Option<gfx::Target>,
    audio: Audio,
    settings: Settings,
    performance: performance::Performance,
    best: Best,

    state: State,
    view: View,
    mission: Mission,
    preview: Mission,
    /// Mission chosen on the start screen, and the fight it sets up.
    kind: MissionKind,
    /// Stores and fuel chosen on the loadout screen.
    config: combat::Config,
    training: Option<tutorial::Training>,
    multiplayer: multiplayer::Multiplayer,
    combat: Combat,
    models: props::Models,
    /// Left mouse button held as the trigger.
    trigger_mouse: bool,
    rwr_voice: props::RwrVoice,
    auto_pilot: Option<ai::Pilot>,
    aircraft: Option<Aircraft>,
    /// The pilot's G tolerance and consciousness.
    pilot: Pilot,
    camera: Camera,
    time: f64,
    attract_time: f64,
    accumulator: f64,
    last_frame: f64,
    toast: String,
    toast_timer: f64,
    transition_timer: f64,
    transition_kind: Outcome,
    flash: f64,
    flash_color: [u8; 3],
    warnings: warnings::Warnings,
    /// Seconds the countermeasure button has been held since the last program.
    cm_hold: f64,
    /// The pilot's ejection once the handle is pulled, and how long it has
    /// been held.
    ejection: Option<eject::Ejection>,
    eject_hold: f64,
    /// The jet the pilot left has hit the ground.
    eject_wreck: bool,
    /// The pad's weapon button was held into a jettison.
    pad_jettisoned: bool,
    result: Option<RunResult>,
    /// The external camera sits inside the jet's Mach cone, where its boom
    /// has already passed.
    in_cone: bool,
    /// AGSM breaths already voiced.
    breaths: u32,
    /// Decaying jolt added to the camera shake by a light-off or an impact.
    kick: f64,
    crash_time: f64,
    /// The player's jet broken in the air and still falling.
    falling: bool,

    keys: HashSet<KeyCode>,
    pad: Gamepad,
    /// The controller was the last input used: on-screen prompts show its
    /// buttons until the keyboard or mouse is touched again.
    pad_active: bool,
    /// Synthetic menu keys retain their controller input context.
    pad_dispatch: bool,
    /// Smoothed right-stick position driving the head or chase camera.
    pad_look: (f64, f64),
    /// Zoom held on the right mouse button or the right stick click, and the eased
    /// 0..1 amount that narrows the field of view.
    zoom_mouse: bool,
    zoom_pad: bool,
    zoom: f64,
    mouse: (f64, f64),
    /// Keyboard pitch, ramped so a tap gives a partial pull and a hold a full one.
    key_pitch: f64,
    airbrake: bool,
    /// The FLCS limiters overridden (ALT+O, or both sticks clicked).
    limiters_off: bool,
    /// Landing gear handle down, and the flaps switch.
    gear_lever: bool,
    flaps_switch: bool,
    /// Action and slot waiting for a key in the controls drawer.
    rebinding: Option<(usize, usize)>,
    mouse_pos: (f32, f32),
    grabbed: bool,
    screenshot_pending: bool,
    look: (f64, f64),
    /// Chase camera orbit: yaw (positive swings the camera to the jet's
    /// right, so the view turns left; the controls feed it negated, so
    /// right looks right as in the cockpit) and pitch.
    chase_look: (f64, f64),
    controls_open: bool,
    controls_was_flying: bool,
    controls_was_frozen: bool,
    paused_was_frozen: bool,
    pages: [Page; 2],
    hands: Hands,
    buttons: Vec<Button>,
    selected: usize,
    win: (f32, f32),

    opts: Opts,
    frame: u32,
    bench_frames: Vec<f64>,
}

/// Mouse stick shaping: a small dead band makes neutral easy to find, and the
/// rest of the travel is rescaled so there is no jump at its edge.
fn stick_curve(v: f64) -> f64 {
    const DEAD: f64 = 0.04;
    v.signum() * ((v.abs() - DEAD).max(0.0) / (1.0 - DEAD)).min(1.0)
}

/// Seconds the pad's weapon button is held to jettison the selected store.
const JETTISON_HOLD: f64 = 0.8;

/// Seconds between countermeasure programs while the button is held.
const CM_REPEAT: f64 = 1.0;

/// Low internal resolution, shown as crisp blocks: about 1.9 window pixels
/// per scene pixel, between 360 and 576 lines.
fn render_size(w: f32, h: f32) -> (u32, u32) {
    let th = clamp((h as f64 / 1.9).round(), 360.0, 576.0).min(h as f64).max(1.0);
    let tw = (th * (w as f64 / h.max(1.0) as f64)).round().max(1.0);
    (tw as u32, th as u32)
}

impl App {
    fn new(opts: Opts) -> App {
        let mut ctx: Box<dyn RenderingBackend> = window::new_rendering_backend();
        if opts.bench.is_some() {
            println!("graphics: {:?}", ctx.info());
        }
        gfx::init_header(&*ctx);
        let font_tex = ctx.new_texture_from_data_and_format(
            &font::atlas(),
            TextureParams { width: font::ATLAS_W as u32, height: font::ATLAS_H as u32, format: TextureFormat::RGBA8, min_filter: FilterMode::Nearest, mag_filter: FilterMode::Nearest, ..Default::default() },
        );
        let frame = airframe::build();
        let world = World::new(&mut *ctx, &frame);
        let models = props::Models::new(&mut *ctx, &frame, &world);
        let cockpit = Cockpit::new(&mut *ctx, &frame.canopy, &mig29::build().canopy);
        let overlay = Canvas::new(&mut *ctx, font_tex, 240_000);
        let atlas_canvas = Canvas::new(&mut *ctx, font_tex, 400_000);
        let (w, h) = window::screen_size();
        let (rw, rh) = render_size(w, h);
        let scene = gfx::Target::new(&mut *ctx, rw, rh, FilterMode::Nearest);
        let mirror = gfx::Target::new(&mut *ctx, cockpit::MIRROR_SIZE.0, cockpit::MIRROR_SIZE.1, FilterMode::Nearest);
        let (mut settings, best) = settings::load();
        if let Some(enabled) = opts.performance_overlay { settings.performance_overlay = enabled; }
        let audio = Audio::start();
        let preview = Mission::new();
        let mut mission = preview.clone();
        let kind = opts.mission.unwrap_or(MissionKind::Gates);
        let combat = Combat::new(MissionKind::Gates, &mut mission);
        if settings.fullscreen && opts.shot.is_none() {
            window::set_fullscreen(true);
        }
        let mut app = App {
            ctx,
            world,
            cockpit,
            overlay,
            atlas_canvas,
            scene,
            mirror,
            output: None,
            audio,
            settings,
            performance: performance::Performance::new(),
            best,
            state: State::Attract,
            view: if opts.chase { View::Chase } else { View::Cockpit },
            mission,
            preview,
            kind,
            config: combat::Config { aggressor: opts.bandits.unwrap_or(sim::Type::Mig29), ..combat::Config::for_jet(kind, opts.jet.unwrap_or_default()) },
            combat,
            training: None,
            multiplayer: multiplayer::Multiplayer::default(),
            models,
            trigger_mouse: false,
            rwr_voice: props::RwrVoice::default(),
            auto_pilot: None,
            aircraft: None,
            pilot: Pilot::default(),
            camera: Camera { eye: V3::ZERO, forward: -V3::Z, right: V3::X, up: V3::Y, fov: 1.0 },
            time: 0.0,
            attract_time: 0.0,
            accumulator: 0.0,
            last_frame: date::now(),
            toast: String::new(),
            toast_timer: 0.0,
            transition_timer: 0.0,
            transition_kind: Outcome::Win,
            flash: 0.0,
            flash_color: [180, 255, 190],
            warnings: warnings::Warnings::default(),
            cm_hold: 0.0,
            ejection: None,
            eject_hold: 0.0,
            eject_wreck: false,
            pad_jettisoned: false,
            result: None,
            in_cone: false,
            breaths: 0,
            kick: 0.0,
            crash_time: 0.0,
            falling: false,
            keys: HashSet::new(),
            pad: Gamepad::new(),
            pad_active: false,
            pad_dispatch: false,
            pad_look: (0.0, 0.0),
            zoom_mouse: false,
            zoom_pad: false,
            zoom: 0.0,
            mouse: (0.0, 0.0),
            key_pitch: 0.0,
            airbrake: false,
            limiters_off: false,
            gear_lever: false,
            flaps_switch: false,
            rebinding: None,
            mouse_pos: (0.0, 0.0),
            grabbed: false,
            screenshot_pending: false,
            look: (0.0, LOOK_PITCH),
            chase_look: (0.0, 0.0),
            controls_open: false,
            controls_was_flying: false,
            controls_was_frozen: false,
            paused_was_frozen: false,
            pages: [Page::Fcr, Page::Eng],
            hands: Hands { throttle: 0.82, ..Default::default() },
            buttons: Vec::new(),
            selected: 0,
            win: (w, h),
            opts,
            frame: 0,
            bench_frames: Vec::new(),
        };
        app.performance.set_enabled(app.settings.performance_overlay);
        app.apply_test_options();
        app.network_options();
        app
    }

    fn apply_test_options(&mut self) {
        if self.opts.bench.is_some() {
            self.start_mission();
        }
        if self.opts.autostart || self.opts.shot.is_some() && !matches!(self.opts.screen.as_deref(), Some("menu") | Some("loadout") | Some("school") | Some("network")) {
            if self.opts.autostart || self.opts.sim_seconds > 0.0 || self.opts.look.is_some() || self.opts.screen.is_some() || self.opts.chase {
                self.start_mission();
            }
        }
        if let Some((y, p)) = self.opts.look {
            if self.view == View::Cockpit {
                self.look = (clamp(y * DEG, -155.0 * DEG, 155.0 * DEG), clamp(p * DEG, -85.0 * DEG, 85.0 * DEG));
            } else {
                self.chase_look = (-y * DEG, p * DEG);
            }
        }
        self.airbrake = self.opts.input.airbrake;
        self.limiters_off = self.opts.input.limiters_off;
        if let (Some(mach), Some(a)) = (self.opts.mach, self.aircraft.as_mut()) {
            a.vel = a.vel.norm() * mach * sim::atmosphere_at(a.pos.y).sound;
            a.update(0.0, &Input::default());
        }
        if let (Some(parts), Some(a)) = (self.opts.breakup, self.aircraft.as_mut()) {
            a.lose(parts);
        }
        if let (true,Some(a))=(self.opts.broken_gear,self.aircraft.as_mut()) {
            a.gear=1.0;
            a.gear_broken=[true;3];
            self.combat.shed_gear(a);
        }
        if let (true, Some(a)) = (self.opts.irst, self.aircraft.as_ref()) {
            self.combat.toggle_irst(a);
        }
        if self.opts.sim_seconds > 0.0 && self.state == State::Flying {
            let steps = (self.opts.sim_seconds / FIXED_DT).round() as usize;
            for _ in 0..steps {
                match self.state {
                    State::Flying => {
                        // --input lock: lock up the nearest target once one is in view.
                        if self.opts.lock && self.combat.lock.is_none() && self.combat.acquiring.is_none() && (self.time * 2.0).fract() < FIXED_DT * 2.0 {
                            if let Some(a) = &self.aircraft {
                                self.combat.designate(a);
                            }
                        }
                        let mut input = if self.opts.auto { self.autopilot() } else { self.opts.input };
                        input.gear_down |= self.gear_lever;
                        self.step(FIXED_DT, input);
                        self.advance_effects(FIXED_DT);
                    }
                    // Let an impact play out, so a screenshot can show it.
                    State::Transition => {
                        self.fall(FIXED_DT);
                        self.advance_effects(FIXED_DT);
                        self.flash = (self.flash - FIXED_DT * 2.4).max(0.0);
                    }
                    _ => break,
                }
                self.time += FIXED_DT;
                self.update_hands(FIXED_DT, Some(self.opts.input));
            }
        }
        if let Some([retina, red, gloc]) = self.opts.vision {
            self.pilot.retina = retina;
            self.pilot.red = red;
            self.pilot.gloc = gloc;
        }
        match self.opts.screen.as_deref() {
            Some("school") => self.state=State::School,
            Some("network") => self.state=State::Network,
            Some("pause") => self.pause(),
            Some("loadout") => self.state = State::Loadout,
            Some("freeze") => self.toggle_freeze(),
            Some("win") | Some("crash") | Some("timeout") | Some("shotdown") => {
                let kind = match self.opts.screen.as_deref() {
                    Some("win") => Outcome::Win,
                    Some("crash") => Outcome::Crash,
                    Some("shotdown") => Outcome::ShotDown,
                    _ => Outcome::Timeout,
                };
                let (_, total) = self.combat.tally();
                let done = if kind == Outcome::Win { total } else { total.saturating_sub(1) };
                self.result = Some(RunResult { kind, time: 112.0, gates: if kind == Outcome::Win { 10 } else { 6 }, score: 7650.0, impact: 190.0, mission: self.kind, done, total, cause: "" });
                self.state = State::Result;
            }
            _ => {}
        }
        if let Some(f) = self.opts.fov {
            self.settings.fov = f;
        }
        if self.opts.controls {
            self.toggle_controls(Some(true));
        }
        if let Some(scene)=self.opts.layout_scene.clone() { ui_check::layout_scene(self,&scene); }
    }

    // ------------------------------------------------------------ flow
    fn flash_toast(&mut self, text: &str, dur: f64) {
        self.toast = text.to_string();
        self.toast_timer = dur;
    }

    fn grab(&mut self, on: bool) {
        if self.opts.shot.is_some() {
            return;
        }
        if on != self.grabbed {
            window::set_cursor_grab(on);
            window::show_mouse(!on);
            self.grabbed = on;
        }
    }

    fn center_stick(&mut self) {
        self.mouse = (0.0, 0.0);
        self.look = (0.0, LOOK_PITCH);
        self.chase_look = (0.0, 0.0);
    }

    fn start_mission(&mut self) {
        if self.controls_open {
            self.toggle_controls(Some(false));
        }
        self.mission = Mission::new();
        let lesson = self.training.as_ref().map(|t| t.lesson).or(self.opts.tutorial);
        if let Some(l) = lesson {
            self.kind = l.mission();
            if self.training.is_none() {
                self.config = combat::Config::for_jet(if l==tutorial::Lesson::Landing {MissionKind::Airfield}else{self.kind},self.config.jet);
                if l != tutorial::Lesson::Weapons { self.config.threats.bandits=0; }
                self.config.threats.sams=0;self.config.threats.guns=0;self.config.refill=false;
                if l==tutorial::Lesson::Landing { self.config.fuel=0.5; }
            }
        }
        self.mission.timed = self.settings.clock && lesson.is_none();
        self.combat = Combat::new(self.kind, &mut self.mission);
        self.combat.training = lesson.is_some();
        self.combat.deploy(&self.config, &mut self.mission);
        if self.kind != MissionKind::Gates {
            self.combat.equip(&self.config);
        }
        self.rwr_voice = props::RwrVoice::default();
        if let Some(w) = self.opts.weapon {
            self.combat.select(w);
        }
        let mut a = Aircraft::new(&self.mission);
        a.set_type(self.config.jet);
        if a.on_ground { a.pos.y=terrain::floor(a.pos.x,a.pos.z)+a.kind.landing_height(); a.last_pos=a.pos; a.flaps=a.kind.gear_flap_fraction(); }
        // Only the airfield sortie starts on the ground, gear down.
        self.gear_lever = a.on_ground;
        self.flaps_switch = false;
        a.fuel = a.kind.fuel_max() * self.config.fuel;
        if !a.on_ground {
            combat::load(&mut a, &self.combat.loadout);
        } else {
            a.stores_mass = self.combat.loadout.mass();
            a.mass = a.kind.empty_mass() + a.stores_mass + a.fuel;
        }
        if let Some(l) = lesson {
            tutorial::position(l, self.config.jet, &mut a);
            self.gear_lever = a.on_ground;
            combat::load(&mut a, &self.combat.loadout);
            if l==tutorial::Lesson::Weapons { self.combat.training_target(&a); }
            else { self.combat.sites.clear(); }
            if l==tutorial::Lesson::Landing {
                let (x,z)=terrain::runway_point(-terrain::RUNWAY_HALF_LENGTH+320.0,0.0);
                let (dx,dz)=terrain::runway_dir();
                self.mission.gates=vec![sim::Gate{pos:v3(x,terrain::base_elevation(),z),axis:v3(dx,0.0,dz),radius:100.0,passed:false}];
                self.mission.gate_index=0;
            }
            self.training = Some(tutorial::Training::new(l, &a, &self.combat));
        }
        self.mission.start_throttle = a.throttle;
        self.aircraft = Some(a);
        self.trigger_mouse = false;
        self.pilot = Pilot::default();
        self.warnings = warnings::Warnings::default();
        self.breaths = 0;
        self.state = State::Flying;
        self.accumulator = 0.0;
        self.transition_timer = 0.0;
        self.flash = 0.0;
        self.world.fx.clear();
        (self.ejection, self.eject_hold, self.eject_wreck) = (None, 0.0, false);
        self.kick = 0.0;
        self.hands = Hands { throttle: self.mission.start_throttle, ..Default::default() };
        self.airbrake = false;
        self.key_pitch = 0.0;
        self.limiters_off = false;
        self.falling = false;
        self.keys.clear();
        self.center_stick();
        (self.zoom_pad, self.zoom) = (false, 0.0);
        self.grab(true);
        let dogfight = format!("DOGFIGHT // {} BANDITS // T LOCK, X WEAPON", self.config.threats.bandits);
        self.flash_toast(
            match self.kind {
                MissionKind::Gates => "COLD RIVER // 10 GATES // MOUSE CAPTURED",
                MissionKind::Dogfight => &dogfight,
                MissionKind::Strike => "STRIKE // BRIDGE AND SA-6 // X WEAPON, T DESIGNATE",
                MissionKind::Airfield => "RUNWAY A // THROTTLE UP, ROTATE AT 150 KT, G GEAR UP",
                MissionKind::Free => "FREE FLIGHT // NOBODY SHOOTS BACK // STOP ON RWY A TO REARM",
            },
            3.4,
        );
        self.last_frame = date::now();
    }

    fn pause(&mut self) {
        if !matches!(self.state, State::Flying | State::Frozen) {
            return;
        }
        self.paused_was_frozen = self.state == State::Frozen;
        self.state = State::Paused;
        self.trigger_mouse = false;
        self.zoom_mouse = false;
        self.keys.clear();
        self.mouse = (0.0, 0.0);
        self.toast_timer = 0.0;
        self.grab(false);
        self.selected = 0;
    }

    fn resume(&mut self) {
        if self.controls_open || self.state != State::Paused {
            return;
        }
        self.state = if self.paused_was_frozen { State::Frozen } else { State::Flying };
        self.keys.clear();
        self.mouse = (0.0, 0.0);
        self.grab(true);
        self.last_frame = date::now();
        self.accumulator = 0.0;
    }

    /// P toggles a freeze-frame: the sim, effects and clock stop, the view stays
    /// where it is, and the mouse looks around without Alt.
    fn toggle_freeze(&mut self) {
        if self.multiplayer.active() { self.toggle_controls(None); return; }
        match self.state {
            State::Flying => {
                self.state = State::Frozen;
                self.keys.clear();
                self.toast_timer = 0.0;
            }
            State::Frozen => {
                self.state = State::Flying;
                self.keys.clear();
                self.toast_timer = 0.0;
                self.last_frame = date::now();
                self.accumulator = 0.0;
            }
            _ => {}
        }
    }

    fn go_menu(&mut self) {
        self.multiplayer.disconnect(&self.audio);
        self.training = None;
        self.opts.tutorial = None;
        self.state = State::Attract;
        self.attract_time = 0.0;
        self.keys.clear();
        self.center_stick();
        self.grab(false);
        self.selected = 0;
    }

    fn toggle_controls(&mut self, force: Option<bool>) {
        let open = force.unwrap_or(!self.controls_open);
        if open == self.controls_open {
            return;
        }
        self.controls_open = open;
        self.selected = 0;
        if open {
            self.controls_was_flying = matches!(self.state, State::Flying | State::Frozen | State::Paused);
            self.controls_was_frozen = self.state == State::Frozen
                || self.state == State::Paused && self.paused_was_frozen;
            if self.controls_was_flying {
                self.state = State::Paused;
                self.keys.clear();
                self.trigger_mouse = false;
                self.zoom_mouse = false;
                self.mouse = (0.0, 0.0);
                self.grab(false);
            }
        } else {
            if self.controls_was_flying {
                self.state = if self.controls_was_frozen { State::Frozen } else { State::Flying };
                self.accumulator = 0.0;
                self.last_frame = date::now();
                self.grab(true);
            }
            self.controls_was_flying = false;
        }
    }

    fn save(&self) {
        if !self.opts.ui_check && !self.opts.future_ui_check && !self.opts.layout_check && self.opts.net_report.is_none() { settings::save(&self.settings, &self.best); }
    }

    fn act(&mut self, a: Action) {
        if self.network_action(a) { return; }
        if self.multiplayer.active() && matches!(a,Action::Restart|Action::Start|Action::Launch) {
            self.flash_toast("LEAVE THE ONLINE ROOM TO START A NEW SORTIE",2.0); return;
        }
        if self.controls_open && !matches!(a, Action::Controls | Action::SensDown | Action::SensUp | Action::FovDown | Action::FovUp | Action::Pitch | Action::Invert | Action::PadLayout | Action::Fullscreen | Action::Performance | Action::Restart | Action::Menu | Action::Reset | Action::Rebind(..) | Action::ResetKeys) {
            return;
        }
        match a {
            Action::School => {
                self.training = None;
                self.opts.tutorial = None;
                self.state = State::School;
                self.grab(false);
                self.selected = 0;
            },
            Action::Lesson(l) => {
                self.kind = l.mission();
                self.config = combat::Config::for_jet(if l==tutorial::Lesson::Landing {MissionKind::Airfield}else{self.kind}, self.config.jet);
                if l != tutorial::Lesson::Weapons { self.config.threats.bandits = 0; }
                self.config.threats.sams = 0;
                self.config.threats.guns = 0;
                self.config.refill = false;
                if l==tutorial::Lesson::Landing { self.config.fuel=0.5; }
                self.opts.tutorial = Some(l);
                self.start_mission();
            },
            Action::Controls => self.toggle_controls(None),
            // Every sortie goes through its setup screen first.
            Action::Start if self.state == State::Attract => {
                self.state = State::Loadout;
                self.selected = 0;
            }
            Action::Start | Action::Restart | Action::Launch => {
                self.controls_open = false;
                self.controls_was_flying = false;
                self.start_mission();
            },
            Action::Station(i) => self.config.cycle(i),
            Action::Fuel => self.config.cycle_fuel(),
            Action::Menu => {
                self.controls_open = false;
                self.controls_was_flying = false;
                self.go_menu();
            },
            Action::Quit => window::order_quit(),
            Action::Mute => self.toggle_mute(),
            Action::SensDown | Action::SensUp if self.pad_active => {
                let step = if a == Action::SensUp { 0.1 } else { -0.1 };
                self.settings.expo = clamp(((self.settings.expo + step) * 10.0).round() / 10.0, 0.0, 1.0);
            }
            Action::SensDown => self.settings.sensitivity = clamp(((self.settings.sensitivity - 0.1) * 10.0).round() / 10.0, 0.5, 5.0),
            Action::SensUp => self.settings.sensitivity = clamp(((self.settings.sensitivity + 0.1) * 10.0).round() / 10.0, 0.5, 5.0),
            Action::FovDown => self.settings.fov = clamp(self.settings.fov - 1.0, settings::FOV_MIN, settings::FOV_MAX),
            Action::FovUp => self.settings.fov = clamp(self.settings.fov + 1.0, settings::FOV_MIN, settings::FOV_MAX),
            Action::Pitch => {
                self.settings.pitch = if self.settings.pitch == PitchSource::Mouse { PitchSource::Keys } else { PitchSource::Mouse };
                self.key_pitch = 0.0;
            }
            Action::Invert => {
                self.settings.invert_pitch = !self.settings.invert_pitch;
                self.key_pitch = 0.0;
            }
            Action::PadLayout => {
                self.settings.pad_layout = if self.settings.pad_layout == PadLayout::Combat { PadLayout::Classic } else { PadLayout::Combat };
            }
            Action::Fullscreen => self.toggle_fullscreen(),
            Action::Performance => {
                self.settings.performance_overlay = !self.settings.performance_overlay;
                self.performance.set_enabled(self.settings.performance_overlay);
                self.save();
            },
            Action::Pick(k) => {
                self.training = None;
                self.opts.tutorial = None;
                self.kind = k;
                let (jet, aggressor, mixed, refill) = (self.config.jet, self.config.aggressor, self.config.threats.mixed, self.config.refill);
                self.config = combat::Config { aggressor, refill, ..combat::Config::for_jet(k, jet) };
                self.config.threats.mixed = mixed;
                self.state = State::Loadout;
                self.selected = 0;
            }
            Action::Jet => {
                if !self.multiplayer.active() {
                    self.config.set_jet(self.config.jet.next());
                    if let Some(s)=&mut self.multiplayer.session { let _=s.set_ready(false,self.config.jet.index()as u8); }
                }
            },
            Action::Aggressor => self.config.next_fleet(),
            Action::Layout => {
                let t = &self.config.threats;
                self.config.threats = scenario::Threats::with_layout(t.layout.next(), self.kind, t.mixed);
            }
            // The counts wrap round; a dogfight always has someone to fight.
            Action::Bandits => {
                let t = &mut self.config.threats;
                t.bandits = (t.bandits + 1) % (scenario::MAX_BANDITS + 1);
                if self.kind == MissionKind::Dogfight {
                    t.bandits = t.bandits.max(1);
                }
            }
            Action::Sams => self.config.threats.sams = (self.config.threats.sams + 1) % (scenario::MAX_SAMS + 1),
            Action::Guns => self.config.threats.guns = (self.config.threats.guns + 1) % (scenario::MAX_GUNS + 1),
            Action::Clock => self.settings.clock = !self.settings.clock,
            Action::Refill => self.config.refill = !self.config.refill,
            Action::Reset => {
                let fs = self.settings.fullscreen;
                let muted = self.settings.muted;
                let keys = self.settings.keys;
                let clock = self.settings.clock;
                let tutorial_completed = self.settings.tutorial_completed;
                self.settings = Settings { fullscreen: fs, muted, clock, keys, tutorial_completed, ..Settings::default() };
                self.performance.set_enabled(self.settings.performance_overlay);
            }
            Action::Rebind(i, slot) => {
                self.rebinding = Some((i as usize, slot as usize));
                self.flash_toast("PRESS A KEY // ESC CANCELS, BACKSPACE CLEARS", 4.0);
            }
            Action::ResetKeys => self.settings.keys = keymap::Keymap::default(),
            _ => {},
        }
        if matches!(a, Action::SensDown | Action::SensUp | Action::FovDown | Action::FovUp | Action::Pitch | Action::Invert | Action::PadLayout | Action::Clock | Action::Reset | Action::ResetKeys) {
            self.mouse = (0.0, 0.0);
            self.save();
        }
    }

    fn toggle_mute(&mut self) {
        if !self.audio.available {
            self.flash_toast("NO AUDIO DEVICE", 1.6);
            return;
        }
        self.settings.muted = !self.settings.muted;
        if matches!(self.state, State::Flying | State::Frozen | State::Paused | State::Transition) {
            self.flash_toast(if self.settings.muted { "AUDIO MUTED" } else { "AUDIO ON" }, 1.4);
        }
        self.save();
    }

    fn toggle_fullscreen(&mut self) {
        self.settings.fullscreen = !self.settings.fullscreen;
        window::set_fullscreen(self.settings.fullscreen);
        if !self.settings.fullscreen {
            window::set_window_size(1280, 720);
        }
        self.save();
    }

    fn begin_finish(&mut self, kind: Outcome) {
        if self.state != State::Flying {
            return;
        }
        if kind == Outcome::ShotDown {
            // The jet comes apart in the air: it stops being flown, and the
            // outside camera circles the fireball, or follows what is left
            // of it down when a wing or the tail was shot away.
            let lost = self.combat.player_lost;
            let a = self.aircraft.as_mut().unwrap();
            if lost.any() {
                a.lose(lost);
                self.falling = true;
            } else {
                a.crashed = true;
                a.impact_speed = a.speed;
                a.collision_label = "shot down";
            }
        }
        let a = self.aircraft.as_ref().unwrap();
        self.mission.score = if self.kind == MissionKind::Gates { self.mission.final_score() } else { self.combat.score(&self.mission) };
        let (done, total) = self.combat.tally();
        self.result = Some(RunResult { kind, time: self.mission.elapsed, gates: self.mission.passed, score: self.mission.score, impact: if a.crashed { a.impact_speed } else { a.speed }, mission: self.kind, done, total, cause: self.combat.death });
        self.transition_kind = kind;
        self.transition_timer = match kind {
            Outcome::Win => 1.35,
            // A broken jet falls until it hits (or for at most this long),
            // then the fireball plays as after any impact.
            Outcome::Crash | Outcome::ShotDown if self.falling => 20.0,
            // Long enough to watch the fireball from the wreck camera.
            Outcome::Crash | Outcome::ShotDown => 2.6,
            Outcome::Timeout => 0.85,
        };
        self.state = State::Transition;
        self.keys.clear();
        self.mouse = (0.0, 0.0);
        self.grab(false);
        match kind {
            Outcome::Win => {
                self.flash_color = [210, 255, 180];
                self.flash = 0.6;
                self.flash_toast("COURSE COMPLETE", 2.0);
                self.audio.tone(520.0, 0.18, 0.04, Wave::Square, 0.0);
                self.audio.tone(780.0, 0.26, 0.04, Wave::Square, 0.17);
                self.audio.tone(1040.0, 0.36, 0.035, Wave::Square, 0.37);
            }
            Outcome::Crash if self.falling => {
                self.flash_color = [255, 150, 70];
                self.flash = 0.5;
                self.toast_timer = 0.0;
                self.kick = 0.8;
                self.crash_time = self.time;
                self.audio.burst(0.9, 3.0, 900.0, 60.0, 0.0);
            }
            Outcome::Crash => {
                let a = self.aircraft.as_ref().unwrap();
                self.flash_color = if a.impact_water { [200, 232, 255] } else { [255, 100, 58] };
                self.flash = 1.0;
                self.toast_timer = 0.0;
                self.kick = 1.0;
                self.crash_time = self.time;
                self.audio.crash(a.impact_water);
                self.world.fx.explode(a);
            }
            Outcome::ShotDown if self.ejection.is_some() => {}
            Outcome::ShotDown => {
                self.flash_color = [255, 100, 58];
                self.flash = 0.8;
                self.toast_timer = 0.0;
                self.kick = 1.0;
                self.crash_time = self.time;
                self.audio.crash(false);
            }
            Outcome::Timeout => {
                self.flash_color = [255, 170, 70];
                self.flash = 0.45;
                self.flash_toast("TIME EXPIRED", 2.0);
                self.audio.warning(self.config.jet, warnings::Cue::LowSpeed);
            }
        }
    }

    fn complete_transition(&mut self) {
        if let Some(r) = self.result {
            // An untimed run is not a record: it has no clock to beat.
            if r.kind == Outcome::Win && r.mission == MissionKind::Gates && self.mission.timed {
                if r.score > self.best.score {
                    self.best.score = r.score.round();
                }
                if r.time > 0.0 && (self.best.time == 0.0 || r.time < self.best.time) {
                    self.best.time = r.time;
                }
                self.save();
            }
        }
        self.state = State::Result;
        self.selected = 0;
    }

    fn ejection_chord(&self) -> bool {
        ejection_requested(&self.settings.keys, &self.keys, &self.pad, self.opts.eject)
    }

    fn flight_controls_blocked(&self) -> bool {
        self.ejection.is_some() || self.ejection_chord()
    }

    fn read_input(&self) -> Input {
        if self.opts.net_report.is_some() { return self.opts.input; }
        let mut input = self.read_control_positions();
        if self.settings.invert_pitch {
            input.pitch_axis = -input.pitch_axis;
        }
        input
    }

    /// Control positions before pitch inversion, for the on-screen stick cue.
    fn read_control_positions(&self) -> Input {
        if self.flight_controls_blocked() {
            return Input { airbrake: self.airbrake, limiters_off: self.limiters_off,
                gear_down: self.gear_lever, flaps: self.flaps_switch, ..Default::default() };
        }
        let km = &self.settings.keys;
        let k = |b: Bind| km.held(b, &self.keys);
        let mouse = if self.grabbed { self.mouse } else { (0.0, 0.0) };
        // Exactly one pitch source is live: mouse Y (down = pull) or W/S.
        let pitch_axis = match self.settings.pitch {
            PitchSource::Mouse => stick_curve(mouse.1),
            PitchSource::Keys => self.key_pitch,
        };
        // The controller adds to the keyboard and mouse and reads zero at
        // rest. Pulling the left stick back pulls the nose up. Flight input
        // applies inversion later.
        let pad = &self.pad;
        let on_ground = self.aircraft.as_ref().is_some_and(|a| a.on_ground);
        let e = self.settings.expo;
        let (lx, ly) = (settings::stick_expo(pad.state.left.0, e), settings::stick_expo(pad.state.left.1, e));
        let (roll, yaw, up, down, burner, brake_clear) = match self.settings.pad_layout {
            // LB / LT run the throttle (held at the stop, LB lights the
            // burner) and the bumpers are free of the rudder: holding X turns
            // the stick's X axis into rudder, and on the ground it also steers
            // the nose wheel.
            PadLayout::Combat => {
                let rudder = pad.held(gamepad::X);
                let roll = if rudder { 0.0 } else { lx };
                let yaw = if rudder || on_ground { lx } else { 0.0 };
                (roll, yaw, pad.held(gamepad::LB), pad.state.lt > 0.3, false, !pad.held(gamepad::LB))
            }
            // RT / LT run the throttle, RT to the stop asks for afterburner,
            // and LB / RB work the rudder.
            PadLayout::Classic => {
                let yaw = pad.held(gamepad::RB) as i32 as f64 - pad.held(gamepad::LB) as i32 as f64;
                (lx, yaw, pad.state.rt > 0.3, pad.state.lt > 0.3, pad.state.rt > 0.9, pad.state.rt < 0.05)
            }
        };
        Input {
            roll_left: k(Bind::RollLeft),
            roll_right: k(Bind::RollRight),
            yaw_left: k(Bind::YawLeft),
            yaw_right: k(Bind::YawRight),
            pitch_axis: pitch_axis - ly,
            roll_axis: roll,
            yaw_axis: clamp(stick_curve(mouse.0) + yaw, -1.0, 1.0),
            throttle_up: k(Bind::ThrottleUp) || up,
            throttle_down: k(Bind::ThrottleDown) || down,
            afterburner: k(Bind::Afterburner) || burner,
            airbrake: self.airbrake,
            limiters_off: self.limiters_off,
            gear_down: self.gear_lever,
            flaps: self.flaps_switch,
            wheel_brake: k(Bind::WheelBrake) || pad.state.lt > 0.3 && brake_clear && on_ground,
            ..Default::default()
        }
    }

    /// Ramps the keyboard pitch: full deflection after 0.11 s held, back to
    /// neutral in under 0.08 s, so taps make small corrections.
    fn ramp_key_pitch(&mut self, dt: f64) {
        let k = |b: Bind| self.settings.keys.held(b, &self.keys);
        let want = if self.settings.pitch == PitchSource::Keys {
            k(Bind::PitchUp) as i32 as f64 - k(Bind::PitchDown) as i32 as f64
        } else {
            0.0
        };
        let rate = if want != 0.0 && want * self.key_pitch >= 0.0 { 9.0 } else { 14.0 };
        self.key_pitch = approach(self.key_pitch, want, rate * dt);
    }

    /// Controller buttons go through the keyboard handlers, so menus and
    /// flight behave the same whichever is used.
    fn pad_buttons(&mut self) {
        use gamepad::{A, B, BACK, DOWN, LEFT, LS, RB, RIGHT, RS, START, UP, X, Y};
        let menu = self.controls_open || matches!(self.state, State::Attract | State::Paused | State::Result | State::Loadout | State::School | State::Network);
        if !menu && self.ejection_chord() {
            self.zoom_pad = false;
            let weapon = if self.settings.pad_layout == PadLayout::Combat { gamepad::A } else { gamepad::UP };
            self.pad_jettisoned = self.pad.held(weapon);
            return;
        }
        if matches!(self.state, State::Attract | State::Loadout) && !self.controls_open && self.pad.pressed(START) {
            self.act(if self.state == State::Loadout { Action::Launch } else { Action::Start });
            return;
        }
        // Menu buttons go through the fixed menu keys; flight buttons go
        // straight to their actions, whatever keys those are bound to.
        let binds = if menu {
            let start = if self.controls_open { KeyCode::Escape } else if self.state == State::Result { KeyCode::R } else { KeyCode::P };
            vec![(A, KeyCode::Enter), (B, KeyCode::Escape), (START, start), (BACK, KeyCode::K)]
        } else {
            let mut v = vec![(START, KeyCode::P), (BACK, KeyCode::K)];
            if self.state == State::Frozen {
                v.push((B, KeyCode::Escape));
            }
            v
        };
        // Zoom lasts only while the right stick is held in.
        self.zoom_pad = !menu && self.pad.held(RS);
        let nav = if menu { self.pad.nav() } else { 0 };
        let mut keys: Vec<KeyCode> = [(UP, KeyCode::Up), (DOWN, KeyCode::Down), (LEFT, KeyCode::Left), (RIGHT, KeyCode::Right)].into_iter().filter(|&(b, _)| nav & b != 0).map(|(_, k)| k).collect();
        keys.extend(binds.into_iter().filter(|&(b, _)| self.pad.pressed(b) && !(self.multiplayer.session.is_some() && b == BACK)).map(|(_, k)| k));
        let pad_navigation = !keys.is_empty();
        self.pad_dispatch = true;
        for k in keys {
            self.key_down_event(k, KeyMods::default(), false);
        }
        self.pad_dispatch = false;
        if pad_navigation { self.pad_active = true; }
        if !menu {
            let airbrake = if self.state == State::Frozen { None } else { Some((B, Bind::Airbrake)) };
            let actions: &[(u16, Bind)] = match self.settings.pad_layout {
                // Fire (RT), lock (RB) and countermeasures (L3) never take a
                // thumb off a stick; the d-pad and face buttons are for what
                // can wait.
                PadLayout::Combat => &[(Y, Bind::View), (A, Bind::Weapon), (RB, Bind::Lock), (LS, Bind::Countermeasures), (LEFT, Bind::LeftMfd), (RIGHT, Bind::RightMfd), (DOWN, Bind::Gear), (UP, Bind::Flaps)],
                PadLayout::Classic => &[(Y, Bind::View), (X, Bind::Gear), (LEFT, Bind::LeftMfd), (RIGHT, Bind::RightMfd), (UP, Bind::Weapon), (DOWN, Bind::Lock), (LS, Bind::Countermeasures)],
            };
            // Both sticks clicked together: the limiter override, which then
            // drops no countermeasures.
            let both = self.pad.pressed(LS) && self.pad.held(RS) || self.pad.pressed(RS) && self.pad.held(LS);
            if both {
                self.toggle_limiters();
            }
            // The MiG has no MFDs: its left d-pad works the radar / IRST switch.
            let mig = self.aircraft.as_ref().is_some_and(|a| a.kind.has_irst());
            // The weapon button cycles on release, so that holding it
            // jettisons the store selected now rather than the next one.
            let weapon = if self.settings.pad_layout == PadLayout::Combat { A } else { UP };
            if !self.pad.held(weapon) && !self.pad.released(weapon) {
                self.pad_jettisoned = false;
            }
            for (button, b) in actions.iter().copied().chain(airbrake) {
                let b = if mig && b == Bind::LeftMfd { Bind::Sensor } else { b };
                if b == Bind::Weapon {
                    continue;
                }
                if self.pad.pressed(button) && !(both && button == LS) {
                    self.bind_pressed(b);
                }
            }
            if self.state == State::Flying && self.pad.long_press(weapon, JETTISON_HOLD) {
                self.pad_jettisoned = true;
                self.bind_pressed(Bind::Jettison);
            }
            if self.pad.released(weapon) {
                if !self.pad_jettisoned {
                    self.bind_pressed(Bind::Weapon);
                }
                self.pad_jettisoned = false;
            }
            // Holding the lock button half a second drops the lock, like TMS aft.
            if self.settings.pad_layout == PadLayout::Combat && self.state == State::Flying && self.pad.long_press(RB, 0.5) {
                if self.multiplayer.active() {self.multiplayer.drop_lock=self.multiplayer.drop_lock.wrapping_add(1);}
                else {self.combat.drop_lock();}
            }
        }
    }

    /// The right stick turns the head in the cockpit or orbits the chase
    /// camera, and springs back to the forward view when released.
    fn update_pad_look(&mut self, dt: f64) {
        let (x, y) = self.pad.state.right;
        let r = 1.0 - (-dt * 10.0).exp();
        self.pad_look.0 += (x - self.pad_look.0) * r;
        self.pad_look.1 += (y - self.pad_look.1) * r;
        if x == 0.0 && y == 0.0 && self.pad_look.0.abs() + self.pad_look.1.abs() < 0.002 {
            if self.pad_look != (0.0, 0.0) {
                self.pad_look = (0.0, 0.0);
                self.look = (0.0, LOOK_PITCH);
                self.chase_look = (0.0, 0.0);
            }
            return;
        }
        let (lx, ly) = self.pad_look;
        self.look = (lx * 150.0 * DEG, clamp(LOOK_PITCH + ly * 75.0 * DEG, -85.0 * DEG, 85.0 * DEG));
        self.chase_look = (-lx * PI, -ly * 60.0 * DEG);
    }

    /// While frozen the right stick pans the view and leaves it there, so a
    /// framed shot holds without keeping the stick deflected.
    fn update_frozen_look(&mut self, dt: f64) {
        let (x, y) = self.pad.state.right;
        self.pad_look = (0.0, 0.0);
        if self.view == View::Cockpit {
            self.look.0 = clamp(self.look.0 + x * 1.6 * dt, -155.0 * DEG, 155.0 * DEG);
            self.look.1 = clamp(self.look.1 + y * 1.2 * dt, -85.0 * DEG, 85.0 * DEG);
        } else {
            self.chase_look.0 -= x * 1.6 * dt;
            self.chase_look.1 = clamp(self.chase_look.1 - y * 1.2 * dt, -65.0 * DEG, 65.0 * DEG);
        }
    }

    fn step(&mut self, dt: f64, input: Input) {
        if self.multiplayer.active() { self.network_step(dt,input); return; }
        let blocked = self.flight_controls_blocked();
        let input = if blocked { self.read_input() } else { input };
        let m = &mut self.mission;
        let a = self.aircraft.as_mut().unwrap();
        m.elapsed += dt;
        if m.timed {
            m.time_left = (m.time_left - dt).max(0.0);
        }
        // Nobody flies a jet the pilot has left.
        let input = if blocked { input } else { self.pilot.controls(a, input) };
        let broken = a.gear_broken;
        a.update(dt, &input);
        let mut eject_toast = None;
        if let Some(e) = &mut self.ejection {
            let toast = e.step(dt, a);
            if e.rocket() {
                self.world.fx.seat_rocket(e.pilot.pos - e.pilot.q.axes().up * 0.75, e.pilot.vel, e.pilot.q.axes().up);
            }
            if let Some(t) = toast {
                if e.phase == eject::Phase::Seat {
                    self.audio.burst(1.1, 3.2, 650.0, 110.0, 0.0);
                    self.kick = self.kick.max(0.9);
                }
                eject_toast = Some(t);
            }
        }
        // A leg folding under a hard arrival, and the airframe scraping the
        // runway in sparks.
        let folded = (0..3).find(|&i| a.gear_broken[i] && !broken[i]);
        let sparks = (a.skid > 3.0 && !a.crashed && (m.elapsed * 30.0).fract() < dt * 30.0).then_some(a.skid_at);
        let (glocs, gcas) = (self.pilot.glocs, self.pilot.gcas);
        self.pilot.update(dt, a);
        if let Some(i) = folded {
            self.flash_toast(&format!("{} DETACHED", sim::WHEELS[i].name.to_uppercase()), 2.4);
            self.kick = self.kick.max(0.6);
            self.audio.burst(0.5, 2.2, 700.0, 60.0, 0.0);
        }
        if let Some(at) = sparks {
            self.world.fx.impact(at, fx::Impact::Metal);
        }
        if let Some(t) = eject_toast {
            self.flash_toast(t, 2.4);
        }
        if self.pilot.glocs > glocs {
            self.flash_toast("G-LOC // PILOT UNCONSCIOUS", 3.0);
        } else if self.pilot.gcas && !gcas {
            self.flash_toast("AUTO GCAS // FLY UP", 2.4);
        }
        let m = &mut self.mission;
        let a = self.aircraft.as_mut().unwrap();
        m.score = m.final_score();
        let rings = self.training.is_none() && matches!(self.kind, MissionKind::Gates | MissionKind::Free);
        if rings && m.gate_index < m.gates.len() {
            let gi = m.gate_index;
            if gate_crossed(a, &m.gates[gi]) {
                m.gates[gi].passed = true;
                m.passed += 1;
                m.gate_index += 1;
                if m.timed {
                    m.time_left += GATE_BONUS;
                }
                m.score = m.final_score();
                // The ring just flown, which free flight counts again on each lap.
                let passed = gi + 1;
                let done = m.gate_index >= m.gates.len();
                let timed = m.timed;
                // Free flight runs the rings again from the first.
                if done && self.kind == MissionKind::Free {
                    m.gate_index = 0;
                    for g in m.gates.iter_mut() {
                        g.passed = false;
                    }
                }
                self.flash_color = [185, 255, 170];
                self.flash = 0.36;
                let clear = if timed { format!("GATE {:02} CLEAR // +{} SEC", passed, GATE_BONUS) } else { format!("GATE {:02} CLEAR", passed) };
                self.flash_toast(&clear, 2.3);
                self.audio.gate();
                if done && self.kind == MissionKind::Gates {
                    self.begin_finish(Outcome::Win);
                    return;
                }
            }
        }
        if self.fight(dt) {
            return;
        }
        self.step_training(dt, input);
        if self.state != State::Flying { return; }
        if self.ejection.is_some() {
            self.after_ejection();
            return;
        }
        // A wing that broke off in flight: the pieces fly and the rest falls.
        let a = self.aircraft.as_ref().unwrap();
        if a.lost.any() && !a.crashed {
            self.combat.break_up(a, a.lost);
            let what = a.structure.failed.unwrap_or("airframe");
            self.falling = true;
            self.begin_finish(Outcome::Crash);
            self.flash_toast(&format!("{} // STRUCTURAL FAILURE", what.to_uppercase()), 2.6);
            return;
        }
        if a.crashed {
            self.begin_finish(Outcome::Crash);
            return;
        }
        if self.mission.timed && self.mission.time_left <= 0.0 {
            self.begin_finish(Outcome::Timeout);
        }
    }

    fn step_training(&mut self, dt: f64, input: Input) {
        let Some(training) = &mut self.training else { return };
        let a = self.aircraft.as_ref().unwrap();
        let changed = training.step(dt, &tutorial::Observation {
            jet: a, combat: &self.combat, input,
            looking: self.look.0.abs() > 8.0 * DEG || (self.look.1 - LOOK_PITCH).abs() > 8.0 * DEG,
            external_view: self.view == View::Chase,
        });
        if training.done() {
            self.settings.tutorial_completed |= 1 << training.lesson.index();
            self.save();
            self.begin_finish(Outcome::Win);
        } else if changed {
            self.audio.gate();
        }
    }

    /// Trigger and pickle: F, the left mouse button while flying, the pad's RT
    /// (A in the classic layout), or --input fire.
    fn trigger(&self) -> bool {
        let pad = match self.settings.pad_layout {
            PadLayout::Combat => self.pad.state.rt > 0.5,
            PadLayout::Classic => self.pad.held(gamepad::A),
        };
        self.settings.keys.held(Bind::Trigger, &self.keys) || self.trigger_mouse || pad || self.opts.fire
    }

    /// One chaff and flare program, with the cartridges left.
    fn dispense(&mut self) {
        if let Some(a) = &self.aircraft {
            self.combat.countermeasures(a);
            let l = &self.combat.loadout;
            self.flash_toast(&format!("CHAFF + FLARES  //  CHAFF {}  FLARES {}", l.chaff, l.flares), 1.2);
        }
    }

    /// Pulls the handle: the canopy goes and the seat sequence starts.
    fn eject(&mut self) {
        let Some(a) = self.aircraft.as_mut() else { return };
        if self.ejection.is_some() || a.ejected {
            return;
        }
        a.ejected = true;
        let e = eject::Ejection::new(a);
        self.flash_toast(&format!("EJECT // {}", e.spec.name), 2.0);
        self.audio.burst(0.5, 2.6, 1800.0, 260.0, 0.0);
        self.kick = self.kick.max(0.5);
        self.flash_color = [255, 245, 220];
        self.flash = self.flash.max(0.25);
        self.ejection = Some(e);
    }

    /// The jet the pilot left flies on alone and comes down; the run ends a
    /// few seconds after the pilot is on the ground.
    fn after_ejection(&mut self) {
        let a = self.aircraft.as_ref().unwrap();
        if a.lost.any() && !a.crashed && !self.falling {
            self.combat.break_up(a, a.lost);
            self.falling = true;
        }
        let a = self.aircraft.as_ref().unwrap();
        if a.crashed && !self.eject_wreck {
            self.eject_wreck = true;
            self.crash_time = self.time;
            self.audio.crash(a.impact_water);
            self.world.fx.explode(a);
        }
        let e = self.ejection.as_ref().unwrap();
        if e.landed_for > 3.0 || e.age > 300.0 {
            if !self.combat.player_dead {
                self.combat.death = "EJECTED";
            }
            self.combat.player_lost = Default::default();
            self.falling = false;
            self.begin_finish(Outcome::ShotDown);
        }
    }

    /// The combat step after the jet has moved: weapons, bandits, targets
    /// and their effects. Returns true when it ended the run.
    fn fight(&mut self, dt: f64) -> bool {
        // A pilot out cold, or still coming round, cannot squeeze the trigger.
        let trigger = self.trigger() && self.pilot.authority() > 0.5 && !self.flight_controls_blocked();
        // The handle, held by a pilot awake to pull it: half a second on the
        // key, both bumpers for a second and a half on the pad.
        let pad_pull = self.pad.held(gamepad::LB) && self.pad.held(gamepad::RB);
        let pull = (self.settings.keys.held(Bind::Eject, &self.keys) || self.opts.eject || pad_pull) && self.pilot.authority() > 0.5;
        self.eject_hold = if pull && self.ejection.is_none() { self.eject_hold + dt } else { 0.0 };
        if self.eject_hold >= if pad_pull { 1.5 } else { 0.5 } {
            self.eject();
        }
        // Held down, the countermeasure button runs another program every
        // CM_REPEAT seconds; both sticks clicked together is the limiter override.
        let cm = !self.flight_controls_blocked() && (self.settings.keys.held(Bind::Countermeasures, &self.keys) || self.pad.held(gamepad::LS) && !self.pad.held(gamepad::RS));
        self.cm_hold = if cm { self.cm_hold + dt } else { 0.0 };
        if self.cm_hold >= CM_REPEAT {
            self.cm_hold -= CM_REPEAT;
            self.dispense();
        }
        let a = self.aircraft.as_mut().unwrap();
        self.combat.step(dt, a, trigger, &mut self.mission);
        let fb = props::present(&mut self.combat, &mut self.world.fx, &self.audio, self.camera.eye, self.settings.muted);
        if let Some(t) = fb.toast {
            self.flash_toast(&t, 2.0);
        }
        if let Some(c) = fb.flash {
            self.flash_color = c;
            self.flash = self.flash.max(0.3);
        }
        self.kick = self.kick.max(fb.kick * 0.5);
        if self.kind != MissionKind::Gates {
            self.mission.score = self.combat.score(&self.mission);
        }
        if self.combat.player_dead {
            if self.ejection.is_some() {
                return false;
            }
            // Shot down with the pilot alive: the jet comes apart and he
            // goes out of it.
            if self.combat.death != "PILOT KILLED" {
                let lost = self.combat.player_lost;
                let a = self.aircraft.as_mut().unwrap();
                if lost.any() {
                    a.lose(lost);
                    self.falling = true;
                } else {
                    a.crashed = true;
                    a.impact_speed = a.speed;
                    a.collision_label = "shot down";
                    self.eject_wreck = true;
                    self.crash_time = self.time;
                    self.audio.crash(false);
                }
                self.flash_color = [255, 100, 58];
                self.flash = 0.8;
                self.eject();
                return false;
            }
            self.begin_finish(Outcome::ShotDown);
            return true;
        }
        if self.combat.won() {
            self.begin_finish(Outcome::Win);
            return true;
        }
        false
    }

    /// --input auto: the AI pilot flies the player's jet toward a point
    /// 250 m over the steerpoint, for scripted screenshots of a mission.
    fn autopilot(&mut self) -> Input {
        let Some(a) = &self.aircraft else { return Input::default() };
        let mut mark = a.clone();
        mark.pos = self.mission.current_gate().pos + V3::Y * 250.0;
        mark.vel = V3::ZERO;
        let pic = ai::Picture { foe: &mark, missile: None, missiles: 0, missile_spec: &weapons::AIM9M, rounds: 0, tactical: ai::Tactical { combat: false, ..Default::default() } };
        let pilot = self.auto_pilot.get_or_insert_with(|| ai::Pilot::new(0.6));
        pilot.fly(a, &pic, FIXED_DT).input
    }

    /// Objective line for the overlays in a combat mission.
    fn objective(&self) -> Option<Objective> {
        if let Some(b)=&self.multiplayer.battle { return Some(Objective {label:b.objective.clone(),score:0.0,cue:"MISSION"}); }
        let (done, total) = self.combat.tally();
        match self.kind {
            MissionKind::Gates => None,
            MissionKind::Dogfight => Some(Objective { label: format!("KILLS {done}/{total}"), score: self.combat.score(&self.mission), cue: "BULLSEYE" }),
            MissionKind::Strike => Some(Objective { label: format!("TARGETS {done}/{total}"), score: self.combat.score(&self.mission), cue: "STPT" }),
            MissionKind::Airfield => Some(Objective { label: ["TAKE OFF", "STPT 1", "LAND RWY A", "STOPPED"][done.min(3)].to_string(), score: self.combat.score(&self.mission), cue: if done == 0 { "RWY" } else { "STPT" } }),
            MissionKind::Free => Some(Objective { label: format!("KILLS {done}  TGTS {total}"), score: self.combat.score(&self.mission), cue: "GATE" }),
        }
    }

    fn update_hands(&mut self, dt: f64, input: Option<Input>) {
        let active = self.state == State::Flying && self.aircraft.as_ref().is_some_and(|a| !a.crashed);
        let (mut roll, mut pitch, mut rudder) = (0.0, 0.0, 0.0);
        if active {
            let i = self.pilot.controls(self.aircraft.as_ref().unwrap(), input.unwrap_or_else(|| self.read_input()));
            roll = i.roll();
            pitch = i.pitch();
            rudder = i.yaw();
        }
        let r = 1.0 - (-dt.max(0.0) * 11.0).exp();
        let tr = 1.0 - (-dt.max(0.0) * 7.0).exp();
        let rr = 1.0 - (-dt.max(0.0) * 8.0).exp();
        let pull_time = if self.pad.held(gamepad::LB) && self.pad.held(gamepad::RB) { 1.5 } else { 0.5 };
        self.hands.eject = if self.ejection.as_ref().is_some_and(|e| e.phase == eject::Phase::Canopy) { 1.0 }
            else if self.ejection_chord() { (self.eject_hold / pull_time).min(1.0) } else { 0.0 };
        let h = &mut self.hands;
        h.roll += (roll - h.roll) * r;
        h.pitch += (pitch - h.pitch) * r;
        h.rudder += (rudder - h.rudder) * rr;
        let throttle = if active { self.aircraft.as_ref().unwrap().throttle } else { self.hands.throttle };
        self.hands.throttle += (throttle - self.hands.throttle) * tr;
        // The gear handle snaps over in a quarter second; its light burns
        // while the gear runs.
        let lever = if self.gear_lever { 1.0 } else { 0.0 };
        self.hands.gear = approach(self.hands.gear, lever, dt.max(0.0) * 4.0);
        self.hands.gear_warn = self.aircraft.as_ref().is_some_and(|a| a.gear > 0.0 && a.gear < 1.0);
    }

    // ------------------------------------------------------------ cameras
    fn attract_camera(&self, t: f64) -> Camera {
        let travel = (t * 72.0) % 18500.0;
        let z = 9300.0 - travel;
        let x = terrain::river_x(z) + (t * 0.14).sin() * 90.0;
        let az = z - 900.0;
        let axx = terrain::river_x(az) + (t * 0.14 + 0.7).sin() * 140.0;
        let ground = terrain::height(x, z);
        let ag = terrain::height(axx, az);
        let y = (ground + 235.0).max(470.0) + (t * 0.27).sin() * 38.0;
        let ay = (ag + 270.0).max(500.0);
        let fwd = v3(axx - x, (ay - y) * 0.22, az - z).norm();
        let base = Quat::from_forward_up(fwd, V3::Y);
        let bank = (t * 0.095).sin() * 0.13 + (t * 0.037).sin() * 0.05;
        let ax = base.axes();
        let rq = Quat::axis_angle(ax.forward, bank);
        Camera { eye: v3(x, y, z), forward: rq.rotate(ax.forward), right: rq.rotate(ax.right), up: rq.rotate(ax.up), fov: 61.0 * DEG }
    }

    fn shake(&self, a: &Aircraft) -> f64 {
        // The wing mode shakes the whole airframe, the seat included, and the
        // airframe buzzes as the equivalent airspeed climbs towards the placard.
        let mut s = 0.00055 + a.turbulence * 0.0012 + a.buffet * 0.004 + a.afterburner_visual * 0.0007 + a.structure.amplitude * 0.05;
        s += 0.0016 * smoothstep(0.6, 1.0, a.structure.margin);
        if a.mach > 0.92 {
            s += 0.0007;
        }
        s + self.kick * 0.02
    }

    /// After the flight has ended: the wrecks and broken-off pieces fall,
    /// and a player's jet that came apart in the air falls with nobody
    /// flying it until it hits, when it blows up as any crash does.
    fn fall(&mut self, dt: f64) {
        let Some(a) = self.aircraft.as_mut() else { return };
        if self.falling {
            a.update(dt, &Input::default());
        }
        self.combat.aftermath(dt, a);
        let fb = props::present(&mut self.combat, &mut self.world.fx, &self.audio, self.camera.eye, self.settings.muted);
        self.kick = self.kick.max(fb.kick * 0.5);
        let a = self.aircraft.as_ref().unwrap();
        if self.falling && a.crashed {
            self.falling = false;
            self.flash_color = if a.impact_water { [200, 232, 255] } else { [255, 100, 58] };
            self.flash = 1.0;
            self.kick = 1.0;
            self.audio.crash(a.impact_water);
            self.world.fx.explode(a);
            self.transition_timer = self.transition_timer.min(2.6);
            if let Some(r) = self.result.as_mut() {
                r.impact = a.impact_speed;
            }
        }
    }

    fn advance_effects(&mut self, dt: f64) {
        self.world.fx.update(dt);
        let flying = self.aircraft.as_ref().filter(|_| matches!(self.state, State::Flying | State::Transition));
        self.world.fx.vapor.update(dt, flying);
        self.kick *= (-dt * 2.2).exp();
    }

    /// After an impact the cockpit is gone: an external camera backs away
    /// from the wreck and slowly circles it.
    fn wreck_camera(&self, a: &Aircraft) -> Camera {
        let t = (self.time - self.crash_time).max(0.0);
        let course = v3(a.vel.x, 0.0, a.vel.z);
        let back = if course.len() > 1e-6 { -course.norm() } else { a.q.axes().back };
        let pull_out = smoothstep(0.0, 2.5, t);
        let around = Quat::axis_angle(V3::Y, 0.55 + t * 0.12).rotate(back);
        // A jet still falling in pieces is followed close; the fireball
        // from further back.
        let near = if a.crashed { 1.0 } else { 0.4 };
        let target = a.pos + V3::Y * ((5.0 + 10.0 * pull_out) * near);
        let mut eye = target + around * ((60.0 + 55.0 * pull_out) * near) + V3::Y * ((14.0 + 18.0 * pull_out) * near);
        let shake = self.kick * 9.0;
        eye = eye + v3((t * 31.1).sin(), (t * 26.3 + 1.7).sin(), (t * 18.3).sin()) * shake;
        eye.y = eye.y.max(terrain::floor(eye.x, eye.z) + 3.0);
        let f = (target - eye).norm();
        let r = f.cross(V3::Y).norm();
        Camera { eye, forward: f, right: r, up: r.cross(f).norm(), fov: 55.0 * DEG }
    }

    fn flight_camera(&self, a: &Aircraft) -> Camera {
        let mut q = a.q;
        if matches!(self.state, State::Flying | State::Transition) {
            let t = self.time;
            let shake = self.shake(a);
            let sp = (t * 26.3).sin() * 0.6 + (t * 41.7 + 1.3).sin() * 0.4;
            let sr = (t * 31.1 + 2.1).sin() * 0.55 + (t * 18.3 + 0.2).sin() * 0.45;
            q = q.mul(Quat::axis_angle(V3::X, sp * shake));
            q = q.mul(Quat::axis_angle(V3::Y, (t * 17.3 + 0.4).sin() * shake * 0.65));
            q = q.mul(Quat::axis_angle(V3::Z, sr * shake * 1.1));
        }
        q = q.mul(Quat::axis_angle(V3::Y, -self.look.0));
        let reach = smoothstep(0.0, 0.20, self.hands.eject) * (1.0 - smoothstep(0.60, 0.95, self.hands.eject));
        let pitch = mix(self.look.1, self.look.1.min(-55.0 * DEG), reach);
        q = q.mul(Quat::axis_angle(V3::X, pitch));
        let ax = q.axes();
        // Head remains against the seat; the raised handle and clear arm path
        // make the grip visible without leaning into the ejection rails.
        let eye = cockpit::eye_world(a);
        Camera { eye, forward: ax.forward, right: ax.right, up: ax.up, fov: self.settings.fov * DEG * self.zoom_scale() }
    }

    /// After ejecting: from the pilot's helmet, looking down and ahead under
    /// the canopy, or from outside following the seat and then the chute.
    fn eject_camera(&self, e: &eject::Ejection) -> Camera {
        let p = &e.pilot;
        let hanging = matches!(e.phase, eject::Phase::Chute | eject::Phase::Landed);
        if self.view == View::Cockpit {
            if e.phase == eject::Phase::Canopy {
                return self.flight_camera(self.aircraft.as_ref().unwrap());
            }
            let reach = if e.phase == eject::Phase::Seat { 1.0 - smoothstep(0.0, 0.45, e.away().unwrap_or(0.0)) }
                else { 0.0 };
            let pitch = mix(self.look.1, self.look.1.min(-55.0 * DEG), reach)
                + if hanging { -25.0 * DEG } else { 0.0 };
            let q = p.q.mul(Quat::axis_angle(V3::Y, -self.look.0)).mul(Quat::axis_angle(V3::X, pitch));
            let ax = q.axes();
            let eye = p.pos + p.q.rotate(v3(0.0, 0.52, 0.0));
            return Camera { eye, forward: ax.forward, right: ax.right, up: ax.up, fov: self.settings.fov * DEG * self.zoom_scale() };
        }
        let course = v3(p.vel.x, 0.0, p.vel.z);
        let back = if course.len() > 1.0 { -course.norm() } else { p.q.axes().back };
        let center = p.pos + V3::Y * (0.4 + 3.6 * e.chute);
        let mut off = back * (7.0 + 9.0 * e.chute) + V3::Y * (1.5 + 2.5 * e.chute);
        off = Quat::axis_angle(V3::Y, self.chase_look.0).rotate(off);
        let side = off.cross(V3::Y).norm();
        off = Quat::axis_angle(side, -self.chase_look.1).rotate(off);
        let mut eye = center + off * self.opts.zoom.unwrap_or(1.0);
        eye.y = eye.y.max(terrain::floor(eye.x, eye.z) + 1.5);
        let f = (center - eye).norm();
        let r = f.cross(V3::Y).norm();
        Camera { eye, forward: f, right: r, up: r.cross(f).norm(), fov: 55.0 * DEG * self.zoom_scale() }
    }

    fn chase_camera(&self, a: &Aircraft) -> Camera {
        let ax = a.q.axes();
        let t = self.time;
        let speed_factor = clamp(a.speed / 380.0, 0.0, 1.0);
        let vdir = if a.speed > 5.0 { a.vel.norm() } else { ax.forward };
        let view_dir = (ax.forward * 0.55 + vdir * 0.45).norm();
        let mut offset = ax.back * (14.5 + speed_factor * 3.0) + ax.up * 4.2;
        offset = Quat::axis_angle(ax.up, self.chase_look.0).rotate(offset);
        offset = Quat::axis_angle(ax.right, self.chase_look.1).rotate(offset);
        offset = offset * self.opts.zoom.unwrap_or(1.0);
        let mut eye = a.pos + offset;
        let orbiting = self.chase_look.0.abs() + self.chase_look.1.abs() > 0.001;
        let look = if orbiting { a.pos + ax.up * 0.35 } else { a.pos + view_dir * 10.0 + ax.up * 0.7 };
        let stable_up = (ax.up * 0.70 + v3(0.0, 0.30, 0.0)).norm();
        let basis = |eye: V3| {
            let f = (look - eye).norm();
            let mut r = f.cross(stable_up).norm();
            if r.len() < 1e-4 {
                r = ax.right;
            }
            (f, r, r.cross(f).norm())
        };
        if matches!(self.state, State::Flying | State::Transition) {
            let (_, r, u) = basis(eye);
            let shake = self.shake(a);
            let sx = (t * 31.1).sin() * 0.6 + (t * 18.3 + 0.4).sin() * 0.4;
            let sy = (t * 26.3 + 1.7).sin() * 0.6 + (t * 41.7 + 0.3).sin() * 0.4;
            // Keep external-camera vibration subtle at this short follow distance.
            eye = eye + r * (sx * shake * 50.0) + u * (sy * shake * 50.0);
        }
        let floor = terrain::floor(eye.x, eye.z) + 2.4;
        if eye.y < floor {
            eye.y = floor;
        }
        let (f, r, u) = basis(eye);
        Camera { eye, forward: f, right: r, up: u, fov: (48.0 + speed_factor * 3.0) * DEG * self.zoom_scale() }
    }

    /// Field-of-view factor for the zoom, down to 0.4 when fully in.
    fn zoom_scale(&self) -> f64 {
        1.0 - 0.6 * self.zoom
    }

    /// Renders the world behind the jet, seen from over the pilot's head,
    /// into the rear-view mirrors' picture and hands it to the cockpit atlas.
    fn draw_mirrors(&mut self) {
        // The MiG-29, Hornet and Flanker carry mirrors; the F-16 has none.
        let Some(a) = self.aircraft.as_ref().filter(|a| a.kind.has_mirrors() && !self.opts.no_mirrors) else { return };
        let rear = cockpit::mirror_camera(a);
        let m = &self.mirror;
        self.ctx.begin_pass(Some(m.pass), PassAction::clear_color(0.1, 0.14, 0.16, 1.0));
        let mission = (self.training.is_none() && matches!(self.kind, MissionKind::Gates | MissionKind::Free)).then_some(&self.mission);
        let scene = props::Scene { models: &self.models, combat: &self.combat, player: Some(a), ejection: None, first_person: false };
        self.world.draw(&mut *self.ctx, &rear, m.w as f64 / m.h as f64, self.time, mission, Some(a), true, Some(&scene as &dyn world::Props));
        self.ctx.end_render_pass();
        self.cockpit.blit_mirror(&mut *self.ctx, &self.world, m);
    }

    fn current_camera(&self) -> Camera {
        match (&self.aircraft, self.state) {
            (_, State::Attract | State::Loadout | State::School | State::Network) | (None, _) => self.attract_camera(self.attract_time),
            (Some(_), _) if self.ejection.is_some() => self.eject_camera(self.ejection.as_ref().unwrap()),
            (Some(a), _) if a.crashed || a.lost.any() => self.wreck_camera(a),
            (Some(a), _) => {
                if self.view == View::Chase {
                    self.chase_camera(a)
                } else {
                    self.flight_camera(a)
                }
            }
        }
    }

    fn update_audio(&self) {
        let active = self.state == State::Flying || self.state == State::Transition;
        let mut p = Params::default();
        if let (Some(a), true) = (&self.aircraft, active) {
            let (thr, speed, ab) = (a.throttle as f32, a.speed as f32, a.afterburner_visual as f32);
            p.eng_gain = (0.025 + thr * 0.07 + ab * 0.075).clamp(0.0, 0.2);
            let rpm = ((a.rpm as f32 - 45.0) / 55.0).clamp(0.0, 1.05);
            p.eng_freq = 450.0 + rpm * 1900.0;
            p.engines = a.kind.engine_count() as f32;
            p.burner = ab;
            // Separated flow roars and shudders through the airframe.
            let buffet = a.buffet as f32;
            p.wind_gain = (speed / 330.0 * 0.055).clamp(0.0, 0.07) + buffet * 0.03;
            p.wind_freq = 500.0 + speed * 14.0;
            p.rumble_gain = 0.008 + thr * 0.012 + ab * 0.015 + buffet * 0.035;
            p.rumble_freq = 39.0 + thr * 22.0 + ab * 15.0 - buffet * 12.0;
            p.whine_gain = 0.002 + rpm * 0.006;
            p.whine_freq = 750.0 + rpm * 2250.0;
            p.gun = if self.combat.firing { 0.34 } else { 0.0 };
            p.gun_rate = self.combat.loadout.gun.rate as f32;
            // Sidewinder tone: a low growl while the seeker searches, a high
            // steady tone once it tracks.
            if self.combat.heater().is_some() {
                let locked = self.combat.seeker.is_some();
                p.seeker_gain = if locked { 0.035 } else { 0.014 };
                p.seeker_freq = if locked { 1250.0 } else { 380.0 + 40.0 * (self.time * 23.0).sin() as f32 };
            }
        }
        p.master = if self.settings.muted || !active && self.state != State::Result { 0.0 } else { 0.5 };
        // Out cold, the world goes quiet.
        if self.pilot.gloc > 0.0 {
            p.master *= 1.0 - 0.8 * self.pilot.vision().black as f32;
        }
        self.audio.set(p);
    }

    /// A one-shot flight action from its key or the controller. Returns
    /// false for held actions (stick, throttle, trigger, brakes), which
    /// the caller tracks as keys held down.
    fn bind_pressed(&mut self, b: Bind) -> bool {
        if self.multiplayer.active() && self.state == State::Flying && self.multiplayer.action(b) { return true; }
        if self.state == State::Flying && self.ejection_chord() && b != Bind::Eject {
            return true;
        }
        if self.ejection.is_some() && !matches!(b, Bind::View | Bind::Center) {
            return true;
        }
        let flying = self.state == State::Flying;
        let looking = matches!(self.state, State::Flying | State::Frozen);
        match b {
            Bind::LeftMfd | Bind::RightMfd if looking => {
                let i = if b == Bind::LeftMfd { 0 } else { 1 };
                self.pages[i] = self.pages[i].next();
            }
            Bind::View if looking || self.state == State::Transition => {
                self.view = if self.view == View::Chase { View::Cockpit } else { View::Chase };
                // Cutting to a camera already inside the cone is no boom.
                self.in_cone = true;
                self.flash_toast(if self.view == View::Chase { "CHASE VIEW" } else { "COCKPIT VIEW" }, 1.8);
            }
            Bind::Center if looking => {
                self.center_stick();
                self.flash_toast("MOUSE STICK CENTERED", 1.1);
            }
            Bind::Weapon if flying => {
                self.combat.cycle_weapon();
                self.flash_toast(&symbology::weapon_line(&self.combat), 1.6);
            }
            Bind::Lock if flying => {
                if let Some(a) = &self.aircraft {
                    self.combat.designate(a);
                }
            }
            Bind::Sensor if flying => {
                if let Some(a) = &self.aircraft {
                    self.combat.toggle_irst(a);
                }
            }
            Bind::Countermeasures if flying => {
                self.cm_hold = 0.0;
                self.dispense();
            }
            Bind::Jettison if flying => {
                if let Some(a) = &self.aircraft {
                    self.combat.jettison(a);
                }
            }
            Bind::Airbrake if flying => {
                self.airbrake = !self.airbrake;
                self.flash_toast(if self.airbrake { "AIRBRAKE DEPLOYED" } else { "AIRBRAKE RETRACTED" }, 1.4);
            }
            Bind::Gear if flying => self.toggle_gear(),
            Bind::Flaps if flying => {
                self.flaps_switch = !self.flaps_switch;
                self.flash_toast(if self.flaps_switch { "FLAPS DOWN" } else { "FLAPS UP" }, 1.4);
            }
            Bind::PitchUp | Bind::PitchDown | Bind::RollLeft | Bind::RollRight | Bind::YawLeft | Bind::YawRight | Bind::ThrottleUp | Bind::ThrottleDown | Bind::Afterburner | Bind::Trigger | Bind::WheelBrake | Bind::PushToTalk => return false,
            _ => {}
        }
        true
    }

    /// The F-16's manual pitch override or the MiG-29's SOS switch: the
    /// stick then reaches past the g and AoA limiters.
    fn toggle_limiters(&mut self) {
        if self.state != State::Flying {
            return;
        }
        self.limiters_off = !self.limiters_off;
        let mig = self.aircraft.as_ref().is_some_and(|a| a.kind.russian());
        let text = match (mig, self.limiters_off) {
            (false, true) => "FLCS LIMITERS OFF // G AND AOA UNPROTECTED",
            (false, false) => "FLCS LIMITERS ON",
            (true, true) => "SOS OFF // G AND AOA UNPROTECTED",
            (true, false) => "SOS ON",
        };
        self.flash_toast(text, 2.0);
    }

    /// Gear handle: it will not come up with weight on the wheels.
    fn toggle_gear(&mut self) {
        let on_ground = self.aircraft.as_ref().is_some_and(|a| a.on_ground);
        if self.gear_lever && on_ground {
            self.flash_toast("GEAR HANDLE LOCKED // WEIGHT ON WHEELS", 1.6);
            return;
        }
        self.gear_lever = !self.gear_lever;
        self.flash_toast(if self.gear_lever { "GEAR DOWN" } else { "GEAR UP" }, 1.4);
    }
}

impl EventHandler for App {
    fn update(&mut self) {
        // Some system overlays take foreground focus without minimising us.
        // Release capture and clear held controls before processing more input.
        if self.opts.shot.is_none() && self.opts.bench.is_none() && !capture::is_foreground() {
            self.pause();
            self.grab(false);
            self.keys.clear();
            self.trigger_mouse = false;
            self.zoom_mouse = false;
        }
        let now = date::now();
        let raw_dt = now - self.last_frame;
        let mut dt = raw_dt.clamp(0.0, 0.05);
        self.last_frame = now;
        if let Some(limit) = self.opts.bench {
            self.bench_frames.push(raw_dt);
            if self.state == State::Flying && self.time > 4.0 {
                self.view = if (self.time as i64 / 4) % 2 == 0 { View::Cockpit } else { View::Chase };
            }
            if self.time >= limit {
                let f = &mut self.bench_frames;
                f.drain(..f.len().min(30));
                f.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let avg = f.iter().sum::<f64>() / f.len().max(1) as f64;
                let p99 = f.get(f.len() * 99 / 100).copied().unwrap_or(0.0);
                println!("bench: {} frames, avg {:.2} ms ({:.0} fps), p99 {:.2} ms, state {:?}", f.len(), avg * 1000.0, 1.0 / avg.max(1e-9), p99 * 1000.0, self.state);
                window::order_quit();
            }
        }
        if self.opts.shot.is_some() {
            dt = 0.0;
        }
        if !matches!(self.state, State::Paused | State::Frozen) {
            self.time += dt;
        }
        // Snapshot and menu checks use scripted input, independent of live hardware.
        if self.opts.shot.is_none() && self.opts.bench.is_none() {
            if let Some(on) = self.pad.poll(now) {
                if !matches!(self.state, State::Paused | State::Frozen) {
                    self.flash_toast(if on { "CONTROLLER CONNECTED" } else { "CONTROLLER LOST" }, 2.0);
                }
                if !on {
                    self.pause();
                }
            }
            self.pad_buttons();
        }
        // Keyboard and mouse events hand the prompts back to themselves.
        if self.pad.active() || self.opts.pad {
            self.pad_active = true;
        } else if !self.pad.state.connected {
            self.pad_active = false;
        }
        self.toast_timer = (self.toast_timer - dt).max(0.0);
        let zoomed = !self.ejection_chord() && (self.zoom_mouse || self.zoom_pad) && matches!(self.state, State::Flying | State::Frozen);
        if matches!(self.state, State::Flying | State::Frozen) {
            self.zoom += (zoomed as i32 as f64 - self.zoom) * (1.0 - (-dt * 9.0).exp());
        }
        self.flash = (self.flash - dt * 2.4).max(0.0);
        match self.state {
            State::Attract | State::Loadout | State::School | State::Network => self.attract_time += dt,
            State::Flying => {
                self.accumulator = (self.accumulator + dt).min(0.12);
                let mut steps = 0;
                // Mouse X works the rudder pedals, which spring back to centre.
                self.mouse.0 *= (-dt / 0.35).exp();
                self.update_pad_look(dt);
                while self.accumulator >= FIXED_DT && steps < 8 && self.state == State::Flying {
                    self.ramp_key_pitch(FIXED_DT);
                    let input = self.read_input();
                    self.step(FIXED_DT, input);
                    self.accumulator -= FIXED_DT;
                    steps += 1;
                }
                if steps == 8 {
                    self.accumulator = 0.0;
                }
                if let (Some(a), false) = (&self.aircraft, self.settings.muted) {
                    props::rwr_tones(&self.combat, a, &self.audio, &mut self.rwr_voice, dt);
                }
                if let Some(a) = &self.aircraft {
                    if self.pilot.breaths != self.breaths {
                        self.breaths = self.pilot.breaths;
                        self.audio.breath();
                    }
                    if !self.settings.muted {
                        if let Some(cue) = self.warnings.update(dt, a) { self.audio.warning(a.kind, cue); }
                    }

                }
            }
            State::Transition => {
                self.fall(dt.min(1.0 / 30.0));
                self.transition_timer -= dt;
                if self.transition_timer <= 0.0 {
                    self.complete_transition();
                }
            }
            State::Frozen => self.update_frozen_look(dt),
            _ => {}
        }
        if !matches!(self.state, State::Paused | State::Frozen) {
            self.advance_effects(dt);
        }
        if self.opts.shot.is_none() && !matches!(self.state, State::Paused | State::Frozen) {
            self.update_hands(dt, None);
        }
        if self.state != State::Paused || self.controls_open || self.frame == 0 {
            self.camera = self.current_camera();
        }
        // The boom is heard where the Mach cone sweeps over the camera: from
        // outside the jet only; the pilot rides ahead of it.
        if let Some(a) = &self.aircraft {
            let inside = (self.view == View::Chase || a.crashed) && vapor::inside_mach_cone(a, self.camera.eye);
            if inside && !self.in_cone && self.state == State::Flying {
                self.audio.sonic_boom();
            }
            self.in_cone = inside;
        }
        // The helmet cues weapons once the head turns off the rest position,
        // and blanks while its line of sight passes through the HUD.
        let head = self.look.0.abs() > 4.0 * DEG || (self.look.1 - LOOK_PITCH).abs() > 4.0 * DEG;
        let hud = self.aircraft.as_ref().is_some_and(|a| symbology::through_hud(a.q.conj().rotate(self.camera.forward)));
        self.combat.helmet = (self.view == View::Cockpit && head && !hud && matches!(self.state, State::Flying | State::Frozen)).then_some(self.camera.forward);
        if self.multiplayer.active() && self.state != State::Flying {
            self.accumulator=(self.accumulator+dt).min(0.12);
            while self.accumulator>=FIXED_DT {
                self.network_step(FIXED_DT,Input::default());self.accumulator-=FIXED_DT;
            }
        }
        if self.opts.layout_scene.is_none() { self.network_poll(); }
        self.update_audio();
    }

    fn draw(&mut self) {
        let (w, h) = self.win;
        let flying_view = !matches!(self.state, State::Attract | State::Loadout | State::School | State::Network) && self.aircraft.is_some();
        // A wrecked jet leaves only the fireball and debris, seen from outside.
        let wrecked = self.aircraft.as_ref().is_some_and(|a| a.crashed);
        let cockpit_view = flying_view && self.view == View::Cockpit && !wrecked && self.ejection.as_ref().is_none_or(|e| e.phase == eject::Phase::Canopy);
        let cam = self.camera;

        // Instrument pages refresh at 30 Hz.
        if cockpit_view && (self.time - self.cockpit.atlas_clock >= 1.0 / 30.0 || self.cockpit.atlas_clock < 0.0 || self.time < self.cockpit.atlas_clock) {
            let a = self.aircraft.as_ref().unwrap();
            self.cockpit.update_atlas(&mut *self.ctx, &mut self.atlas_canvas, a, &self.mission, self.pages, self.time, &self.combat);
            self.cockpit.atlas_clock = self.time;
        }
        if cockpit_view {
            self.draw_mirrors();
        }

        // World and cockpit into the low-resolution scene target.
        let aspect = (w / h) as f64;
        self.ctx.begin_pass(Some(self.scene.pass), PassAction::clear_color(0.1, 0.14, 0.16, 1.0));
        // Gate rings only on the gate run and in free flight; the fight brings
        // its own props and smoke.
        let mission = if flying_view { (self.training.is_none() && matches!(self.kind, MissionKind::Gates | MissionKind::Free)).then_some(&self.mission) } else { Some(&self.preview) };
        if flying_view {
            props::fill_fx(&self.combat, &mut self.world.fx, &cam, self.aircraft.as_ref());
        } else {
            self.world.fx.extra_blend.clear();
            self.world.fx.extra_glow.clear();
        }
        let player = if flying_view && !wrecked { self.aircraft.as_ref() } else { None };
        let scene = props::Scene { models: &self.models, combat: &self.combat, player, ejection: self.ejection.as_ref(), first_person: self.view == View::Cockpit && self.ejection.is_some() };
        self.world.draw(&mut *self.ctx, &cam, aspect, self.time, mission, player, cockpit_view, flying_view.then_some(&scene as &dyn world::Props));
        self.ctx.end_render_pass();
        if cockpit_view {
            let a = self.aircraft.as_ref().unwrap();
            let cam_b = cockpit::body_camera(&cam, a);
            let sun_b = a.q.conj().rotate(world::sun_dir());
            self.ctx.begin_pass(Some(self.scene.pass), PassAction::Clear { color: None, depth: Some(1.0), stencil: None });
            self.cockpit.draw(&mut *self.ctx, &cam_b, aspect, sun_b, &self.hands, a.kind, self.ejection.is_none());
            self.ctx.end_render_pass();
        }

        // Composite to the window (or an offscreen target for screenshots).
        let out_pass = self.output.as_ref().map(|t| t.pass);
        self.ctx.begin_pass(out_pass, PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        // What the pilot sees through G: only from the seat.
        let vision = if cockpit_view && matches!(self.state, State::Flying | State::Frozen | State::Paused | State::Transition) { self.pilot.vision() } else { physio::Vision::default() };
        let gloc = vision.grey;
        self.world.post(&mut *self.ctx, &self.scene, w, h, gloc as f32, (self.time * 60.0).floor() as f32 % 997.0);
        // Instrument faces carry information, so they skip the pixelation.
        if cockpit_view {
            let a = self.aircraft.as_ref().unwrap();
            let cam_b = cockpit::body_camera(&cam, a);
            self.cockpit.draw_faces(&mut *self.ctx, &cam_b, aspect, [w, h], gloc as f32, a.kind);
        }

        // 2D overlays at native resolution.
        let mut buttons = Vec::new();
        let i = if self.state == State::Flying { self.read_control_positions() } else { Input { airbrake: self.airbrake, ..Default::default() } };
        let pad = self.pad_active;
        let obj = self.objective();
        let cue = ui::StickCue { pitch: i.pitch(), yaw: i.yaw(), roll: i.roll(), source: self.settings.pitch, layout: self.settings.pad_layout, pad, brake: self.airbrake };
        {
            let cv = &mut self.overlay;
            if flying_view {
                let a = self.aircraft.as_ref().unwrap();
                if let Some(e) = &self.ejection {
                    let s = ui::ui_scale(w, h);
                    cv.text(if self.view == View::Cockpit { "EJECTED PILOT // FIRST PERSON" } else { "EJECTED PILOT // EXTERNAL VIEW" },
                        12.0 * s, 12.0 * s, s, cockpit::HUD, canvas::Align::Left);
                    if self.toast_timer > 0.0 {
                        cv.text(&self.toast, w * 0.5, 30.0 * s, s, cockpit::HUD, canvas::Align::Center);
                    }
                    cv.text(&format!("{} // {:.0} M AGL", e.spec.name, (e.pilot.pos.y - terrain::floor(e.pilot.pos.x, e.pilot.pos.z)).max(0.0)),
                        12.0 * s, 27.0 * s, s, cockpit::HUD, canvas::Align::Left);
                    cv.text("V VIEW    ALT + MOUSE LOOK    P FREEZE", 12.0 * s, h - 16.0 * s, s, cockpit::HUD, canvas::Align::Left);
                } else if cockpit_view {
                    let cam_b = cockpit::body_camera(&cam, a);
                    let focal = h as f64 * 0.5 / (cam.fov / 2.0).tan();
                    let body = cockpit::BodyProjector { eye: cam_b.eye, right: cam_b.right, up: cam_b.up, fwd: cam_b.forward, focal, w: w as f64, h: h as f64 };
                    cockpit::draw_hud(cv, a, &self.mission, &cam, &body, &self.combat);
                    symbology::helmet(cv, w, h, ui::ui_scale(w, h), a, &self.combat, &cam, self.time);
                    // Light loss covers the HUD and helmet too, not the game's own read-outs.
                    cv.flush(&mut *self.ctx, w, h);
                    self.world.vision(&mut *self.ctx, &self.scene, w, h, &vision);
                    ui::cockpit_overlay(cv, w, h, a, &self.mission, &self.toast, self.toast_timer, &cue, obj.as_ref());
                    symbology::overlay(cv, w, h, ui::ui_scale(w, h), a, &self.combat, &cam, self.time, false);
                } else {
                    ui::chase_overlay(cv, w, h, a, &self.mission, &cam, &self.toast, self.toast_timer, self.time, self.state == State::Flying, &cue, obj.as_ref());
                    symbology::overlay(cv, w, h, ui::ui_scale(w, h), a, &self.combat, &cam, self.time, true);
                }
            }
            if flying_view && self.multiplayer.active() { ui::network_overlay(cv,w,h,&self.multiplayer,&self.combat,self.aircraft.as_ref().unwrap(),&cam); }
            if self.opts.layout_check && flying_view && !self.controls_open && self.training.is_some() {
                cv.layout_audit=Some(menu_layout::Audit::default());
            }
            if flying_view && !self.controls_open && self.state == State::Flying {
                if let Some(t) = &self.training { ui::training_overlay(cv, w, h, t, &self.settings, pad); }
            }
            if flying_view && self.settings.performance_overlay {
                let s = ui::ui_scale(w, h);
                let u = (s - 1.0).max(1.0);
                let (x, y) = if cockpit_view { (w * 0.026, h * 0.058 + 34.0 * s) } else { (12.0 * s, 52.0 * s) };
                self.performance.draw(cv, x, y + 97.0 * u, u);
            }
            if self.flash > 0.005 {
                let c = self.flash_color;
                cv.rect(0.0, 0.0, w, h, rgba(c[0], c[1], c[2], (self.flash * 0.34) as f32));
            }
            let s = ui::ui_scale(w, h);
            if self.opts.layout_check && cv.layout_audit.is_none() { cv.layout_audit=Some(menu_layout::Audit::default()); }
            let mut u = Ui { cv, w, h, s, buttons: Vec::new(), selected: self.selected, mouse: if pad { (-1000.0, -1000.0) } else { self.mouse_pos }, pad, layout: self.settings.pad_layout };
            if !self.controls_open {
                match self.state {
                    State::Attract => u.start_screen(self.settings.muted || !self.audio.available, &self.best, self.time, self.settings.pitch, self.kind, &self.config, self.settings.clock),
                    State::Network => u.network_screen(&self.multiplayer, self.config.jet, &self.settings.keys),
                    State::School => u.school_screen(self.settings.tutorial_completed, self.config.jet),
                    State::Loadout => u.loadout_screen(self.kind, &self.config, self.settings.clock),
                    State::Paused => {}
                    State::Result => {
                        u.cv.rect(0.0, 0.0, w, h, rgba(2, 5, 4, 0.5));
                        if let Some(r) = &self.result {
                            if let Some(t) = &self.training { u.training_result(t, r.kind); }
                            else { u.result_screen(r, &self.best); }
                        }
                    }
                    _ => {}
                }
            } else {
                u.controls_drawer(&self.settings, matches!(self.state, State::Paused | State::Frozen), self.rebinding, self.multiplayer.active());
                if self.multiplayer.active() { u.online_controls_button(); }
            }
            buttons.append(&mut u.buttons);
            if let Some(audit)=cv.layout_audit.take() {
                let issues=audit.issues(w,h);
                if !issues.is_empty() {
                    if !self.opts.layout_failed {for issue in &issues { eprintln!("LAYOUT frame {}: {issue}",self.frame); }}
                    self.opts.layout_failed=true;
                }else if self.frame==0 { println!("PASS menu layout: {} items checked",audit.items.len()); }
            }
        }
        self.overlay.flush(&mut *self.ctx, w, h);
        if self.screenshot_pending && self.output.is_none() {
            self.screenshot_pending = false;
            match capture::screenshot(w as u32, h as u32) {
                Ok(path) => {
                    println!("saved {}", path.display());
                    self.flash_toast("SCREENSHOT SAVED // PICTURES / VIPER-RUN", 2.0);
                }
                Err(e) => {
                    eprintln!("screenshot failed: {e}");
                    self.flash_toast("SCREENSHOT FAILED", 2.0);
                }
            }
        }
        self.ctx.end_render_pass();
        self.ctx.commit_frame();
        self.buttons = buttons;
        if self.selected >= self.buttons.len() {
            self.selected = 0;
        }

        self.performance.presented();
        self.frame += 1;
        if (self.opts.ui_check && ui_check::step(self,self.frame)) || (self.opts.future_ui_check && ui_check::future_step(self,self.frame)) {
            self.opts.ui_check = false;
            self.opts.future_ui_check = false;
            self.opts.frames = self.frame;
        }
        if let (Some(path), Some(out)) = (self.opts.shot.clone(), self.output.as_ref()) {
            if self.frame >= self.opts.frames && !self.opts.ui_check && !self.opts.future_ui_check {
                let mut px = vec![0u8; (out.w * out.h * 4) as usize];
                self.ctx.texture_read_pixels(out.color, &mut px);
                match png::write(&path, out.w, out.h, &px, true) {
                    Ok(()) => println!("saved {path} ({}x{}), {} terrain chunks drawn", out.w, out.h, self.world.drawn_chunks),
                    Err(e) => eprintln!("screenshot failed: {e}"),
                }
                assert!(!self.opts.layout_failed,"menu layout check failed; screenshot saved for inspection");
                window::order_quit();
            }
        }
    }

    fn resize_event(&mut self, w: f32, h: f32) {
        if w < 2.0 || h < 2.0 {
            return;
        }
        self.win = (w, h);
        let (rw, rh) = render_size(w, h);
        if rw != self.scene.w || rh != self.scene.h {
            self.scene.delete(&mut *self.ctx);
            self.scene = gfx::Target::new(&mut *self.ctx, rw, rh, FilterMode::Nearest);
        }
        if self.opts.shot.is_some() {
            if let Some(o) = self.output.take() {
                o.delete(&mut *self.ctx);
            }
            self.output = Some(gfx::Target::new(&mut *self.ctx, w as u32, h as u32, FilterMode::Nearest));
        }
    }

    fn raw_mouse_motion(&mut self, dx: f32, dy: f32) {
        if !matches!(self.state, State::Flying | State::Frozen) || !self.grabbed {
            return;
        }
        let (dx, dy) = (dx as f64, dy as f64);
        if dx.abs() + dy.abs() > 6.0 {
            self.pad_active = false;
        }
        if self.state == State::Frozen || self.keys.contains(&KeyCode::LeftAlt) || self.keys.contains(&KeyCode::RightAlt) {
            if self.view == View::Cockpit {
                self.look.0 = clamp(self.look.0 + dx * 0.003, -155.0 * DEG, 155.0 * DEG);
                self.look.1 = clamp(self.look.1 - dy * 0.003, -85.0 * DEG, 85.0 * DEG);
            } else {
                self.chase_look.0 -= dx * 0.003;
                self.chase_look.1 = clamp(self.chase_look.1 + dy * 0.003, -65.0 * DEG, 65.0 * DEG);
            }
            return;
        }
        let sens = 0.0022 * self.settings.sensitivity;
        self.mouse.0 = clamp(self.mouse.0 + dx * sens, -1.0, 1.0);
        self.mouse.1 = clamp(self.mouse.1 + dy * sens, -1.0, 1.0);
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        if (x - self.mouse_pos.0).abs() + (y - self.mouse_pos.1).abs() > 6.0 {
            self.pad_active = false;
        }
        self.mouse_pos = (x, y);
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        self.pad_active = false;
        self.mouse_pos = (x, y);
        if matches!(self.state, State::Flying | State::Frozen) && !self.controls_open {
            match button {
                MouseButton::Right => self.zoom_mouse = true,
                MouseButton::Middle => {
                    self.center_stick();
                    self.flash_toast("MOUSE STICK CENTERED", 1.1);
                }
                MouseButton::Left => {
                    // With the mouse captured the left button is the trigger;
                    // otherwise a click captures it again.
                    if self.grabbed && self.state == State::Flying {
                        self.trigger_mouse = true;
                    }
                    self.grab(true);
                }
                _ => {}
            }
            return;
        }
        if button == MouseButton::Left {
            if let Some(b) = self.buttons.iter().find(|b| b.hit(x, y)).copied() {
                self.act(b.action);
            }
        }
    }

    fn key_down_event(&mut self, key: KeyCode, mods: KeyMods, repeat: bool) {
        self.pad_active = self.pad_dispatch;
        if self.state==State::Network && self.multiplayer.menu.editing.is_some() {
            match key {
                KeyCode::Escape=>self.multiplayer.menu.finish(true),
                KeyCode::Enter | KeyCode::KpEnter=>self.multiplayer.menu.finish(false),
                KeyCode::Backspace=>self.multiplayer.menu.backspace(),
                KeyCode::V if mods.ctrl=>{if let Some(text)=window::clipboard_get(){for ch in text.chars(){self.multiplayer.menu.character(ch);}}},
                _=>{}
            }
            return;
        }
        if key == KeyCode::F12 && !repeat {
            self.screenshot_pending = true;
            return;
        }
        // Leave desktop capture, recording and overlay chords to Windows.
        if key == KeyCode::PrintScreen || mods.logo || matches!(key, KeyCode::LeftSuper | KeyCode::RightSuper) {
            self.pause();
            self.grab(false);
            return;
        }
        if mods.alt && !matches!(key, KeyCode::LeftAlt | KeyCode::RightAlt | KeyCode::O) {
            if key == KeyCode::Enter && !repeat {
                self.toggle_fullscreen();
            }
            return;
        }
        if key == KeyCode::F11 && !repeat {
            self.toggle_fullscreen();
            return;
        }
        // A binding row in the drawer is waiting for its key.
        if let Some((i, slot)) = self.rebinding {
            if repeat {
                return;
            }
            self.rebinding = None;
            let b = keymap::BINDS[i];
            match key {
                KeyCode::Escape => self.flash_toast("BINDING UNCHANGED", 1.2),
                KeyCode::Backspace | KeyCode::Delete => {
                    self.settings.keys.set(b, slot, None);
                    self.save();
                    self.flash_toast(&format!("{} CLEARED", b.label()), 1.4);
                }
                k if keymap::bindable(k) => {
                    self.settings.keys.set(b, slot, Some(k));
                    self.save();
                    self.flash_toast(&format!("{} // {}", b.label(), keymap::key_name(k)), 1.6);
                }
                _ => self.flash_toast("THAT KEY IS RESERVED", 1.4),
            }
            return;
        }
        if self.state == State::Flying && self.settings.keys.lookup(key) == Some(Bind::Eject) {
            self.keys.insert(key);
            return;
        }
        if self.state == State::Flying && self.ejection_chord() {
            return;
        }
        if key == KeyCode::K && !repeat {
            self.toggle_controls(None);
            return;
        }
        if self.multiplayer.session.is_some() && self.settings.keys.lookup(key)==Some(Bind::PushToTalk) {
            self.keys.insert(key);
            if !matches!(key,KeyCode::Up|KeyCode::Down|KeyCode::Left|KeyCode::Right|KeyCode::Tab){return;}
        }
        let in_menu = self.controls_open || matches!(self.state, State::Attract | State::Paused | State::Result | State::Loadout | State::School | State::Network);
        // Space engages from the mission board and launches from the setup
        // screen, whichever button is highlighted.
        if key == KeyCode::Space && !repeat && !self.controls_open && matches!(self.state, State::Attract | State::Loadout) {
            self.act(if self.state == State::Loadout { Action::Launch } else { Action::Start });
            return;
        }
        if in_menu && !self.buttons.is_empty() {
            let direction = match key {
                KeyCode::Left => Some((-1.0, 0.0)), KeyCode::Right => Some((1.0, 0.0)),
                KeyCode::Up => Some((0.0, -1.0)), KeyCode::Down => Some((0.0, 1.0)), _ => None,
            };
            if let Some((dx, dy)) = direction {
                if let Some(i) = ui::navigation(&self.buttons, self.selected, dx, dy) { self.selected = i; }
                return;
            }
            if key == KeyCode::Tab {
                self.selected = (self.selected + 1) % self.buttons.len();
                return;
            }
        }
        if self.controls_open {
            match key {
                KeyCode::Escape => self.toggle_controls(Some(false)),
                KeyCode::Enter | KeyCode::KpEnter | KeyCode::Space if !repeat => {
                    if let Some(b) = self.buttons.get(self.selected).copied() {
                        self.act(b.action);
                    }
                }
                _ => {}
            }
            return;
        }
        let alt = matches!(key, KeyCode::LeftAlt | KeyCode::RightAlt);
        if alt && self.state == State::Flying {
            // Turning the head must not add a flight-control demand.
            self.mouse = (0.0, 0.0);
            self.keys.insert(key);
            return;
        }
        // ALT+O: the limiter override, a fixed combination so no flight key
        // can set it off by itself.
        let alt_held = self.keys.contains(&KeyCode::LeftAlt) || self.keys.contains(&KeyCode::RightAlt);
        if key == KeyCode::O && alt_held && !repeat {
            self.toggle_limiters();
            return;
        }
        if repeat {
            return;
        }
        if let Some(b) = self.settings.keys.lookup(key) {
            if self.bind_pressed(b) {
                return;
            }
        }
        match key {
            KeyCode::M => self.toggle_mute(),
            KeyCode::P => match self.state {
                State::Flying | State::Frozen => self.toggle_freeze(),
                State::Paused => self.resume(),
                _ => {}
            },
            KeyCode::Escape => match self.state {
                State::Flying | State::Frozen | State::Paused | State::Attract => self.toggle_controls(Some(true)),
                State::Network if self.multiplayer.active() => { self.network_action(Action::NetResume); },
                State::Result | State::Loadout | State::School | State::Network => self.go_menu(),
                _ => {}
            },
            KeyCode::Enter | KeyCode::KpEnter => {
                if matches!(self.state, State::Attract | State::Paused | State::Result | State::Loadout | State::School | State::Network) {
                    if let Some(b) = self.buttons.get(self.selected).copied() {
                        self.act(b.action);
                    }
                }
            }
            KeyCode::R if matches!(self.state, State::Paused | State::Result) => self.act(Action::Restart),
            KeyCode::Q if mods.ctrl && self.state != State::Flying => window::order_quit(),
            _ => {
                if self.state == State::Flying {
                    self.keys.insert(key);
                }
            }
        }
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, _x: f32, _y: f32) {
        match button {
            MouseButton::Left => self.trigger_mouse = false,
            MouseButton::Right => self.zoom_mouse = false,
            _ => {}
        }
    }

    fn char_event(&mut self, character: char, keymods: KeyMods, _repeat: bool) {
        if self.state==State::Network && !keymods.ctrl && !keymods.alt { self.multiplayer.menu.character(character); }
    }

    fn key_up_event(&mut self, key: KeyCode, _mods: KeyMods) {
        self.keys.remove(&key);
        if self.state == State::Flying && matches!(key, KeyCode::LeftAlt | KeyCode::RightAlt) && !self.keys.contains(&KeyCode::LeftAlt) && !self.keys.contains(&KeyCode::RightAlt) {
            self.center_stick();
        }
    }

    fn window_minimized_event(&mut self) {
        if self.opts.shot.is_some() || self.opts.bench.is_some() { return; }
        self.keys.clear();
        self.pause();
        self.grab(false);
        self.trigger_mouse = false;
        self.zoom_mouse = false;
    }
}

fn main() {
    let opts = parse_opts();
    if let Some(path) = &opts.audio_demo {
        audio::demo(path, opts.jet.unwrap_or_default()).expect("write audio demo");
        return;
    }
    let shot = opts.shot.is_some();
    let bench = opts.bench.is_some();
    let (win_w, win_h) = opts.size.unwrap_or((1280, 720));
    let t = std::time::Instant::now();
    terrain::init();
    if shot {
        println!("terrain ready in {:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
    }
    let conf = conf::Conf {
        window_title: "VIPER RUN".to_string(),
        window_width: win_w,
        window_height: win_h,
        window_resizable: true,
        icon: Some(window_icon()),
        high_dpi: false,
        sample_count: 1,
        platform: conf::Platform { swap_interval: Some(if bench { 0 } else { 1 }), ..Default::default() },
        ..Default::default()
    };
    miniquad::start(conf, move || {
        let mut app = App::new(opts);
        let (w, h) = window::screen_size();
        app.resize_event(w, h);
        Box::new(app)
    });
}

fn window_icon() -> conf::Icon {
    let mut icon = conf::Icon { small: [0; 1024], medium: [0; 4096], big: [0; 16384] };
    icon.small.copy_from_slice(&icon::render(16));
    icon.medium.copy_from_slice(&icon::render(32));
    icon.big.copy_from_slice(&icon::render(64));
    icon
}

fn ejection_requested(km: &keymap::Keymap, keys: &HashSet<KeyCode>, pad: &Gamepad, scripted: bool) -> bool {
    scripted || km.held(Bind::Eject, keys) || pad.held(gamepad::LB) && pad.held(gamepad::RB)
}

#[cfg(test)]
mod input_priority_tests {
    use super::*;

    #[test]
    fn ejection_chord_and_remapped_key_take_priority_in_both_pad_layouts() {
        let mut km = keymap::Keymap::default();
        km.set(Bind::Eject, 0, Some(KeyCode::J));
        km.set(Bind::Eject, 1, None);
        let mut keys = HashSet::new();
        let mut pad = Gamepad::new();
        for buttons in [0, gamepad::LB, gamepad::RB, gamepad::LB | gamepad::RB] {
            pad.state.buttons = buttons;
            assert_eq!(ejection_requested(&km, &keys, &pad, false), buttons == gamepad::LB | gamepad::RB);
        }
        pad.state.buttons = 0;
        keys.insert(KeyCode::J);
        keys.insert(KeyCode::F);
        keys.insert(KeyCode::W);
        assert!(ejection_requested(&km, &keys, &pad, false));
        keys.remove(&KeyCode::J);
        assert!(!ejection_requested(&km, &keys, &pad, false));
        assert!(ejection_requested(&km, &keys, &pad, true));
    }
}
