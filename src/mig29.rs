//! MiG-29S (9.13S) Fulcrum-C, low-poly, measured off a scale three-view: 17.32 m
//! from the pitot tip to the stabilator tips, 11.36 m span, 4.73 m tall on
//! its gear. A slim forward fuselage with a white radome, the IRST ball and
//! a bubble canopy runs back into the 9.13's humped spine and the beaver tail
//! between the nozzles. Broad, curved leading-edge root extensions flow into
//! the flat lifting centre body, which sits on two engine nacelles splayed a
//! little apart and fed by steeply raked intakes under the LERX. The 42
//! degree wings droop 3 degrees from the tail booms, which carry the
//! all-moving stabilators and the twin fins, canted out 6 degrees over their
//! root extensions. Smooth (Gouraud) surfaces in vertex colours: a modern
//! three-tone grey-blue digital (pixel) camouflage laid on by the aircraft
//! shader over the upper skin, a pale belly, bare-metal engine bays and red
//! stars on the fins and under the wings. Body axes as for the F-16 model:
//! x right, y up, z aft, the centre of gravity at the origin.
use crate::math::*;
use crate::mesh::{Frame, Mesh, Rgb};

const LIGHT: Rgb = Rgb(0.69, 0.72, 0.73);
const BELLY: Rgb = Rgb(0.76, 0.80, 0.82);
const RADOME: Rgb = Rgb(0.86, 0.87, 0.83);
const GLASS: Rgb = Rgb(0.13, 0.19, 0.23);
const FRAME: Rgb = Rgb(0.52, 0.58, 0.57);
const DUCT: Rgb = Rgb(0.05, 0.05, 0.06);
const METAL: Rgb = Rgb(0.34, 0.32, 0.30);
const BARE: Rgb = Rgb(0.52, 0.50, 0.47);
const SOOT: Rgb = Rgb(0.17, 0.16, 0.15);
const STAR: Rgb = Rgb(0.78, 0.12, 0.10);
const WHITE: Rgb = Rgb(0.90, 0.90, 0.88);

/// Stabilator pivot (right side; the left mirrors it) and its hinge axis.
pub const STAB_PIVOT: V3 = v3(1.95, 0.36, 6.55);
/// Right nozzle exit (the left mirrors it).
pub const NOZZLE: V3 = v3(1.11, 0.05, 7.25);
/// Gear leg pivots, the axis each swings about and its retraction swing: the nose leg folds
/// aft between the intake ducts, the main legs forward into the wing roots.
pub const GEAR_PIVOTS: [(V3, V3, f64); 3] = [(v3(0.0, 0.14, -1.72), V3::X, -95.0 * DEG), (v3(-1.50, 0.30, 1.95), V3::X, 88.0 * DEG), (v3(1.50, 0.30, 1.95), V3::X, 88.0 * DEG)];

/// Pilot design eye, about 0.2 m under the canopy over the helmet.
pub const EYE: V3 = v3(0.0, 1.38, -3.55);
/// Box the first-person view cuts out of the skin, so the cockpit model
/// shows inside: half width, floor, and from the windscreen to the spine.
#[cfg(test)]
pub const CUTOUT: (f64, f64, f64, f64) = (0.44, 0.55, -4.78, -2.0);

pub struct Fulcrum {
    /// The airframe without its moving parts.
    pub body: Mesh,
    /// Canopy and windscreen, apart so the cockpit view can leave them out.
    pub canopy: Mesh,
    /// Right and left stabilators.
    pub stabs: [Mesh; 2],
    /// Nose, left main and right main gear legs, fully down.
    pub gear: [Mesh; 3],
    /// Upper and lower airbrake petals, closed.
    pub brakes: [Mesh; 2],
}

/// Underwing pylons on each side, inboard to outboard.
const PYLONS: [f64; 3] = [2.55, 3.35, 4.20];

/// Centre of the lower face of the right-hand pylon at span station `x`.
fn pylon(x: f64) -> V3 {
    v3(x, wing_point(1.0, x, 0.42, false).y - 0.24, mix(wing_le(x), wing_te(x), 0.42))
}

/// Offset that carries a store from the F-16 station `at` (as the loadout
/// places it) onto the matching MiG pylon: the wingtip AIM-9s to the outer
/// pylons, under their launch rails, the outboard pair to the middle pylons
/// and the inboard pair to the inner ones.
pub fn store_shift(at: V3) -> V3 {
    let side = if at.x < 0.0 { -1.0 } else { 1.0 };
    let x = at.x.abs();
    let tip = crate::airframe::AIM9_TIP;
    let (from, to) = if x > 4.0 {
        (v3(tip.x, tip.y, tip.z + 1.43), pylon(PYLONS[2]) - V3::Y * 0.15)
    } else if x > 2.9 {
        (v3(3.50, 0.08, 2.35), pylon(PYLONS[1]))
    } else {
        (v3(2.30, 0.08, 1.85), pylon(PYLONS[0]))
    };
    let d = to - from;
    v3(side * d.x, d.y, d.z)
}

/// Catmull-Rom through the rows of `table` (first column z) at `z`.
fn spline<const N: usize>(table: &[[f64; N]], z: f64) -> [f64; N] {
    let mut k = 0;
    while k < table.len() - 2 && z > table[k + 1][0] {
        k += 1;
    }
    let t = clamp((z - table[k][0]) / (table[k + 1][0] - table[k][0]), 0.0, 1.0);
    let (a, b, c, d) = (&table[k.saturating_sub(1)], &table[k], &table[k + 1], &table[(k + 2).min(table.len() - 1)]);
    let mut out = [0.0; N];
    for i in 0..N {
        out[i] = 0.5 * ((2.0 * b[i]) + (-a[i] + c[i]) * t + (2.0 * a[i] - 5.0 * b[i] + 4.0 * c[i] - d[i]) * t * t
            + (-a[i] + 3.0 * b[i] - 3.0 * c[i] + d[i]) * t * t * t);
    }
    out
}

/// Piecewise-linear map of a surface parameter in [0, 1] onto the
/// breakpoints `t`, one cell between each pair, so cell edges (and colour
/// changes) land on the panel lines.
fn steps(t: &[f64], u: f64) -> f64 {
    let n = t.len() - 1;
    let f = clamp(u, 0.0, 1.0) * n as f64;
    let k = (f.floor() as usize).min(n - 1);
    mix(t[k], t[k + 1], f - k as f64)
}

/// NACA 4-digit half thickness at chord fraction `c`, for unit thickness.
fn naca(c: f64) -> f64 {
    5.0 * (0.2969 * c.sqrt() - 0.1260 * c - 0.3516 * c * c + 0.2843 * c.powi(3) - 0.1036 * c.powi(4))
}

/// Upper skin is LIGHT, which `flag_camo` hands to the shader's digital
/// camouflage; the belly stays pale.
fn skin(_p: V3, n: V3) -> Rgb {
    if n.y < -0.4 {
        BELLY
    } else {
        LIGHT
    }
}

/// Marks the vertices painted LIGHT (plain, or darkened on the moving
/// surfaces) with uv (-2, -2): the aircraft shader lays the three-tone
/// digital camouflage over them, in body coordinates, so the pattern runs on
/// across the joins between parts.
fn flag_camo(m: &mut Mesh) {
    for c in m.v.chunks_mut(crate::mesh::STRIDE) {
        let k = c[7] as f64 / LIGHT.1;
        let grey = (c[6] as f64 - LIGHT.0 * k).abs() < 0.01 && (c[8] as f64 - LIGHT.2 * k).abs() < 0.01;
        if grey && k > 0.85 && k < 1.01 {
            c[9] = -2.0;
            c[10] = -2.0;
        }
    }
}

/// Flat polygon facing `out`, wound so its normal points that way.
fn face(m: &mut Mesh, pts: &[V3], out: V3, col: Rgb) {
    let n = (pts[1] - pts[0]).cross(pts[2] - pts[0]);
    if n.dot(out) >= 0.0 {
        m.polygon(pts, col);
    } else {
        let r: Vec<V3> = pts.iter().rev().copied().collect();
        m.polygon(&r, col);
    }
}

/// Red star with a white border, of radius `r` centred on `c`, lying on a
/// plane through `c` with normal `n`, lifted a little off it.
fn star(m: &mut Mesh, c: V3, n: V3, r: f64) {
    let e1 = n.cross(if n.y.abs() > 0.9 { V3::X } else { V3::Y }).norm();
    let e2 = n.cross(e1);
    for (r, lift, col) in [(r * 1.2, 0.012, WHITE), (r, 0.02, STAR)] {
        let o = c + n * lift;
        let p = |k: usize, rad: f64| {
            let a = k as f64 * PI / 5.0;
            o + (e2 * a.cos() + e1 * a.sin()) * rad
        };
        for k in 0..5 {
            face(m, &[o, p(2 * k, r), p(2 * k + 1, r * 0.4)], n, col);
            face(m, &[o, p(2 * k + 1, r * 0.4), p(2 * k + 2, r)], n, col);
        }
    }
}

// ------------------------------------------------------------------ fuselage
// z, half width, centre height, upper and lower half heights, superellipse
// exponents: round at the radome, squarer under the cockpit, then the
// shallow body between the nacelles and the flat beaver tail.
const FUSE: [[f64; 7]; 15] = [
    [-7.50, 0.02, 0.27, 0.02, 0.02, 2.0, 2.0],
    [-7.10, 0.17, 0.30, 0.17, 0.17, 2.0, 2.0],
    [-6.50, 0.30, 0.37, 0.31, 0.31, 2.0, 2.0],
    [-5.80, 0.42, 0.46, 0.46, 0.44, 2.1, 2.2],
    [-5.00, 0.50, 0.55, 0.58, 0.56, 2.2, 2.4],
    [-4.20, 0.56, 0.57, 0.60, 0.56, 2.3, 2.5],
    [-3.20, 0.60, 0.57, 0.58, 0.52, 2.4, 2.4],
    [-2.20, 0.62, 0.58, 0.52, 0.44, 2.5, 2.2],
    [-1.00, 0.62, 0.55, 0.45, 0.40, 2.6, 2.2],
    [0.50, 0.60, 0.52, 0.43, 0.38, 2.6, 2.4],
    [2.00, 0.58, 0.50, 0.37, 0.36, 2.6, 2.6],
    [3.35, 0.52, 0.48, 0.27, 0.32, 2.6, 2.6],
    [5.00, 0.42, 0.45, 0.20, 0.26, 2.5, 2.5],
    [6.10, 0.45, 0.44, 0.21, 0.20, 2.3, 2.3],
    [6.80, 0.42, 0.44, 0.18, 0.18, 2.2, 2.2],
];
const FUSE_Z: (f64, f64) = (-7.50, BRAKE_FRONT);
/// Radome joint.
const RADOME_Z: f64 = -5.80;

/// Dorsal spine behind the canopy: height over the fuselage and half width.
/// The 9.13's "fatback": the spine swells from behind the canopy to past the
/// wing root over the enlarged No. 1 fuel tank and the L-203 jammer.
fn spine(z: f64) -> (f64, f64) {
    let [_, h, w] = spline(&[[-2.40, 0.0, 0.36], [-1.90, 0.45, 0.38], [-1.00, 0.53, 0.42], [0.50, 0.47, 0.42], [2.00, 0.36, 0.38], [3.35, 0.22, 0.32], [5.00, 0.08, 0.26], [6.40, 0.0, 0.22], [6.80, 0.0, 0.22]], z);
    (h.max(0.0), w)
}

fn fuselage_point(z: f64, th: f64) -> V3 {
    let [_, w, yc, hu, hl, nu, nl] = spline(&FUSE, z);
    let (c, s) = (th.cos(), th.sin());
    let n = if s >= 0.0 { nu } else { nl };
    let x = w.max(0.0) * c.signum() * c.abs().powf(2.0 / n);
    let mut y = yc + if s >= 0.0 { hu } else { hl }.max(0.0) * s.signum() * s.abs().powf(2.0 / n);
    if s > 0.0 {
        let (h, ws) = spine(z);
        let k = 1.0 - (x / ws).powi(2);
        if h > 0.0 && k > 0.0 {
            y += h * k.powf(1.5);
        }
    }
    v3(x, y, z)
}

/// Height of the upper fuselage skin at a lateral offset (no spine).
fn fuselage_top_at(z: f64, x: f64) -> f64 {
    let [_, w, yc, hu, _, nu, _] = spline(&FUSE, z);
    let r = (x.abs() / w).min(1.0);
    yc + hu * (1.0 - r.powf(nu)).max(0.0).powf(1.0 / nu)
}

// -------------------------------------------------------------------- canopy
// z, half width at the sill, height of the top.
const CANOPY: [[f64; 3]; 6] = [[-4.80, 0.28, 1.17], [-4.35, 0.40, 1.44], [-3.80, 0.46, 1.66], [-3.10, 0.47, 1.74], [-2.50, 0.44, 1.70], [-1.90, 0.34, 1.56]];
/// Windscreen arch and rear end of the canopy, with a frame cell between.
pub const CANOPY_Z: [f64; 10] = [-4.80, -4.50, -4.20, -4.10, -3.65, -3.20, -2.75, -2.35, -2.12, -1.90];
/// Around the canopy from the right sill to the left, fine enough that the
/// flat cells stay clear of the skin under them: the sill frames and the
/// two windscreen posts (between CANOPY_POST) get their own thin cells.
pub const CANOPY_PHI: [f64; 15] = [0.0, CANOPY_SILL, 0.36, 0.62, CANOPY_POST.0, CANOPY_POST.1, 1.26, PI / 2.0, PI - 1.26, PI - CANOPY_POST.1, PI - CANOPY_POST.0, PI - 0.62, PI - 0.36, PI - CANOPY_SILL, PI];
pub const CANOPY_SILL: f64 = 0.10;
pub const CANOPY_POST: (f64, f64) = (0.86, 0.96);

pub fn canopy_point(z: f64, phi: f64) -> V3 {
    let [_, hw, top] = spline(&CANOPY, z);
    let x = hw * phi.cos();
    let y = fuselage_top_at(z, x) - 0.02 + (top - fuselage_top_at(z, 0.0)).max(0.0) * phi.sin().max(0.0).powf(0.85);
    v3(x, y, z)
}

fn canopy_colour(p: V3, n: V3) -> Rgb {
    let [_, hw, _] = spline(&CANOPY, p.z);
    let phi = clamp(p.x / hw, -1.0, 1.0).acos();
    let sill = phi < CANOPY_SILL || phi > PI - CANOPY_SILL;
    let side = phi.min(PI - phi);
    let post = p.z < CANOPY_Z[2] && side > CANOPY_POST.0 && side < CANOPY_POST.1;
    let arch = p.z > CANOPY_Z[2] && p.z < CANOPY_Z[3];
    if sill || post || arch {
        FRAME
    } else {
        // A paler band along the top, where the glass catches the sky.
        GLASS.scale(1.0 + 0.5 * n.y.max(0.0).powi(4))
    }
}

// --------------------------------------------------- LERX and centre body
// z, half width of the outer edge, height at the fuselage side and at the
// edge, and thickness at the fuselage side. Ahead of the intakes it is the
// thin, drooped LERX; behind, the flat lifting body over the nacelles out
// to the tail booms.
const DECK: [[f64; 5]; 9] = [
    [-5.70, 0.38, 0.66, 0.62, 0.10],
    [-4.60, 0.80, 0.68, 0.60, 0.14],
    [-3.50, 1.10, 0.68, 0.57, 0.20],
    [-2.20, 1.40, 0.68, 0.55, 0.30],
    [-0.80, 1.62, 0.67, 0.52, 0.40],
    [0.40, 1.76, 0.66, 0.50, 0.40],
    [2.00, 1.78, 0.65, 0.50, 0.40],
    [4.50, 1.78, 0.62, 0.48, 0.30],
    [6.30, 1.70, 0.56, 0.46, 0.0],
];
const DECK_Z: (f64, f64) = (-5.70, 6.30);
/// Inboard edge of the deck surfaces, inside the fuselage.
const DECK_IN: f64 = 0.30;

fn deck_point(side: f64, z: f64, s: f64, upper: bool) -> V3 {
    let [_, w, y_root, y_edge, th] = spline(&DECK, z);
    let k = 1.0 - s * s;
    let top = y_edge + (y_root - y_edge) * k;
    let y = if upper { top } else { top - th.max(0.0) * k.powf(0.6) };
    v3(side * mix(DECK_IN, w, s), y, z)
}

// ------------------------------------------------------------------ nacelles
// z, half width, centre height, half height, superellipse exponent: a rounded
// box behind the intakes, easing round at the nozzles.
const NACELLE: [[f64; 5]; 5] = [
    [-2.00, 0.42, -0.10, 0.44, 9.0],
    [0.50, 0.45, -0.10, 0.46, 7.0],
    [3.50, 0.47, -0.06, 0.48, 4.5],
    [5.00, 0.50, 0.00, 0.50, 2.6],
    [5.95, 0.53, 0.05, 0.53, 2.0],
];
const NACELLE_END: f64 = 5.95;

/// Engine axis offset: the nacelles splay out 1.5 degrees toward the nozzles.
fn nacelle_x(z: f64) -> f64 {
    0.86 + (z + 2.0) * 0.027
}

/// The intake mouth is raked: the top lip under the LERX sits 1.25 m ahead
/// of the bottom lip, so the opening faces forward and down.
fn intake_lip(th: f64) -> f64 {
    mix(-0.70, -1.95, (th.sin() + 1.0) * 0.5)
}

fn nacelle_axis(side: f64, z: f64) -> V3 {
    v3(side * nacelle_x(z), spline(&NACELLE, z)[2], z)
}

fn nacelle_point(side: f64, z: f64, th: f64, scale: f64) -> V3 {
    let [_, w, _, h, n] = spline(&NACELLE, z);
    let (c, s) = (th.cos(), th.sin());
    let e = 2.0 / n;
    nacelle_axis(side, z) + v3(side * w * scale * c.signum() * c.abs().powf(e), h * scale * s.signum() * s.abs().powf(e), 0.0)
}

// --------------------------------------------------------------- tail booms
// z, half width, half height.
const BOOM: [[f64; 3]; 6] = [[0.30, 0.02, 0.02], [1.30, 0.17, 0.19], [3.00, 0.20, 0.22], [6.20, 0.20, 0.22], [7.20, 0.14, 0.14], [7.75, 0.02, 0.02]];
const BOOM_X: f64 = 1.80;
const BOOM_Y: f64 = 0.48;

fn boom_point(side: f64, z: f64, th: f64) -> V3 {
    let [_, w, h] = spline(&BOOM, z);
    v3(side * (BOOM_X + w.max(0.0) * th.cos()), BOOM_Y + h.max(0.0) * th.sin(), z)
}

// --------------------------------------------------------------------- wings
const WING_ROOT: f64 = 1.70;
const WING_TIP: f64 = 5.68;
/// Spanwise panel breaks: slat root, flap and aileron ends.
const WING_X: [f64; 6] = [WING_ROOT, 2.38, 3.29, 4.01, 4.88, WING_TIP];
/// Chordwise breaks: slat hinge, spar lines, flap and aileron hinge.
const WING_C: [f64; 8] = [0.0, 0.03, 0.14, 0.35, 0.55, 0.76, 0.88, 1.0];

fn wing_le(x: f64) -> f64 {
    0.45 + (x - WING_ROOT) * (42.0 * DEG).tan()
}

fn wing_te(x: f64) -> f64 {
    4.90 + (x - WING_ROOT) * 0.075
}

/// Wing surface at span station `x` (positive) and chord fraction `c`:
/// 6 percent thick at the root thinning to 4.5 at the tip, 3 degrees anhedral.
fn wing_point(side: f64, x: f64, c: f64, upper: bool) -> V3 {
    let (le, te) = (wing_le(x), wing_te(x));
    let tc = mix(0.06, 0.045, (x - WING_ROOT) / (WING_TIP - WING_ROOT));
    let y = 0.46 - (x - WING_ROOT) * (3.0 * DEG).tan() + if upper { 1.0 } else { -1.0 } * naca(c) * tc * (te - le);
    v3(side * x, y, mix(le, te, c))
}

fn wing_colour(p: V3, n: V3) -> Rgb {
    let x = p.x.abs();
    let c = (p.z - wing_le(x)) / (wing_te(x) - wing_le(x));
    let moving = (c < WING_C[2] && x > WING_X[1]) || c > WING_C[5];
    skin(p, n).scale(if moving { 0.9 } else { 1.0 })
}

// ---------------------------------------------------------------------- fins
const FIN_ROOT_Y: f64 = 0.64;
const FIN_H: f64 = 2.55;
/// Height breaks: top of the root extension, rudder ends, dielectric cap.
const FIN_HS: [f64; 7] = [0.0, 0.42, 1.0, 1.6, 2.1, 2.42, FIN_H];
const FIN_C: [f64; 8] = [0.0, 0.04, 0.15, 0.35, 0.55, 0.72, 0.86, 1.0];

/// Fin surface at height `h` along the canted span and chord fraction `c`,
/// on the outer (`sign` 1) or inner face. The leading edge kinks where the
/// low root extension meets the fin proper.
fn fin_point(side: f64, h: f64, c: f64, sign: f64) -> V3 {
    let cant = 6.0 * DEG;
    let up = v3(side * cant.sin(), cant.cos(), 0.0);
    let out = v3(side * cant.cos(), -cant.sin(), 0.0);
    let le = if h < FIN_HS[1] { mix(3.0, 4.3, h / FIN_HS[1]) } else { mix(4.3, 6.55, (h - FIN_HS[1]) / (FIN_H - FIN_HS[1])) };
    let te = mix(7.15, 7.22, h / FIN_H);
    let t = naca(c) * 0.045 * (te - le).min(3.2);
    v3(side * BOOM_X, FIN_ROOT_Y, 0.0) + up * h + V3::Z * mix(le, te, c) + out * (t * sign)
}

fn fin_colour(p: V3, n: V3) -> Rgb {
    let side = p.x.signum();
    let h = (p.y - FIN_ROOT_Y) / (6.0 * DEG).cos();
    if h > FIN_HS[5] {
        return WHITE;
    }
    let le = fin_point(side, h, 0.0, 0.0).z;
    let te = fin_point(side, h, 1.0, 0.0).z;
    let rudder = h > 0.5 && (p.z - le) / (te - le) > FIN_C[5];
    skin(p, n).scale(if rudder { 0.9 } else { 1.0 })
}

// --------------------------------------------------------------- stabilators
fn stab_point(side: f64, s: f64, c: f64, upper: bool) -> V3 {
    let x = mix(STAB_PIVOT.x, 3.82, s);
    let (le, te) = (mix(5.42, 7.70, s), mix(7.85, 8.65, s));
    let y = STAB_PIVOT.y + if upper { 1.0 } else { -1.0 } * naca(c) * 0.045 * (te - le);
    v3(side * x, y, mix(le, te, c))
}

/// Closing cap over the tip of a thin surface, from its upper and lower edges.
fn tip_cap(m: &mut Mesh, upper: &[V3], lower: &[V3], out: V3, col: Rgb) {
    let mut pts = upper.to_vec();
    pts.extend(lower.iter().rev().skip(1).take(lower.len() - 2));
    face(m, &pts, out, col);
}

pub fn build() -> Fulcrum {
    let mut m = Mesh::new();

    // Fuselage up to the airbrake hinges; the open bay has no fixed skin
    // underneath the moving panels.
    let axis = |p: V3| v3(0.0, spline(&FUSE, p.z)[2], p.z);
    let fuse_z = |u: f64| mix(FUSE_Z.0, FUSE_Z.1, 0.5 * u + 0.5 * u * u);
    m.surface(34, 28, |u, v| fuselage_point(fuse_z(u), v * TAU), |p, n| if p.z < RADOME_Z { RADOME } else { skin(p, n) }, Some(&axis));
    let tail: Vec<V3> = (0..28).map(|k| fuselage_point(FUSE_Z.1, k as f64 * TAU / 28.0)).collect();
    face(&mut m, &tail, V3::Z, SOOT);
    brake_bay(&mut m);
    // Pitot boom on the radome tip, with its vane collar.
    m.capsule(v3(0.0, 0.27, -7.42), v3(0.0, 0.27, -8.65), 0.04, 0.018, METAL, 5);
    m.capsule(v3(0.0, 0.27, -8.12), v3(0.0, 0.27, -8.22), 0.035, 0.035, METAL, 5);

    // Canopy, the IRST ball ahead of it (offset to starboard) and a blade
    // antenna on the spine.
    let mut canopy = Mesh::new();
    canopy.surface(CANOPY_Z.len() - 1, CANOPY_PHI.len() - 1, |u, v| canopy_point(steps(&CANOPY_Z, u), steps(&CANOPY_PHI, v)), canopy_colour, Some(&|p: V3| v3(0.0, 0.9, p.z)));
    let irst = v3(0.17, fuselage_top_at(-4.98, 0.17) - 0.02, -4.98);
    m.ellipsoid(irst, v3(0.13, 0.12, 0.15), Rgb(0.25, 0.33, 0.36), 4, 8);
    m.ellipsoid(irst + v3(0.0, -0.03, 0.10), v3(0.15, 0.08, 0.20), LIGHT, 3, 8);
    // The blade's foot follows the spine down, sunk a little into it.
    let top = |z: f64| fuselage_point(z, PI / 2.0).y - 0.03;
    let blade = [v3(0.0, top(0.0), 0.0), v3(0.0, top(0.36), 0.36), v3(0.0, top(0.36) + 0.30, 0.48), v3(0.0, top(0.0) + 0.30, 0.38)];
    for s in [-1.0, 1.0] {
        let p: Vec<V3> = blade.iter().map(|&p| p + V3::X * (s * 0.012)).collect();
        face(&mut m, &p, v3(s, 0.0, 0.0), METAL);
    }
    for k in 0..4 {
        let (a, b) = (blade[k], blade[(k + 1) % 4]);
        face(&mut m, &[a - V3::X * 0.012, a + V3::X * 0.012, b + V3::X * 0.012, b - V3::X * 0.012], (a + b) * 0.5 - (blade[0] + blade[2]) * 0.5, METAL);
    }

    for side in [-1.0, 1.0] {
        // LERX and the centre body deck.
        let deck_z = |u: f64| mix(DECK_Z.0, DECK_Z.1, u);
        for upper in [true, false] {
            let out = move |p: V3| p - V3::Y * if upper { 1.0 } else { -1.0 };
            m.surface(24, 5, move |u, v| deck_point(side, deck_z(u), v, upper), |p, n| if upper { skin(p, n) } else { BELLY }, Some(&out));
        }
        // Five dimensioned auxiliary louvers, with painted leaves and
        // shallow dark gaps. The legacy FG model locates these immediately
        // behind the canopy, 0.60 m across and about 1.16 m along the LERX.
        for k in 0..5 {
            let z0=CANOPY_Z[9]+0.10+k as f64*0.235;
            let q=|x:f64,z:f64|{
                let s=(x-DECK_IN)/(spline(&DECK,z)[1]-DECK_IN);
                deck_point(side,z,s,true)+V3::Y*0.010
            };
            let (x0,x1)=(0.49,1.08);
            face(&mut m,&[q(x0,z0),q(x1,z0),q(x1,z0+0.215),q(x0,z0+0.215)],V3::Y,DUCT);
            let (a,b,c,d)=(q(x0,z0+0.035)+V3::Y*0.020,q(x1,z0+0.035)+V3::Y*0.020,
                q(x1,z0+0.215),q(x0,z0+0.215));
            face(&mut m,&[a,b,c,d],V3::Y,LIGHT);
            for (p,r) in [(a,d),(b,c)] {
                m.quad(p,r,r-V3::Y*0.012,p-V3::Y*0.012,METAL);
            }
        }
        // Gun muzzle in the port LERX root.
        if side < 0.0 {
            m.ellipsoid(v3(-0.63, 0.69, -3.40), v3(0.08, 0.06, 0.40), METAL, 3, 6);
            m.ellipsoid(v3(-0.63, 0.69, -3.74), v3(0.05, 0.04, 0.08), DUCT, 3, 6);
        }

        // Engine nacelle from the raked intake to the nozzle, its intake
        // lip and the dark duct inside, and the afterburner nozzle.
        let n_axis = move |p: V3| nacelle_axis(side, p.z);
        m.surface(18, 16, move |u, v| {
            let th = v * TAU;
            nacelle_point(side, mix(intake_lip(th), NACELLE_END, u), th, 1.0)
        }, |p, n| if p.z > 4.8 { BARE } else { skin(p, n) }, Some(&n_axis));
        m.surface(1, 16, move |u, v| {
            let th = v * TAU;
            nacelle_point(side, intake_lip(th), th, mix(1.0, 0.80, u))
        }, |_, _| LIGHT, Some(&|p: V3| p + v3(0.0, 0.5, 1.0)));
        m.surface(3, 16, move |u, v| {
            let th = v * TAU;
            nacelle_point(side, intake_lip(th) + 1.1 * u, th, 0.80 * (1.0 - u.powi(3)))
        }, |_, _| DUCT, Some(&move |p: V3| n_axis(p) * 2.0 - p));
        let base = nacelle_axis(side, NACELLE_END);
        let dir = v3(side * 0.027, 0.0, 1.0).norm();
        m.lathe(base, dir, &[(0.0, 0.53), (0.30, 0.52), (1.30, 0.46)], 16, METAL);
        m.lathe(base, dir, &[(1.30, 0.46), (1.31, 0.40), (0.70, 0.34), (0.60, 0.0)], 16, SOOT);

        // Tail boom carrying the fin and the stabilator.
        let b_axis = move |p: V3| v3(side * BOOM_X, BOOM_Y, p.z);
        m.surface(16, 8, move |u, v| boom_point(side, mix(BOOM[0][0], BOOM[5][0], u), v * TAU), skin, Some(&b_axis));

        // Wing.
        for upper in [true, false] {
            let out = move |p: V3| p - V3::Y * if upper { 1.0 } else { -1.0 };
            m.surface(WING_X.len() - 1, WING_C.len() - 1, move |u, v| wing_point(side, steps(&WING_X, u), steps(&WING_C, v), upper), move |p, n| if upper { wing_colour(p, n) } else { BELLY }, Some(&out));
        }
        let edge = |upper: bool| -> Vec<V3> { WING_C.iter().map(|&c| wing_point(side, WING_TIP, c, upper)).collect() };
        tip_cap(&mut m, &edge(true), &edge(false), V3::X * side, LIGHT);
        let p = wing_point(side, 4.95, 0.38, false);
        star(&mut m, p, -V3::Y, 0.36);
        // Pylons, with launch rails under the outer two for the missiles.
        for (k, &x) in PYLONS.iter().enumerate() {
            let p = pylon(x);
            let at = |dy: f64, dz: f64| Frame { o: v3(side * p.x, p.y + dy, p.z + dz), x: V3::X, y: V3::Y, z: V3::Z };
            m.block(&at(0.15, 0.0), v3(0.05, 0.15, 0.62), LIGHT);
            if k > 0 {
                m.block(&at(-0.04, -0.15), v3(0.035, 0.04, 0.80), METAL);
            }
        }

        // Fin, its root extension and the dielectric cap on top.
        for sign in [1.0, -1.0] {
            let out = move |p: V3| p - v3(side * (6.0 * DEG).cos(), -(6.0 * DEG).sin(), 0.0) * sign;
            m.surface(FIN_HS.len() - 1, FIN_C.len() - 1, move |u, v| fin_point(side, steps(&FIN_HS, u), steps(&FIN_C, v), sign), fin_colour, Some(&out));
            let n = v3(side * (6.0 * DEG).cos(), -(6.0 * DEG).sin(), 0.0) * sign;
            star(&mut m, fin_point(side, 1.35, 0.45, sign), n, 0.36);
        }
        let edge = |sign: f64| -> Vec<V3> { FIN_C.iter().map(|&c| fin_point(side, FIN_H, c, sign)).collect() };
        tip_cap(&mut m, &edge(1.0), &edge(-1.0), v3(side * (6.0 * DEG).sin(), (6.0 * DEG).cos(), 0.0), WHITE);
    }

    let stab = |side: f64| {
        let mut st = Mesh::new();
        let cs = [0.0, 0.05, 0.2, 0.45, 0.7, 1.0];
        for upper in [true, false] {
            let out = move |p: V3| p - V3::Y * if upper { 1.0 } else { -1.0 };
            st.surface(4, cs.len() - 1, move |u, v| stab_point(side, u, steps(&cs, v), upper), move |p, n| if upper { skin(p, n) } else { BELLY }, Some(&out));
        }
        let edge = |upper: bool| -> Vec<V3> { cs.iter().map(|&c| stab_point(side, 1.0, c, upper)).collect() };
        tip_cap(&mut st, &edge(true), &edge(false), V3::X * side, LIGHT);
        st
    };
    let mut stabs = [stab(1.0), stab(-1.0)];
    let mut brakes = [airbrake(true), airbrake(false)];
    for mesh in std::iter::once(&mut m).chain(stabs.iter_mut()).chain(brakes.iter_mut()) {
        flag_camo(mesh);
    }
    Fulcrum { body: m, canopy, stabs, gear: landing_gear(), brakes }
}

/// The 9.13S retains the split tail airbrake, surrounding the drag-chute
/// container between the nozzles. Upper 0.75 m^2 at 56 degrees, lower
/// 0.55 m^2 at 60 degrees (DCS MiG-29 flight manual, General Design).
/// The photographed rear view sets the broad sides and U-shaped aft edges;
/// their dimensions are fitted to the published areas, not factory drawings.
const BRAKE_FRONT: f64 = 6.20;
const BRAKE_HALF: f64 = 0.42;
const CHUTE_NOTCH: f64 = 0.21;
const BRAKE_NU: usize = 8;
const BRAKE_NV: usize = 32;
const BRAKE_THICKNESS: f64 = 0.012;
const CHUTE_Y: f64 = 0.44;
const CHUTE_RADIUS: f64 = 0.15;

/// Signed rotation about the leading-edge hinge in the model's y-up axes.
pub fn brake_angle(upper: bool, opening: f64) -> f64 {
    clamp(opening, 0.0, 1.0) * if upper { -56.0 * DEG } else { 60.0 * DEG }
}

pub fn brake_hinge(upper: bool) -> V3 {
    fuselage_point(BRAKE_FRONT, if upper { PI / 2.0 } else { 1.5 * PI })
}

/// Closed panel. A semicircular cutout in plan view leaves the fixed chute
/// tube exposed; the lower panel has the deeper cutout seen in rear photos.
fn brake_point(upper: bool, u: f64, v: f64, lift: f64) -> V3 {
    let side = if upper { 1.0 } else { -1.0 };
    let x = mix(-BRAKE_HALF, BRAKE_HALF, v);
    let notch = (1.0 - (x / CHUTE_NOTCH).powi(2)).max(0.0).sqrt();
    let (aft, depth, camber) = if upper { (7.25, 0.40, 0.11) } else { (7.05, 0.50, 0.10) };
    let z = mix(BRAKE_FRONT, aft - depth * notch, u);
    let hinge = brake_hinge(upper);
    let y = hinge.y - side * u * (0.015 + camber * (x / BRAKE_HALF).powi(2));
    v3(x, y + side * lift, z)
}

/// Fixed sidewalls and the separate drag-chute container. The broad upper
/// and lower skins belong exclusively to the moving meshes.
fn brake_bay(m: &mut Mesh) {
    let hinge_top = brake_hinge(true).y;
    let hinge_bottom = brake_hinge(false).y;
    for side in [-1.0, 1.0] {
        let p = |x: f64, y: f64, z: f64| v3(side * x, y, z);
        let wall = [p(BRAKE_HALF, hinge_bottom, BRAKE_FRONT), p(BRAKE_HALF, hinge_top, BRAKE_FRONT),
            p(BRAKE_HALF, hinge_top - 0.125, 7.25), p(BRAKE_HALF, hinge_bottom + 0.115, 7.05)];
        face(m, &wall, V3::X * side, LIGHT);
        let inner: Vec<V3> = wall.iter().map(|&q| q - V3::X * (side * 0.018)).collect();
        face(m, &inner, -V3::X * side, METAL);
        for i in 0..wall.len() {
            let j = (i + 1) % wall.len();
            face(m, &[wall[i], wall[j], inner[j], inner[i]], (wall[i] + wall[j]) * 0.5 - v3(0.0, CHUTE_Y, 6.70), METAL);
        }
    }
    let tube = v3(0.0, CHUTE_Y, BRAKE_FRONT - 0.04);
    m.lathe(tube, V3::Z, &[(0.0, CHUTE_RADIUS), (1.20, CHUTE_RADIUS), (1.31, CHUTE_RADIUS * 0.94)], 16, LIGHT);
    // Rounded, closed lid aft of the nozzle exits, with a narrow dark seam.
    m.lathe(tube, V3::Z, &[(1.31, CHUTE_RADIUS * 0.95), (1.33, CHUTE_RADIUS * 0.95)], 16, SOOT);
    m.lathe(tube, V3::Z, &[(1.33, CHUTE_RADIUS * 0.95), (1.39, CHUTE_RADIUS * 0.80), (1.43, 0.0)], 16, LIGHT);
}

fn airbrake(upper: bool) -> Mesh {
    let mut m = Mesh::new();
    let side = if upper { 1.0 } else { -1.0 };
    let at = |u: f64, v: f64, lift: f64| brake_point(upper, u, v, lift);
    let outer = |p: V3| p - V3::Y * side;
    let inner = |p: V3| p + V3::Y * side;
    m.surface(BRAKE_NU, BRAKE_NV, |u, v| at(u, v, BRAKE_THICKNESS), |_, _| if upper { LIGHT } else { BELLY }, Some(&outer));
    m.surface(BRAKE_NU, BRAKE_NV, |u, v| at(u, v, -BRAKE_THICKNESS), |_, _| METAL, Some(&inner));
    // Close the thin rim, including the curved cutout around the chute.
    for (u0, v0, u1, v1) in [(0.0, 0.0, 0.0, 1.0), (1.0, 0.0, 1.0, 1.0), (0.0, 0.0, 1.0, 0.0), (0.0, 1.0, 1.0, 1.0)] {
        let segments = if u0 == u1 { BRAKE_NV } else { BRAKE_NU };
        for k in 0..segments {
            let (a, b) = (k as f64 / segments as f64, (k + 1) as f64 / segments as f64);
            let (ua, va, ub, vb) = (mix(u0, u1, a), mix(v0, v1, a), mix(u0, u1, b), mix(v0, v1, b));
            let tangent = at(ub, vb, 0.0) - at(ua, va, 0.0);
            let direction = if u0 == u1 { if u0 == 0.0 { -1.0 } else { 1.0 } } else if v0 == 0.0 { 1.0 } else { -1.0 };
            let out = tangent.cross(V3::Y) * direction;
            face(&mut m, &[at(ua, va, BRAKE_THICKNESS), at(ub, vb, BRAKE_THICKNESS), at(ub, vb, -BRAKE_THICKNESS), at(ua, va, -BRAKE_THICKNESS)], out, METAL);
        }
    }
    // Internal stiffeners visible with the airbrake deployed.
    for v in [0.16, 0.84] {
        let a = at(0.08, v, -0.023);
        let b = at(0.88, v, -0.023);
        m.capsule(a, b, 0.012, 0.012, BARE, 5);
    }
    m
}

/// Landing gear legs fully down: the nose leg with its twin wheels and mud
/// guard well aft, under the intakes; the main legs with single wheels under
/// the wing roots, 3.1 m apart. The tyres meet the ground 1.55 m under the
/// datum, where the sim puts its wheels.
fn landing_gear() -> [Mesh; 3] {
    let strut = Rgb(0.72, 0.74, 0.72);
    let tyre = Rgb(0.07, 0.07, 0.075);
    let hub = Rgb(0.30, 0.47, 0.32);
    let wheel = |m: &mut Mesh, c: V3, r: f64, w: f64| {
        m.lathe(c - V3::X * (w * 0.5), V3::X, &[(0.0, 0.0), (0.0, r * 0.82), (0.02, r), (w - 0.02, r), (w, r * 0.82), (w, 0.0)], 10, tyre);
        m.lathe(c - V3::X * (w * 0.5 + 0.005), V3::X, &[(0.0, 0.0), (0.0, r * 0.55), (w + 0.01, r * 0.55), (w + 0.01, 0.0)], 8, hub);
    };
    let leg = |m: &mut Mesh, a: V3, b: V3, r: f64| {
        let l = (b - a).len();
        m.lathe(a, (b - a).norm(), &[(0.0, 0.0), (0.0, r), (l * 0.55, r), (l * 0.56, r * 0.75), (l, r * 0.75), (l + 0.01, 0.0)], 6, strut);
    };
    let plate = |m: &mut Mesh, o: V3, half: V3, col: Rgb| m.block(&Frame { o, x: V3::X, y: V3::Y, z: V3::Z }, half, col);

    let mut nose = Mesh::new();
    let (p, _, _) = GEAR_PIVOTS[0];
    // Ten-sided tyres rest on a flat, r cos 18 degrees under the axle.
    let axle = v3(0.0, -1.55 + 0.285 * (PI / 10.0).cos(), -1.50);
    leg(&mut nose, p, axle, 0.06);
    nose.lathe(axle - V3::X * 0.22, V3::X, &[(0.0, 0.0), (0.0, 0.035), (0.44, 0.035), (0.44, 0.0)], 5, strut);
    for s in [-1.0, 1.0] {
        wheel(&mut nose, axle + V3::X * (s * 0.16), 0.285, 0.14);
        plate(&mut nose, p + v3(s * 0.20, -0.32, 0.05), v3(0.008, 0.28, 0.42), BELLY);
    }
    // The mud guard that keeps the wheels from throwing debris into the intakes.
    plate(&mut nose, axle + v3(0.0, 0.34, 0.26), v3(0.30, 0.015, 0.20), strut.scale(0.9));

    let main = |side: f64| {
        let mut m = Mesh::new();
        let (p, _, _) = GEAR_PIVOTS[if side < 0.0 { 1 } else { 2 }];
        let axle = v3(side * 1.55, -1.55 + 0.42 * (PI / 10.0).cos(), 2.16);
        let foot = axle - V3::X * (side * 0.20);
        leg(&mut m, p, foot, 0.08);
        m.lathe(foot, V3::X * side, &[(0.0, 0.0), (0.0, 0.04), (0.2, 0.04), (0.2, 0.0)], 5, strut);
        // Drag brace from the leg forward into the well.
        let (brace, mid) = (p + v3(0.0, -0.05, -0.75), p.lerp(foot, 0.55));
        let l = (mid - brace).len();
        m.lathe(brace, (mid - brace).norm(), &[(0.0, 0.0), (0.0, 0.035), (l, 0.035), (l, 0.0)], 5, strut.scale(0.85));
        wheel(&mut m, axle, 0.42, 0.26);
        plate(&mut m, p.lerp(foot, 0.4) + V3::X * (side * 0.11), v3(0.008, 0.45, 0.35), BELLY);
        m
    };
    [nose, main(-1.0), main(1.0)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::STRIDE;

    fn extent(meshes: &[&Mesh]) -> (V3, V3) {
        let (mut lo, mut hi) = (v3(f64::INFINITY, f64::INFINITY, f64::INFINITY), v3(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY));
        for mesh in meshes {
            for v in mesh.v.chunks(STRIDE) {
                let p = v3(v[0] as f64, v[1] as f64, v[2] as f64);
                lo = v3(lo.x.min(p.x), lo.y.min(p.y), lo.z.min(p.z));
                hi = v3(hi.x.max(p.x), hi.y.max(p.y), hi.z.max(p.z));
            }
        }
        (lo, hi)
    }

    #[test]
    fn auxiliary_intakes_fit_the_lerx_and_match_the_reference_envelope() {
        assert!(((1.08-0.49)-0.60_f64).abs()<0.02);
        let front=CANOPY_Z[9]+0.10;
        let aft=front+4.0*0.235+0.215;
        assert!((aft-front-1.16).abs()<0.02);
        for z in [front,aft] {
            assert!(spline(&DECK,z)[1]>1.08);
        }
    }

    #[test]
    fn fulcrum_matches_its_three_view() {
        let f = build();
        let (lo, hi) = extent(&[&f.body, &f.stabs[0], &f.stabs[1]]);
        let (span, length) = (hi.x - lo.x, hi.z - lo.z);
        assert!((span - 11.36).abs() < 0.1, "span {span:.2} m");
        assert!((length - 17.32).abs() < 0.1, "length {length:.2} m");
        // Fin tip 4.73 m above the ground with the sim's gear down (1.55 m).
        assert!((hi.y + 1.55 - 4.73).abs() < 0.1, "height {:.2} m", hi.y + 1.55);
    }

    #[test]
    fn wheels_stand_on_the_ground() {
        let f = build();
        for (i, g) in f.gear.iter().enumerate() {
            let (lo, _) = extent(&[g]);
            assert!((lo.y + 1.55).abs() < 0.02, "gear {i} bottom at {:.3} m", lo.y);
        }
    }

    #[test]
    fn geometry_is_finite_with_unit_normals() {
        let f = build();
        for mesh in [&f.body, &f.canopy, &f.stabs[0], &f.stabs[1], &f.gear[0], &f.gear[1], &f.gear[2], &f.brakes[0], &f.brakes[1]] {
            assert!(mesh.count() > 0 && mesh.count() % 3 == 0);
            for v in mesh.v.chunks(STRIDE) {
                let p = v3(v[0] as f64, v[1] as f64, v[2] as f64);
                let n = v3(v[3] as f64, v[4] as f64, v[5] as f64);
                assert!(p.is_finite() && n.is_finite(), "bad vertex at {p:?}");
                assert!((n.len() - 1.0).abs() < 0.02 || n.len() < 1e-6, "normal {:.3} at {p:?}", n.len());
            }
        }
    }

    #[test]
    fn airbrake_panels_match_published_areas_and_surround_the_chute() {
        let f = build();
        for (i, target) in [0.75, 0.55].into_iter().enumerate() {
            // Only the exterior skin, excluding the inner skin, rim and ribs.
            let skin = &f.brakes[i].v[..BRAKE_NU * BRAKE_NV * 6 * STRIDE];
            let mut area = 0.0;
            for tri in skin.chunks_exact(3 * STRIDE) {
                let at = |k| v3(tri[k * STRIDE] as f64, tri[k * STRIDE + 1] as f64, tri[k * STRIDE + 2] as f64);
                area += (at(1) - at(0)).cross(at(2) - at(0)).len() * 0.5;
            }
            assert!((area - target).abs() < target * 0.05, "panel {i} area {area:.3} m^2, expected {target}");
            let (lo, hi) = extent(&[&f.brakes[i]]);
            assert!((hi.x - lo.x - 0.84).abs() < 0.002, "panel {i} width");
            assert!(lo.z > 6.1 && hi.z > 7.0, "panel {i} belongs at the nozzle exits");
            let upper = i == 0;
            assert!(brake_point(upper, 1.0, 0.5, 0.0).z + 0.35 < brake_point(upper, 1.0, 0.0, 0.0).z,
                "panel {i} needs an aft cutout around the chute");
        }
    }

    #[test]
    fn airbrakes_open_clear_of_the_fixed_chute_and_keep_their_hinges() {
        for (upper, degrees) in [(true, -56.0), (false, 60.0)] {
            let hinge = brake_hinge(upper);
            assert!((brake_angle(upper, 1.0) / DEG - degrees).abs() < 1e-9);
            for opening in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let q = Quat::axis_angle(V3::X, brake_angle(upper, opening));
                for j in 0..=32 {
                    let v = j as f64 / 32.0;
                    let leading = brake_point(upper, 0.0, v, 0.0);
                    assert!((hinge + q.rotate(leading - hinge) - leading).len() < 1e-9, "hinge moves");
                    for k in 0..=16 {
                        // The inner face is the surface nearest the container.
                        let p = brake_point(upper, k as f64 / 16.0, v, -BRAKE_THICKNESS);
                        let moved = hinge + q.rotate(p - hinge);
                        let distance = (moved.x * moved.x + (moved.y - CHUTE_Y).powi(2)).sqrt();
                        assert!(distance > CHUTE_RADIUS + 0.005, "panel intersects chute at {moved:?}");
                    }
                }
                if opening == 1.0 {
                    let p = brake_point(upper, 1.0, 0.0, 0.0);
                    let moved = hinge + q.rotate(p - hinge);
                    let travel = (moved.y - p.y) * if upper { 1.0 } else { -1.0 };
                    assert!(travel > 0.65, "panel opens in the wrong direction: {travel}");
                }
            }
        }
    }

    #[test]
    fn canopy_covers_the_fuselage_and_spine() {
        // Centres of the flat canopy cells as drawn, against the true skin.
        for i in 0..CANOPY_Z.len() - 2 {
            for j in 0..CANOPY_PHI.len() - 1 {
                let (z0, z1, f0, f1) = (CANOPY_Z[i], CANOPY_Z[i + 1], CANOPY_PHI[j], CANOPY_PHI[j + 1]);
                let p = (canopy_point(z0, f0) + canopy_point(z0, f1) + canopy_point(z1, f0) + canopy_point(z1, f1)) * 0.25;
                let z = p.z;
                // The skin under the glass at the same x, spine included.
                let mut th = PI / 2.0;
                for _ in 0..40 {
                    let q = fuselage_point(z, th);
                    th += (q.x - p.x) * 0.5;
                }
                let skin = fuselage_point(z, th);
                assert!(p.y > skin.y - 0.025, "glass {p:?} under skin {skin:?}");
            }
        }
    }

    #[test]
    fn intakes_clear_the_fuselage() {
        // The upper inner corner of each intake mouth stays below the belly,
        // so the fuselage never shows inside the opening.
        for side in [-1.0, 1.0] {
            let th = 3.0 * PI / 4.0;
            let corner = nacelle_point(side, intake_lip(th), th, 0.8);
            let [_, w, yc, _, hl, _, nl] = spline(&FUSE, corner.z);
            let r = (corner.x.abs() / w).min(1.0);
            let belly = yc - hl * (1.0 - r.powf(nl)).max(0.0).powf(1.0 / nl);
            assert!(corner.y < belly, "corner {corner:?} belly {belly:.2}");
        }
    }
}
