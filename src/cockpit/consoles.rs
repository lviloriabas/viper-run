//! F-16C Block 50 side consoles. Panel order, sizes and legends follow the
//! left and right console figures of T.O. 1F-16CM-1 (Falcon BMS edition,
//! section 1.5); every panel is a Dzus-rail lightplate 5.75 in wide with a
//! height in 0.375 in steps. Heights, the 10 degree inboard tilt of the tops
//! and the forward sections pitched up toward the front come from the
//! FlightGear F-16 cockpit model, measured from the design eye, and so do the
//! places of the consoles, the stick and the throttle (see
//! docs/F16-REFERENCES.md).
//!
//! Each panel face is drawn into the right half of the instrument atlas, so
//! `Cockpit::draw_faces` redraws its legends sharp, and carries chunky
//! toggles, guards, knobs, buttons and wheels in low-poly geometry.
use super::{uv, R, WHITE};
use crate::canvas::{rgba, Align, Canvas, Col};
use crate::math::*;
use crate::mesh::{Frame, Mesh, Rgb};

const IN: f64 = 0.0254;
/// Front edge of both consoles, where the stick mount and throttle quadrant start.
pub(super) const FRONT_Z: f64 = -3.725;
/// Inboard edge of both consoles, against the seat bucket.
const X_IN: f64 = 0.262;
/// Side walls of the tub at console height, and the top of their vertical part.
pub(super) const WALL_X: f64 = 0.58;
pub(super) const WALL_Y: f64 = 0.575;
/// Console tops lean 10 degrees toward the pilot (outboard edge higher).
const ROLL: f64 = 10.0 * DEG;
/// The forward sections rise 4.6 degrees toward the front.
const PITCH: f64 = 4.6 * DEG;
/// Forward section length and inboard edge height at the front edge, left and right.
const FWD: [f64; 2] = [14.25 * IN, 12.0 * IN];
const Y_FRONT: [f64; 2] = [0.513, 0.51];
/// Overall console length along the top, in inches.
const LEN: f64 = 34.0;
/// Width along the top from the inboard edge to the wall, in inches.
const WID: f64 = 12.7;
/// Throttle quadrant: top of its housing on the outboard side of the left
/// console, and the lever slot.
pub(super) const QUADRANT_TOP: f64 = 0.575;
pub(super) const SLOT_X: f64 = -0.495;
/// Panel faces stand this far proud of the console top.
const LIFT: f64 = 0.0015;

/// Atlas pixels per inch of panel face: the 5x7 font then reads about
/// 0.19 in tall, half again the size of the real legends.
const PPI: f32 = 36.0;
/// Atlas area for the lightplates, and a plain patch for blank plates.
const AREA: R = R { x: 1024.0, y: 0.0, w: 1024.0, h: 992.0 };
const BLANK: R = R { x: 2016.0, y: 992.0, w: 32.0, h: 32.0 };

const FACE: Col = rgba(0x2c, 0x30, 0x32, 1.0);
const SEAM: Col = rgba(0x15, 0x18, 0x19, 1.0);
const DIM: Col = rgba(0x9c, 0xa2, 0x9c, 1.0);
const HOLE: Col = rgba(0x10, 0x12, 0x13, 1.0);
const AMBER_OFF: Col = rgba(0x4a, 0x3b, 0x1c, 1.0);
const AMBER_INK: Col = rgba(0xc9, 0xa9, 0x62, 1.0);

/// Console structure in FS 36231 dark gull grey; switch hardware.
const STRUCT: Rgb = Rgb(0.20, 0.215, 0.22);
const NUT: Rgb = Rgb(0.33, 0.34, 0.34);
const LEVER: Rgb = Rgb(0.62, 0.63, 0.62);
const KNOB: Rgb = Rgb(0.07, 0.075, 0.08);
const GUARD_RED: Rgb = Rgb(0.62, 0.09, 0.07);
const GUARD_BLACK: Rgb = Rgb(0.11, 0.115, 0.12);
const O2_RED: Rgb = Rgb(0.66, 0.10, 0.08);
const O2_WHITE: Rgb = Rgb(0.80, 0.80, 0.77);
const O2_GREEN: Rgb = Rgb(0.18, 0.45, 0.22);

/// Panel contents in the face's own frame: inches from its top-left corner,
/// `u` along the legends, `v` down the face.
#[derive(Clone, Copy)]
enum It {
    /// Centred legend.
    T(f64, f64, &'static str),
    /// Title up the face at `u`, reading bottom to top.
    V(f64, &'static str),
    Ln(f64, f64, f64, f64),
    /// Toggle thrown up (1), centred (0) or down (-1) the face; sideways.
    Tg(f64, f64, i8),
    Tw(f64, f64, i8),
    /// Guarded switch under its closed cover, red or black.
    Gd(f64, f64, bool),
    /// Rotary knob: radius, pointer in degrees clockwise from up.
    Kn(f64, f64, f64, f64),
    /// Round push button of radius `r`.
    Pb(f64, f64, f64),
    /// Indicator lamp, `|` splits two lines; round green lamp.
    Lt(f64, f64, f64, f64, &'static str),
    Gl(f64, f64, f64),
    /// Display window with green digits.
    Dp(f64, f64, f64, f64, &'static str),
    /// Square lit push button.
    Sq(f64, f64, f64, f64, &'static str),
    /// Trim wheel: rolls up and down the face (true) or across it.
    Wh(f64, f64, bool),
    /// Oxygen regulator lever.
    Lv(f64, f64, Rgb),
    /// Small round gauge; fill connector.
    Gg(f64, f64, f64),
    Cn(f64, f64, f64),
    /// Trim position scale: length, vertical.
    Sc(f64, f64, f64, bool),
    /// Guard rail beside a switch.
    Rl(f64, f64),
    /// Data transfer cartridge.
    Ct(f64, f64, f64, f64),
}
use It::*;

/// A lightplate: console side (-1 left, 1 right), its place on the console
/// top (`s` aft of the front edge, `l` outboard of the inboard edge) and its
/// extent (`len` fore and aft, `wid` across), in inches. Rotated plates read
/// from the outboard side, as the fuel, IFF and lighting rows do.
struct Panel {
    side: f64,
    s: f64,
    l: f64,
    len: f64,
    wid: f64,
    rot: bool,
    items: &'static [It],
}

const fn p(side: f64, s: f64, l: f64, len: f64, wid: f64, rot: bool, items: &'static [It]) -> Panel {
    Panel { side, s, l, len, wid, rot, items }
}

const L: f64 = -1.0;
const RT: f64 = 1.0;

#[rustfmt::skip]
const PANELS: [Panel; 28] = [
    // ---------------------------------------------- left console, forward section
    p(L, 0.0, 0.0, 4.875, 5.75, false, &[ // UHF backup control
        V(5.5, "UHF"),
        Ln(0.25, 0.25, 2.05, 0.25), Ln(0.25, 1.35, 2.05, 1.35), Ln(0.25, 0.25, 0.25, 1.35), Ln(2.05, 0.25, 2.05, 1.35), Ln(0.7, 0.25, 0.7, 1.35),
        Ln(0.25, 0.53, 2.05, 0.53), Ln(0.25, 0.8, 2.05, 0.8), Ln(0.25, 1.07, 2.05, 1.07),
        T(3.6, 0.35, "CHAN"), Dp(3.6, 0.72, 0.55, 0.32, "1"), Kn(4.75, 0.75, 0.28, 30.0),
        T(0.85, 1.65, "TEST"), Pb(0.85, 1.95, 0.11), Dp(2.75, 1.85, 1.75, 0.34, "305.00"),
        T(4.75, 1.65, "STATUS"), Pb(4.75, 1.95, 0.11),
        Kn(0.85, 2.65, 0.17, 0.0), Kn(1.85, 2.65, 0.17, 0.0), Kn(2.85, 2.65, 0.17, 0.0), Kn(3.85, 2.65, 0.17, 0.0), Kn(4.85, 2.65, 0.17, 0.0),
        T(0.5, 3.35, "OFF"), T(1.25, 3.2, "MAIN"), T(2.0, 3.35, "BOTH"), T(2.05, 4.05, "ADF"), Kn(1.25, 3.85, 0.32, -40.0),
        T(2.9, 3.3, "VOL"), Kn(2.9, 3.7, 0.18, 20.0),
        T(3.6, 3.35, "MNL"), T(4.35, 3.2, "PRESET"), T(5.05, 3.35, "GRD"), Kn(4.35, 3.85, 0.32, 0.0),
        Pb(1.95, 4.45, 0.1), T(1.95, 4.72, "TONE"), Tw(3.05, 4.4, 1), T(3.05, 4.72, "SQUELCH"),
    ]),
    p(L, 4.875, 0.0, 3.0, 5.75, false, &[ // AUDIO 1
        V(0.25, "AUDIO 1"), Ln(3.1, 0.15, 3.1, 2.85),
        T(1.25, 0.3, "COMM 1"), Kn(1.25, 0.8, 0.2, -60.0), T(2.45, 0.3, "COMM 2"), Kn(2.45, 0.8, 0.2, -60.0),
        T(3.75, 0.22, "SECURE"), T(3.75, 0.45, "VOICE"), Kn(3.75, 0.85, 0.2, -60.0), T(4.95, 0.3, "MSL"), Kn(4.95, 0.8, 0.2, 40.0),
        T(1.25, 1.6, "SQL"), Tg(1.25, 2.1, 1), T(1.25, 2.65, "GD"), T(2.45, 1.6, "SQL"), Tg(2.45, 2.1, 1), T(2.45, 2.65, "GD"),
        T(3.75, 1.6, "TF"), Kn(3.75, 2.15, 0.2, 40.0), T(4.95, 1.6, "THREAT"), Kn(4.95, 2.15, 0.2, 40.0),
    ]),
    p(L, 7.875, 0.0, 1.5, 5.75, false, &[ // AUDIO 2
        V(0.25, "AUDIO 2"),
        T(1.3, 0.3, "INTERCOM"), Kn(1.3, 0.95, 0.2, 30.0), T(2.55, 0.3, "TACAN"), Kn(2.55, 0.95, 0.2, -60.0),
        T(3.75, 0.3, "ILS"), Kn(3.75, 0.95, 0.2, -60.0), T(4.95, 0.3, "HOT MIC"), Tg(4.95, 0.85, 0), T(4.95, 1.3, "CIPHER"),
    ]),
    p(L, 9.375, 0.0, 2.625, 5.75, false, &[ // ECM
        V(0.25, "ECM"),
        T(0.9, 0.28, "OPR"), Tg(0.9, 0.68, 0), T(0.9, 1.08, "STBY"), T(0.9, 1.45, "DIM"), Kn(0.9, 1.85, 0.16, 0.0),
        T(1.85, 0.6, "XMIT"), Kn(1.85, 1.0, 0.14, 0.0), Pb(0.75, 2.35, 0.1), T(1.3, 2.35, "RESET"), T(1.95, 1.95, "BIT"), Pb(1.95, 2.3, 0.1),
        Sq(2.75, 0.85, 0.56, 0.7, "1"), Sq(3.5, 0.85, 0.56, 0.7, "2"), Sq(4.25, 0.85, 0.56, 0.7, "3"), Sq(5.0, 0.85, 0.56, 0.7, "4"),
        Sq(2.75, 1.75, 0.56, 0.7, "5"), Sq(3.5, 1.75, 0.56, 0.7, "FRM"), Sq(4.25, 1.75, 0.56, 0.7, "SPL"), Sq(5.0, 1.75, 0.56, 0.7, "NIL"),
    ]),
    p(L, 12.0, 0.0, 2.25, 5.75, false, &[ // AVTR
        V(0.25, "AVTR"),
        T(0.95, 0.4, "HUD"), T(2.25, 0.4, "MFD"), Kn(1.6, 1.15, 0.3, -30.0), T(1.6, 1.95, "VIDEO SELECT"),
        Lt(3.3, 0.5, 0.65, 0.28, "AVTR"), Lt(3.3, 0.85, 0.65, 0.28, "CTVS"), Pb(3.3, 1.45, 0.1), T(3.3, 1.85, "TEST"),
        T(4.75, 0.3, "ON"), Tg(4.75, 0.95, 0), T(5.3, 0.95, "OFF"), T(4.75, 1.6, "AUTO"),
    ]),
    p(L, 0.0, 5.75, 2.625, 3.0, false, &[ // MANUAL PITCH override
        T(1.5, 0.35, "MANUAL PITCH"), T(0.7, 1.35, "OVRD"), Tw(1.5, 1.35, 1), T(2.3, 1.35, "NORM"),
    ]),
    p(L, 2.625, 5.75, 7.875, 3.0, false, &[ // ENG & JET START
        V(0.25, "ENG & JET START"),
        T(1.7, 0.3, "JFS"), Gl(1.0, 0.85, 0.15), T(1.0, 1.15, "RUN"), T(2.1, 0.55, "START 1"), Tg(2.1, 0.95, 0), T(2.1, 1.38, "START 2"),
        Ln(0.6, 1.7, 2.8, 1.7),
        T(1.7, 2.0, "ENG CONT"), T(1.7, 2.3, "PRI"), Gd(1.7, 2.85, true), T(1.7, 3.4, "SEC"),
        Ln(0.6, 3.75, 2.8, 3.75),
        T(1.7, 4.05, "AB RESET"), Tg(1.7, 4.6, 0), T(2.45, 4.6, "NORM"), T(1.7, 5.15, "ENG DATA"),
        Ln(0.6, 5.5, 2.8, 5.5),
        T(1.7, 5.8, "MAX POWER"), Rl(1.2, 6.6), Tg(1.7, 6.6, -1), Rl(2.2, 6.6), T(1.7, 7.35, "OFF"),
    ]),
    p(L, 10.5, 5.75, 3.75, 3.0, false, &[ // ELEC
        V(0.25, "ELEC"),
        T(1.1, 0.3, "MAIN PWR"), Tg(1.1, 0.95, 1), T(1.1, 1.55, "OFF"),
        Lt(2.35, 0.35, 0.9, 0.28, "PMG"), Lt(2.35, 0.7, 0.9, 0.28, "MAIN"), Lt(2.35, 1.05, 0.9, 0.28, "STBY"), Lt(2.35, 1.4, 0.9, 0.28, "EPU"), Lt(2.35, 1.75, 0.9, 0.28, "FAIL"),
        T(1.1, 2.3, "CAUTION"), T(1.1, 2.55, "RESET"), Pb(1.1, 3.05, 0.13),
    ]),
    p(L, 9.375, 8.75, 4.875, 2.125, false, &[ // EPU
        V(0.22, "EPU"),
        Lt(1.2, 0.85, 1.4, 0.62, "HYDRAZN|AIR"), T(0.8, 1.75, "RUN"), Gl(1.45, 1.75, 0.18),
        T(1.15, 2.45, "ON"), Gd(1.15, 3.25, true), T(1.75, 3.25, "NORM"), T(1.15, 4.1, "OFF"),
    ]),
    // ---------------------------------------------- left console, aft section
    p(L, 14.25, 8.625, 5.75, 2.25, true, &[ // FUEL
        V(0.2, "FUEL"),
        T(1.0, 0.28, "MASTER"), Gd(1.0, 1.05, true), T(1.0, 1.75, "OFF"),
        T(2.35, 0.22, "TANK"), T(2.35, 0.45, "INERTING"), Tg(2.35, 1.05, -1), T(2.35, 1.75, "OFF"),
        T(3.75, 0.35, "NORM"), T(3.05, 0.65, "OFF"), T(4.45, 0.65, "AFT"), T(4.5, 1.65, "FWD"), Kn(3.75, 1.15, 0.3, 0.0), T(3.75, 1.95, "ENG FEED"),
        T(5.15, 0.22, "AIR"), T(5.15, 0.45, "REFUEL"), Tg(5.15, 1.05, -1), T(5.15, 1.75, "CLOSE"),
    ]),
    p(L, 14.25, 4.875, 5.75, 3.75, true, &[ // IFF
        V(0.2, "IFF"),
        T(1.05, 0.3, "C & I"), Kn(1.05, 0.85, 0.22, -45.0), T(0.6, 1.35, "UFC"), T(1.5, 1.35, "BACKUP"),
        T(1.05, 1.9, "MASTER"), T(0.5, 2.25, "STBY"), T(1.6, 2.25, "NORM"), Kn(1.05, 2.7, 0.32, 30.0), T(0.45, 3.2, "OFF"), T(1.75, 3.2, "EMER"),
        T(2.65, 0.2, "M-4"), T(2.65, 0.43, "CODE"), Tg(2.65, 0.85, 1), T(2.65, 1.4, "ZERO"),
        T(3.85, 0.3, "REPLY"), Tg(3.85, 0.85, 0), T(3.85, 1.4, "OUT"),
        T(5.0, 0.3, "MONITOR"), Tg(5.0, 0.85, -1), T(5.0, 1.4, "OUT"),
        T(2.65, 2.0, "ENABLE"), Tg(2.65, 2.55, 0), T(2.65, 3.1, "M3/MS"),
        T(4.4, 2.1, "MODE"), Sq(3.6, 2.7, 0.44, 0.55, "M1"), Sq(4.15, 2.7, 0.44, 0.55, "M2"), Sq(4.7, 2.7, 0.44, 0.55, "M3"), Sq(5.25, 2.7, 0.44, 0.55, "MC"),
    ]),
    p(L, 14.25, 1.5, 5.75, 3.375, true, &[ // EXT LIGHTING
        V(0.22, "EXT LIGHTING"),
        T(1.25, 0.32, "ANTI"), T(1.25, 0.55, "COLLISION"), Tg(1.1, 1.05, 1), T(1.1, 1.5, "OFF"),
        T(2.95, 0.15, "POSITION"),
        T(2.4, 0.52, "FLASH"), Tg(2.4, 1.0, 1), T(2.4, 1.45, "STEADY"),
        T(3.5, 0.52, "WING/TAIL"), Tg(3.5, 1.15, 0), T(5.0, 0.52, "FUSELAGE"), Tg(5.0, 1.15, 0),
        T(4.25, 0.85, "BRT"), T(4.25, 1.15, "OFF"), T(4.25, 1.45, "DIM"),
        T(1.1, 2.0, "FORM"), Kn(1.1, 2.6, 0.2, -90.0), T(1.65, 3.05, "BRT"),
        T(2.95, 1.95, "MASTER"), T(2.95, 2.2, "NORM"), Kn(2.95, 2.8, 0.25, 0.0),
        T(4.75, 1.95, "AERIAL"), T(4.75, 2.2, "REFUELING"), Kn(4.75, 2.8, 0.2, -90.0),
    ]),
    p(L, 20.0, 2.25, 5.75, 4.125, false, &[ // MANUAL TRIM
        T(1.2, 0.3, "TRIM/AP DISC"), Tg(1.0, 0.9, 1), T(1.0, 1.45, "DISC"),
        T(3.05, 0.3, "NOSE DN"), Sc(3.0, 1.4, 1.6, true), Wh(3.65, 1.4, true), T(3.05, 2.5, "NOSE UP"), T(3.4, 2.8, "PITCH"),
        T(0.75, 3.2, "L WING"), T(1.95, 3.2, "R WING"), Sc(1.35, 3.55, 1.7, false), Wh(1.35, 4.35, false), T(1.35, 4.95, "ROLL TRIM"),
        T(2.75, 3.75, "L"), T(3.65, 3.75, "R"), Kn(3.2, 4.25, 0.3, 0.0), T(3.2, 4.95, "YAW TRIM"),
        T(2.06, 5.5, "MANUAL TRIM"),
    ]),
    p(L, 20.0, 6.375, 5.75, 4.875, false, &[ // FLT CONTROL
        T(1.0, 0.3, "ALT FLAPS"), T(1.0, 0.55, "NORM"), Tg(1.0, 1.0, 1), T(1.0, 1.45, "EXTEND"),
        T(2.45, 0.4, "FLCS"), Lt(2.45, 0.75, 0.62, 0.28, "RUN"), Lt(2.45, 1.1, 0.62, 0.28, "FAIL"),
        T(3.85, 0.3, "BIT"), Tg(3.85, 0.95, -1), T(3.85, 1.5, "OFF"),
        T(1.0, 2.0, "MANUAL TF"), T(1.0, 2.25, "FLYUP"), Tg(1.0, 2.8, 1), T(1.0, 3.35, "DISABLE"),
        T(3.85, 2.0, "FLCS"), T(3.85, 2.25, "RESET"), Tg(3.85, 2.8, -1), T(3.85, 3.35, "OFF"),
        T(1.0, 3.9, "DIGITAL"), T(1.0, 4.15, "BACKUP"), Tg(1.0, 4.65, -1), T(1.0, 5.15, "OFF"),
        T(3.85, 3.9, "LE FLAPS"), T(3.85, 4.15, "AUTO"), Tg(3.85, 4.6, 1), T(3.85, 5.05, "LOCK"),
        T(2.44, 5.52, "FLT CONTROL"),
    ]),
    p(L, 25.75, 5.75, 3.0, 5.75, false, &[ // TEST
        V(5.55, "TEST"),
        T(0.95, 0.25, "FIRE &"), T(0.95, 0.5, "OHEAT"), Pb(0.95, 0.95, 0.13), T(0.95, 1.38, "DETECT"),
        T(2.3, 0.3, "OXY QTY"), Tg(2.3, 0.85, -1), T(2.3, 1.38, "OFF"),
        T(3.72, 0.3, "FLCS PWR"), Lt(3.55, 0.7, 0.32, 0.26, "A"), Lt(3.9, 0.7, 0.32, 0.26, "B"), Lt(3.55, 1.0, 0.32, 0.26, "C"), Lt(3.9, 1.0, 0.32, 0.26, "D"),
        T(4.85, 0.3, "TEST"), Tg(4.85, 0.85, 0), T(4.85, 1.38, "MAINT"),
        T(0.95, 1.75, "MAL & IND"), T(0.95, 2.0, "LTS"), Pb(0.95, 2.5, 0.13),
        T(2.15, 1.8, "PROBE HEAT"), Tg(2.15, 2.35, 0), T(2.15, 2.82, "TEST"),
        T(3.9, 1.8, "EPU/GEN"), Tg(3.9, 2.35, -1), T(3.9, 2.82, "OFF"),
    ]),
    p(L, 25.75, 0.0, 3.0, 5.75, false, &[ // ANTI-G
        T(2.9, 0.65, "ANTI-G"), Pb(2.9, 1.45, 0.22), T(2.9, 2.15, "TEST"),
    ]),
    // ---------------------------------------------- right console, forward section
    p(RT, 0.0, 2.5, 1.5, 5.75, false, &[ // SNSR PWR
        V(0.22, "SNSR PWR"),
        T(1.15, 0.2, "LEFT"), T(1.15, 0.42, "HDPT"), Tg(1.15, 0.95, 1), T(2.4, 0.2, "RIGHT"), T(2.4, 0.42, "HDPT"), Tg(2.4, 0.95, 1),
        T(3.65, 0.3, "FCR"), Tg(3.65, 0.95, 1), T(4.9, 0.3, "RDR ALT"), Tg(4.9, 0.95, 1),
    ]),
    p(RT, 1.5, 2.5, 2.625, 5.75, false, &[ // HUD
        V(0.22, "HUD"),
        T(1.15, 0.25, "VV/VAH"), Tg(1.15, 0.72, 1), T(2.4, 0.25, "ATT/FPM"), Tg(2.4, 0.72, 1),
        T(3.65, 0.15, "DED"), T(3.65, 0.38, "DATA"), Tg(3.65, 0.72, 1), T(4.9, 0.15, "DEPR"), T(4.9, 0.38, "RET"), Tg(4.9, 0.72, 0),
        T(1.15, 1.45, "CAS"), Tg(1.15, 1.95, 1), T(2.4, 1.45, "RADAR"), Tg(2.4, 1.95, 0),
        T(3.65, 1.45, "DAY"), Tg(3.65, 1.95, 1), T(4.9, 1.45, "TEST"), Tg(4.9, 1.95, -1),
    ]),
    p(RT, 4.125, 2.5, 5.625, 5.75, false, &[ // interior LIGHTING
        V(0.22, "LIGHTING"),
        Ln(0.7, 0.45, 2.3, 0.45), T(3.05, 0.45, "PRIMARY"), Ln(3.8, 0.45, 5.4, 0.45),
        T(1.3, 0.85, "CONSOLES"), Kn(1.3, 1.45, 0.25, 30.0), T(2.85, 0.85, "INST PNL"), Kn(2.85, 1.45, 0.25, 30.0),
        T(4.45, 0.75, "DATA ENTRY"), T(4.45, 1.0, "DISPLAY"), Kn(4.45, 1.55, 0.25, 30.0),
        Ln(0.7, 2.35, 2.5, 2.35), T(3.05, 2.35, "FLOOD"), Ln(3.6, 2.35, 5.4, 2.35),
        T(1.3, 2.75, "CONSOLES"), Kn(1.3, 3.35, 0.25, -90.0), T(2.85, 2.75, "INST PNL"), Kn(2.85, 3.35, 0.25, -90.0),
        T(4.45, 2.75, "MAL & IND LTS"), T(4.45, 3.0, "BRT"), Tg(4.45, 3.45, 1), T(4.45, 3.9, "DIM"),
        T(1.3, 3.95, "OFF   BRT"), T(2.85, 3.95, "OFF   BRT"),
    ]),
    p(RT, 9.75, 2.5, 2.25, 5.75, false, &[ // AIR COND
        V(0.22, "AIR COND"),
        T(1.55, 0.3, "AUTO"), T(0.75, 0.75, "COOL"), T(2.35, 0.75, "WARM"), Kn(1.55, 1.2, 0.3, -40.0), T(0.85, 1.7, "OFF"), T(2.25, 1.7, "MAN"), T(1.55, 2.0, "TEMP"),
        T(4.2, 0.3, "NORM"), T(3.4, 0.75, "OFF"), T(5.0, 0.75, "DUMP"), Kn(4.2, 1.2, 0.3, 0.0), T(5.0, 1.65, "RAM"), T(4.2, 2.0, "AIR SOURCE"),
    ]),
    p(RT, 3.75, 0.0, 1.875, 2.5, false, &[ // ZEROIZE
        T(1.25, 0.28, "ZEROIZE"), T(0.5, 1.0, "OFP"), Gd(1.25, 1.0, false), T(2.0, 1.0, "DATA"), T(1.25, 1.65, "OFF"),
    ]),
    p(RT, 9.0, 0.0, 2.25, 2.5, false, &[ // VOICE MESSAGE
        T(1.25, 0.3, "VOICE"), T(1.25, 0.55, "MESSAGE"), Tg(1.25, 1.2, 1), T(1.25, 1.85, "INHIBIT"),
    ]),
    p(RT, 0.0, 8.25, 5.25, 1.5, false, &[ // NUCLEAR CONSENT
        V(0.25, "NUCLEAR CONSENT"), T(0.95, 1.8, "ARM"), Gd(0.95, 2.6, false), T(0.95, 3.45, "OFF"),
    ]),
    // ---------------------------------------------- right console, aft section
    p(RT, 12.0, 2.5, 5.75, 3.375, true, &[ // OXYGEN REGULATOR
        T(0.85, 0.3, "EMER"), Lv(0.85, 1.6, O2_RED), T(0.85, 2.9, "NORMAL"),
        T(1.95, 0.3, "100%"), Lv(1.95, 1.6, O2_WHITE), T(1.95, 2.9, "NORM"),
        T(3.05, 0.3, "ON"), Lv(3.05, 1.6, O2_GREEN), T(3.05, 2.9, "OFF"),
        T(4.55, 0.3, "FLOW"), Dp(4.55, 0.72, 0.9, 0.42, ""), Gg(4.55, 1.95, 0.55), T(4.55, 2.75, "PSI"),
        T(2.6, 3.2, "OXYGEN REGULATOR"),
    ]),
    p(RT, 12.0, 7.375, 5.75, 3.75, true, &[ // SECURE VOICE (KY-58)
        T(2.875, 0.25, "SECURE VOICE"),
        T(1.0, 0.6, "MODE"), Kn(1.0, 1.2, 0.3, -30.0), T(0.45, 1.75, "P"), T(1.0, 1.85, "C"), T(1.55, 1.75, "LD"),
        Kn(1.0, 2.75, 0.3, 30.0), T(1.0, 3.35, "FILL"),
        Cn(2.75, 1.9, 0.42),
        T(4.6, 0.6, "POWER"), Kn(4.6, 1.2, 0.26, 0.0), T(4.6, 1.75, "ON"),
        T(4.6, 2.3, "DELAY"), Tg(4.6, 2.8, 1), T(4.6, 3.35, "PLAIN"),
    ]),
    p(RT, 17.75, 2.5, 5.75, 4.125, true, &[ // AVIONIC POWER
        T(1.15, 0.3, "INS"), T(0.4, 0.75, "OFF"), T(1.9, 0.75, "NAV"), Kn(1.15, 1.25, 0.36, 60.0), T(0.45, 1.85, "STOR"), T(1.85, 1.85, "ATT"),
        T(1.15, 2.45, "MIDS LVT"), Kn(1.15, 3.0, 0.26, 0.0), T(0.5, 3.45, "ZERO"), T(1.8, 3.45, "ON"),
        T(3.2, 0.4, "MMC"), Tg(3.2, 0.95, 1), T(4.6, 0.4, "ST STA"), Tg(4.6, 0.95, 1),
        T(3.2, 1.5, "MFD"), Tg(3.2, 2.05, 1), T(4.6, 1.5, "UFC"), Tg(4.6, 2.05, 1),
        T(3.2, 2.6, "GPS"), Tg(3.2, 3.15, 1), T(4.6, 2.6, "DL"), Tg(4.6, 3.15, 1),
        T(2.875, 3.9, "AVIONIC POWER"),
    ]),
    p(RT, 17.75, 10.0, 5.75, 1.5, true, &[ // ENGINE ANTI ICE and ANT SEL
        T(1.3, 0.25, "ANTI ICE"), Tg(1.3, 0.85, 0), T(1.3, 1.3, "ENGINE"),
        T(3.95, 0.25, "ANT SEL"), Tg(3.4, 0.85, 0), Tg(4.5, 0.85, 0), T(3.4, 1.3, "IFF"), T(4.5, 1.3, "UHF"),
    ]),
    p(RT, 23.5, 2.5, 5.75, 4.5, true, &[ // DTU
        T(2.875, 0.3, "DTU"), Ct(2.875, 2.15, 4.1, 2.8), T(2.875, 4.1, "DTC"),
    ]),
];

/// Cover plates over the empty Dzus spaces: side, s, l, len, wid (inches).
#[rustfmt::skip]
const BLANKS: [(f64, f64, f64, f64, f64); 18] = [
    (L, 9.375, 10.875, 4.875, 1.825), (L, 14.25, 0.0, 5.75, 1.5), (L, 14.25, 10.875, 5.75, 1.825),
    (L, 20.0, 0.0, 5.75, 2.25), (L, 20.0, 11.25, 5.75, 1.45), (L, 25.75, 11.5, 3.0, 1.2), (L, 28.75, 0.0, LEN - 28.75, WID),
    (RT, 5.625, 0.0, 3.375, 2.5), (RT, 11.25, 0.0, 0.75, 2.5), (RT, 5.25, 8.25, 6.75, 1.5), (RT, 0.0, 9.75, 12.0, 2.95),
    (RT, 12.0, 0.0, 17.25, 2.5), (RT, 12.0, 5.875, 5.75, 1.5), (RT, 12.0, 11.125, 5.75, 1.575),
    (RT, 17.75, 6.625, 5.75, 3.375), (RT, 17.75, 11.5, 5.75, 1.2), (RT, 23.5, 7.0, 5.75, 5.7), (RT, 29.25, 0.0, LEN - 29.25, WID),
];

// ================================================================ placement
fn k(side: f64) -> usize {
    if side < 0.0 { 0 } else { 1 }
}

/// Console top axes at `s` metres aft of the front edge: outboard across the
/// top, aft along it, and the normal.
fn axes(side: f64, s: f64) -> (V3, V3, V3) {
    let lat = v3(side * ROLL.cos(), ROLL.sin(), 0.0);
    let aft = if s < FWD[k(side)] { v3(0.0, -PITCH.sin(), PITCH.cos()) } else { V3::Z };
    let n = lat.cross(aft);
    (lat, aft, if n.y < 0.0 { -n } else { n })
}

/// Point on the console top `s` aft of the front edge and `l` outboard of
/// the inboard edge, both in metres along the surface.
fn top(side: f64, s: f64, l: f64) -> V3 {
    let i = k(side);
    let fwd = v3(0.0, -PITCH.sin(), PITCH.cos());
    let along = if s <= FWD[i] { fwd * s } else { fwd * FWD[i] + V3::Z * (s - FWD[i]) };
    v3(side * X_IN, Y_FRONT[i], FRONT_Z) + along + axes(side, 0.0).0 * l
}

/// Height of the console top at body (x, z), for mounts standing on it.
pub(super) fn top_y(x: f64, z: f64) -> f64 {
    let side = x.signum();
    let i = k(side);
    let zf = FRONT_Z + FWD[i] * PITCH.cos();
    let y0 = if z < zf { Y_FRONT[i] - (z - FRONT_Z) * PITCH.tan() } else { Y_FRONT[i] - FWD[i] * PITCH.sin() };
    y0 + (x.abs() - X_IN).max(0.0) * ROLL.tan()
}

/// Face frame of a plate at the top-left corner of its legends: x along the
/// legends, y up them, z out of the console; and the face size in inches.
fn plate_frame(side: f64, s: f64, l: f64, len: f64, wid: f64, rot: bool) -> (Frame, f64, f64) {
    let (s0, l0, sl, wl) = (s * IN, l * IN, len * IN, wid * IN);
    let (lat, aft, n) = axes(side, s0 + 1e-4);
    let (o, x, y) = match (rot, side < 0.0) {
        (false, true) => (top(side, s0, l0 + wl), -lat, -aft),
        (false, false) => (top(side, s0, l0), lat, -aft),
        (true, true) => (top(side, s0 + sl, l0 + wl), -aft, lat),
        (true, false) => (top(side, s0, l0 + wl), aft, lat),
    };
    let (w, h) = if rot { (len, wid) } else { (wid, len) };
    (Frame { o: o + n * LIFT, x, y, z: n }, w, h)
}

fn frame(p: &Panel) -> (Frame, f64, f64) {
    plate_frame(p.side, p.s, p.l, p.len, p.wid, p.rot)
}

/// Atlas rectangles of the plates, shelf-packed tallest first.
fn rects() -> Vec<R> {
    let size: Vec<(f32, f32)> = PANELS.iter().map(|p| {
        let (w, h) = if p.rot { (p.len, p.wid) } else { (p.wid, p.len) };
        ((w as f32 * PPI).round(), (h as f32 * PPI).round())
    }).collect();
    let mut order: Vec<usize> = (0..PANELS.len()).collect();
    order.sort_by(|&a, &b| size[b].1.total_cmp(&size[a].1));
    let mut out = vec![R { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }; PANELS.len()];
    let (mut x, mut y, mut shelf) = (AREA.x, AREA.y, 0.0f32);
    for i in order {
        let (w, h) = size[i];
        if x + w > AREA.x + AREA.w {
            (x, y, shelf) = (AREA.x, y + shelf + 1.0, 0.0);
        }
        out[i] = R { x, y, w, h };
        x += w + 1.0;
        shelf = shelf.max(h);
    }
    out
}

// ================================================================ geometry
/// Closed prism from `a` to `b`, radius `r0` at `a` and `r1` at `b`.
#[allow(clippy::too_many_arguments)]
fn prism(m: &mut Mesh, a: V3, b: V3, r0: f64, r1: f64, sides: usize, hint: V3, col: Rgb) {
    let ax = (b - a).norm();
    let e1 = (hint - ax * hint.dot(ax)).norm();
    let e2 = ax.cross(e1);
    let ring = |c: V3, r: f64| -> Vec<V3> {
        (0..sides).map(|i| {
            let t = (i as f64 + 0.5) * TAU / sides as f64;
            c + (e1 * t.cos() + e2 * t.sin()) * r
        }).collect()
    };
    let (p, q) = (ring(a, r0), ring(b, r1));
    for i in 0..sides {
        let j = (i + 1) % sides;
        m.quad(p[i], p[j], q[j], q[i], col);
    }
    m.polygon(&p, col);
    m.polygon(&q, col);
}

/// Box along a frame's axes without bevels: six quads.
fn cuboid(m: &mut Mesh, f: &Frame, h: V3, col: Rgb) {
    let c = |x: f64, y: f64, z: f64| f.p(x * h.x, y * h.y, z * h.z);
    for (a, b, cc, d) in [
        (c(-1., -1., 1.), c(1., -1., 1.), c(1., 1., 1.), c(-1., 1., 1.)),
        (c(-1., -1., -1.), c(-1., 1., -1.), c(1., 1., -1.), c(1., -1., -1.)),
        (c(-1., -1., -1.), c(1., -1., -1.), c(1., -1., 1.), c(-1., -1., 1.)),
        (c(-1., 1., -1.), c(-1., 1., 1.), c(1., 1., 1.), c(1., 1., -1.)),
        (c(-1., -1., -1.), c(-1., -1., 1.), c(-1., 1., 1.), c(-1., 1., -1.)),
        (c(1., -1., -1.), c(1., 1., -1.), c(1., 1., 1.), c(1., -1., 1.)),
    ] {
        m.quad(a, b, cc, d, col);
    }
}

/// Bat-handle toggle on its hex nut, thrown along `throw`.
fn toggle(m: &mut Mesh, f: &Frame, b: V3, throw: V3) {
    prism(m, b, b + f.z * 0.005, 0.0062, 0.0056, 6, f.y, NUT);
    let tip = b + f.z * 0.023 + throw * 0.008;
    prism(m, b + f.z * 0.004, tip, 0.0022, 0.0017, 4, f.y, LEVER);
    let d = (tip - b).norm();
    prism(m, tip - d * 0.004, tip + d * 0.002, 0.0033, 0.0030, 5, f.y, LEVER);
}

fn item_geometry(m: &mut Mesh, f: &Frame, it: &It) {
    let at = |u: f64, v: f64| f.p(u * IN, -v * IN, 0.0);
    match *it {
        Tg(u, v, pos) => toggle(m, f, at(u, v), f.y * pos as f64),
        Tw(u, v, pos) => toggle(m, f, at(u, v), f.x * pos as f64),
        Gd(u, v, red) => {
            // Closed flip cover: two cheeks and a lid over the switch.
            let col = if red { GUARD_RED } else { GUARD_BLACK };
            let c = at(u, v);
            for s in [-1.0, 1.0] {
                cuboid(m, &Frame { o: c + f.x * (s * 0.0085) + f.z * 0.007, ..*f }, v3(0.0015, 0.011, 0.007), col);
            }
            cuboid(m, &Frame { o: c + f.z * 0.0145, ..*f }, v3(0.010, 0.011, 0.0015), col.scale(1.1));
        }
        Kn(u, v, r, ang) => {
            let (c, r) = (at(u, v), r * IN);
            let h = (0.005 + r * 0.7).min(0.013);
            prism(m, c, c + f.z * h, r, r * 0.86, 8, f.y, KNOB);
            // White index line across the cap.
            let a = ang * DEG;
            let d = f.y * a.cos() + f.x * a.sin();
            let w = (f.x * a.cos() - f.y * a.sin()) * 0.0011;
            let t = c + f.z * (h + 0.0004);
            m.quad(t - w, t + d * r * 0.8 - w, t + d * r * 0.8 + w, t + w, Rgb(0.85, 0.85, 0.80));
        }
        Pb(u, v, r) => prism(m, at(u, v), at(u, v) + f.z * 0.004, r * IN, r * IN * 0.85, 6, f.y, KNOB.scale(1.6)),
        Sq(u, v, w, h, _) => {
            // Raised rim round the lit cap, which stays the atlas face.
            let c = at(u, v) + f.z * 0.002;
            let (hw, hh) = (w * IN * 0.5, h * IN * 0.5);
            for s in [-1.0, 1.0] {
                cuboid(m, &Frame { o: c + f.x * (s * (hw + 0.0012)), ..*f }, v3(0.0012, hh + 0.0024, 0.002), KNOB.scale(1.3));
                cuboid(m, &Frame { o: c + f.y * (s * (hh + 0.0012)), ..*f }, v3(hw, 0.0012, 0.002), KNOB.scale(1.3));
            }
        }
        Wh(u, v, along) => {
            // Mostly sunk in its slot: the axis lies across the rolling direction.
            let ax = if along { f.x } else { f.y };
            let c = at(u, v) + f.z * 0.002;
            prism(m, c - ax * 0.004, c + ax * 0.004, 0.017, 0.017, 10, f.z, KNOB.scale(1.4));
        }
        Lv(u, v, col) => {
            // Paddle lever leaning up the face, as on the CRU-73 regulator.
            let b = at(u, v);
            let up = (f.z + f.y * 0.35).norm();
            let side = f.x;
            let fr = Frame { o: b + up * 0.012, x: side, y: up.cross(side), z: up };
            cuboid(m, &fr, v3(0.0035, 0.0025, 0.012), col.scale(0.8));
            cuboid(m, &Frame { o: b + up * 0.026, ..fr }, v3(0.0085, 0.0045, 0.005), col);
        }
        Gg(u, v, r) => m.ring(&Frame { o: at(u, v) + f.z * 0.004, ..*f }, r * IN, r * IN + 0.003, 0.004, NUT.scale(0.7), 12),
        Cn(u, v, r) => m.ring(&Frame { o: at(u, v) + f.z * 0.006, ..*f }, r * IN * 0.75, r * IN, 0.006, NUT.scale(0.8), 12),
        Rl(u, v) => cuboid(m, &Frame { o: at(u, v) + f.z * 0.009, ..*f }, v3(0.0015, 0.016, 0.009), GUARD_BLACK),
        Ct(u, v, w, h) => {
            // Cartridge seated in the receptacle, with its pull handle.
            let c = at(u, v);
            cuboid(m, &Frame { o: c + f.z * 0.006, ..*f }, v3(w * IN * 0.42, h * IN * 0.42, 0.006), GUARD_BLACK.scale(1.4));
            cuboid(m, &Frame { o: c + f.z * 0.014 + f.x * (-w * IN * 0.28), ..*f }, v3(0.004, h * IN * 0.3, 0.003), NUT);
        }
        _ => {}
    }
}

/// Plate face as a textured quad over `rect`.
fn face(m: &mut Mesh, f: &Frame, w: f64, h: f64, rect: R) {
    let (u0, v0, u1, v1) = uv(rect);
    let (w, h) = (w * IN, h * IN);
    m.quad_uv(f.p(0.0, 0.0, 0.0), f.p(w, 0.0, 0.0), f.p(w, -h, 0.0), f.p(0.0, -h, 0.0), [(u0, v0), (u1, v0), (u1, v1), (u0, v1)]);
}

/// Both consoles: structure, lightplates with their hardware, cover plates
/// and the throttle quadrant housing. The caller facets the mesh.
pub fn build(m: &mut Mesh) {
    for side in [-1.0, 1.0] {
        // Top, in its two sections, and the inboard, outboard, front and aft faces down to the floor.
        let s1 = FWD[k(side)];
        let (wl, len) = (WID * IN, LEN * IN);
        let pts = [(0.0, s1), (s1, len)];
        for (a, b) in pts {
            m.quad(top(side, a, 0.0), top(side, a, wl), top(side, b, wl), top(side, b, 0.0), STRUCT);
            for l in [0.0, wl] {
                let (p, q) = (top(side, a, l), top(side, b, l));
                m.quad(p, q, v3(q.x, 0.22, q.z), v3(p.x, 0.22, p.z), STRUCT.scale(0.7));
            }
        }
        for s in [0.0, len] {
            let (p, q) = (top(side, s, 0.0), top(side, s, wl));
            m.quad(p, q, v3(q.x, 0.22, q.z), v3(p.x, 0.22, p.z), STRUCT.scale(0.6));
        }
    }

    let rect = rects();
    for (p, r) in PANELS.iter().zip(rect) {
        let (f, w, h) = frame(p);
        face(m, &f, w, h, r);
        for it in p.items {
            item_geometry(m, &f, it);
        }
    }
    for &(side, s, l, len, wid) in BLANKS.iter() {
        let (f, w, h) = plate_frame(side, s, l, len, wid, false);
        face(m, &f, w, h, BLANK);
    }

    // Throttle quadrant housing along the outboard side of the left
    // console's forward section, raised to carry the lever slot.
    let (sa, sb, la, lb) = (0.0, 9.25 * IN, 8.75 * IN, WID * IN);
    let hi = |s: f64, l: f64| {
        let p = top(-1.0, s, l);
        v3(p.x, QUADRANT_TOP, p.z)
    };
    let housing = Rgb(0.24, 0.255, 0.26);
    m.quad(hi(sa, la), hi(sa, lb), hi(sb, lb), hi(sb, la), housing);
    m.quad(top(-1.0, sa, la), hi(sa, la), hi(sb, la), top(-1.0, sb, la), housing.scale(0.8));
    m.quad(top(-1.0, sa, la), top(-1.0, sa, lb), hi(sa, lb), hi(sa, la), housing.scale(0.7));
    m.quad(top(-1.0, sb, la), hi(sb, la), hi(sb, lb), top(-1.0, sb, lb), housing.scale(0.75));
    // Slot for the lever, and the friction control on the aft face.
    let sx = SLOT_X.abs();
    let slot = |s: f64, dx: f64| v3(-(sx + dx), QUADRANT_TOP + 0.0008, FRONT_Z + s);
    m.quad(slot(0.012, -0.007), slot(0.012, 0.007), slot(sb - 0.012, 0.007), slot(sb - 0.012, -0.007), Rgb(0.02, 0.02, 0.022));
    let fr = v3(-(sx + 0.035), QUADRANT_TOP - 0.03, FRONT_Z + sb);
    prism(m, fr, fr + V3::Z * 0.012, 0.012, 0.011, 8, V3::Y, KNOB);
}

// ================================================================ legends
fn lightplate(cv: &mut Canvas, r: R) {
    cv.rect(r.x, r.y, r.w, r.h, FACE);
    cv.stroke_rect(r.x + 0.5, r.y + 0.5, r.w - 1.0, r.h - 1.0, 1.0, SEAM);
    // Dzus fasteners at the corners, 3/16 in in from the edges.
    let d = (0.19 * PPI).round();
    for (x, y) in [(d, d), (r.w - d, d), (d, r.h - d), (r.w - d, r.h - d)] {
        cv.disc(r.x + x, r.y + y, 3.5, HOLE);
        cv.line(r.x + x - 2.5, r.y + y, r.x + x + 2.5, r.y + y, 1.0, rgba(0x5b, 0x61, 0x60, 1.0));
    }
}

fn item_legend(cv: &mut Canvas, r: R, h: f64, it: &It) {
    let px = |u: f64| (r.x + u as f32 * PPI).round();
    let py = |v: f64| (r.y + v as f32 * PPI).round();
    let sz = |w: f64| (w as f32 * PPI).round();
    match *it {
        T(u, v, s) => cv.text(s, px(u), py(v), 1.0, WHITE, Align::Center),
        V(u, s) => {
            // Rotated a quarter turn: glyph columns land on whole pixels.
            let w = Canvas::text_width(s, 1.0);
            cv.set_transform((r.x + u as f32 * PPI - 3.5).round(), py(h * 0.5), -std::f32::consts::FRAC_PI_2);
            cv.text(s, -(w * 0.5).round(), 3.5, 1.0, WHITE, Align::Left);
            cv.reset_transform();
        }
        Ln(u0, v0, u1, v1) => cv.line(px(u0), py(v0), px(u1), py(v1), 1.0, DIM),
        Tg(u, v, _) | Tw(u, v, _) => cv.disc(px(u), py(v), 0.17 * PPI, HOLE),
        Gd(u, v, _) => cv.rect(px(u) - 0.42 * PPI, py(v) - 0.55 * PPI, 0.84 * PPI, 1.1 * PPI, HOLE),
        Kn(u, v, rr, _) => {
            cv.circle(px(u), py(v), (rr as f32 + 0.07) * PPI, 1.0, DIM);
            for a in [-120.0f32, -60.0, 0.0, 60.0, 120.0] {
                let t = (a - 90.0).to_radians();
                let (r0, r1) = ((rr as f32 + 0.08) * PPI, (rr as f32 + 0.16) * PPI);
                cv.line(px(u) + t.cos() * r0, py(v) + t.sin() * r0, px(u) + t.cos() * r1, py(v) + t.sin() * r1, 1.0, WHITE);
            }
        }
        Pb(u, v, rr) => cv.circle(px(u), py(v), (rr as f32 + 0.05) * PPI, 1.0, DIM),
        Lt(u, v, w, hh, s) => {
            cv.rect(px(u) - sz(w) * 0.5, py(v) - sz(hh) * 0.5, sz(w), sz(hh), AMBER_OFF);
            let lines: Vec<&str> = s.split('|').collect();
            for (i, l) in lines.iter().enumerate() {
                let dy = (i as f32 - (lines.len() as f32 - 1.0) * 0.5) * 9.0;
                cv.text(l, px(u), py(v) + dy, 1.0, AMBER_INK, Align::Center);
            }
        }
        Gl(u, v, rr) => {
            cv.disc(px(u), py(v), rr as f32 * PPI + 1.0, HOLE);
            cv.disc(px(u), py(v), rr as f32 * PPI - 1.0, rgba(0x1e, 0x4a, 0x2a, 1.0));
        }
        Dp(u, v, w, hh, s) => {
            cv.rect(px(u) - sz(w) * 0.5, py(v) - sz(hh) * 0.5, sz(w), sz(hh), rgba(0x03, 0x07, 0x05, 1.0));
            cv.stroke_rect(px(u) - sz(w) * 0.5, py(v) - sz(hh) * 0.5, sz(w), sz(hh), 1.0, SEAM);
            cv.text(s, px(u), py(v), 1.0, super::GREEN, Align::Center);
        }
        Sq(u, v, w, hh, s) => {
            cv.rect(px(u) - sz(w) * 0.5, py(v) - sz(hh) * 0.5, sz(w), sz(hh), rgba(0x14, 0x17, 0x18, 1.0));
            cv.text(s, px(u), py(v), 1.0, WHITE, Align::Center);
        }
        Wh(u, v, along) => {
            let (w, hh) = if along { (sz(0.3), sz(1.1)) } else { (sz(1.1), sz(0.3)) };
            cv.rect(px(u) - w * 0.5, py(v) - hh * 0.5, w, hh, HOLE);
        }
        Lv(u, v, _) => cv.rect(px(u) - 3.0, py(v) - sz(1.0), 6.0, sz(1.6), HOLE),
        Gg(u, v, rr) => {
            let (x, y, rad) = (px(u), py(v), rr as f32 * PPI);
            cv.disc(x, y, rad, rgba(0x05, 0x07, 0x07, 1.0));
            for i in 0..=10 {
                let t = ((-225.0 + i as f32 * 27.0) - 0.0).to_radians();
                cv.line(x + t.cos() * rad * 0.7, y + t.sin() * rad * 0.7, x + t.cos() * rad * 0.92, y + t.sin() * rad * 0.92, 1.0, WHITE);
            }
            let t = (-60.0f32).to_radians();
            cv.line(x, y, x + t.cos() * rad * 0.8, y + t.sin() * rad * 0.8, 2.0, WHITE);
        }
        Cn(u, v, rr) => {
            cv.disc(px(u), py(v), rr as f32 * PPI, HOLE);
            for i in 0..6 {
                let t = (i as f32 * 60.0).to_radians();
                cv.disc(px(u) + t.cos() * 5.0, py(v) + t.sin() * 5.0, 1.2, DIM);
            }
        }
        Sc(u, v, len, vert) => {
            let (w, hh) = if vert { (sz(0.22), sz(len)) } else { (sz(len), sz(0.22)) };
            let (x0, y0) = (px(u) - w * 0.5, py(v) - hh * 0.5);
            cv.rect(x0, y0, w, hh, HOLE);
            for i in 0..=6 {
                let t = i as f32 / 6.0;
                if vert {
                    cv.line(x0 - 3.0, y0 + hh * t, x0, y0 + hh * t, 1.0, WHITE);
                } else {
                    cv.line(x0 + w * t, y0 + hh, x0 + w * t, y0 + hh + 3.0, 1.0, WHITE);
                }
            }
            cv.disc(px(u), py(v), 2.0, WHITE);
        }
        Ct(u, v, w, hh) => cv.rect(px(u) - sz(w) * 0.5, py(v) - sz(hh) * 0.5, sz(w), sz(hh), HOLE),
        Rl(..) => {}
    }
}

/// Draws every lightplate into the right half of the atlas.
pub fn draw(cv: &mut Canvas) {
    cv.rect(BLANK.x, BLANK.y, BLANK.w, BLANK.h, FACE);
    for (p, r) in PANELS.iter().zip(rects()) {
        lightplate(cv, r);
        let h = if p.rot { p.wid } else { p.len };
        for it in p.items {
            item_legend(cv, r, h, it);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxes() -> Vec<(f64, f64, f64, f64, f64)> {
        let mut b: Vec<_> = PANELS.iter().map(|p| (p.side, p.s, p.l, p.len, p.wid)).collect();
        b.extend(BLANKS.iter().copied());
        b
    }

    #[test]
    fn plates_fit_one_section_and_do_not_overlap() {
        let b = boxes();
        for (i, &(side, s, l, len, wid)) in b.iter().enumerate() {
            let f = FWD[k(side)] / IN;
            assert!(s + len <= f + 1e-6 || s >= f - 1e-6, "plate {i} straddles the section break");
            assert!(l >= 0.0 && l + wid <= WID + 1e-6 && s >= 0.0 && s + len <= LEN + 1e-6, "plate {i} off the console");
            for (j, &(side2, s2, l2, len2, wid2)) in b.iter().enumerate().skip(i + 1) {
                let clear = side != side2 || s + len <= s2 + 1e-6 || s2 + len2 <= s + 1e-6 || l + wid <= l2 + 1e-6 || l2 + wid2 <= l + 1e-6;
                assert!(clear, "plates {i} and {j} overlap");
            }
        }
    }

    #[test]
    fn plates_are_dzus_sized() {
        // 5.75 in across the rails, heights in 3/8 in steps.
        for p in PANELS.iter() {
            let steps = |x: f64| ((x / 0.375) - (x / 0.375).round()).abs() < 1e-6;
            assert!(steps(p.len) || steps(p.wid), "panel at s {} l {} is not on the 3/8 in grid", p.s, p.l);
        }
    }

    #[test]
    fn atlas_packing_fits() {
        let r = rects();
        for (i, a) in r.iter().enumerate() {
            assert!(a.x >= AREA.x && a.y >= AREA.y && a.x + a.w <= AREA.x + AREA.w && a.y + a.h <= AREA.y + AREA.h, "panel {i} outside the atlas area");
            for b in r.iter().skip(i + 1) {
                assert!(a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y);
            }
        }
    }

    #[test]
    fn tops_lean_inboard_and_match_the_reference_heights() {
        let eye = crate::airframe::EYE;
        for side in [-1.0, 1.0] {
            let (_, _, n) = axes(side, 0.5);
            assert!(n.x * side < 0.0 && (n.y - ROLL.cos()).abs() < 1e-9);
            // Inboard edge about 0.76 m under the design eye aft of the forward section.
            let y = top_y(side * X_IN, FRONT_Z + 0.6);
            assert!((eye.y - y - 0.765).abs() < 0.01);
            assert!((top(side, 0.6, 0.0).y - y).abs() < 1e-9);
        }
    }
}
