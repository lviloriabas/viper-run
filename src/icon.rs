//! Procedural pixel-art icon: a low-poly F-16 seen from above in 3/4 view,
//! casting its shadow on a perspective grid, in the main menu's palette.
//! Shared by the game (window icon) and build.rs (Windows .ico resource).

/// Pixel art is drawn on a grid of at most this many cells and upscaled
/// with nearest neighbour, so large icons keep the chunky retro look.
const MAX_ART: usize = 64;

/// Stable taskbar identity, including after moving or renaming the portable EXE.
#[cfg(windows)]
pub fn set_app_id() {
    #[link(name = "shell32")]
    unsafe extern "system" { fn SetCurrentProcessExplicitAppUserModelID(id: *const u16) -> i32; }
    let id: Vec<u16> = "lviloriabas.ViperRun\0".encode_utf16().collect();
    unsafe { SetCurrentProcessExplicitAppUserModelID(id.as_ptr()); }
}

/// Set both window and class icons from the compiled multi-size resource.
/// Explorer can consult the class icon when grouping a fullscreen window.
#[cfg(windows)]
pub fn install_windows_icon() {
    use std::ffi::c_void;
    #[link(name="user32")]
    unsafe extern "system" {
        fn EnumWindows(callback: unsafe extern "system" fn(isize,isize)->i32, arg:isize)->i32;
        fn GetWindowThreadProcessId(hwnd:*mut c_void,pid:*mut u32)->u32;
        fn LoadImageW(module:*mut c_void,name:*const u16,kind:u32,w:i32,h:i32,flags:u32)->isize;
        fn SendMessageW(hwnd:isize,msg:u32,wp:usize,lp:isize)->isize;
        fn SetClassLongPtrW(hwnd:isize,index:i32,value:isize)->usize;
    }
    #[link(name="kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(name:*const u16)->*mut c_void;
        fn GetCurrentProcessId()->u32;
    }
    unsafe extern "system" fn visit(hwnd:isize,arg:isize)->i32 {
        unsafe {
            let mut pid=0;GetWindowThreadProcessId(hwnd as *mut c_void,&mut pid);
            if pid!=arg as u32{return 1;}
            let module=GetModuleHandleW(std::ptr::null());
            for (size,slot,class) in [(32,1,-14),(16,0,-34)] {
                let icon=LoadImageW(module,1usize as *const u16,1,size,size,0x8000);
                if icon!=0 { SendMessageW(hwnd,0x0080,slot,icon);SetClassLongPtrW(hwnd,class,icon); }
            }
        }
        1
    }
    unsafe {EnumWindows(visit,GetCurrentProcessId() as isize);}
}

// Main menu colours (see ui.rs).
const INK: u32 = 0x050807;
const PANEL: u32 = 0x0a110e;
const LINE: u32 = 0x31473a;
const LINE_HOT: u32 = 0x6e9b6d;
const ACID: u32 = 0xd8ff86;
const AMBER: u32 = 0xffc05b;
/// Flat-shading ramp from shadowed faces to sunlit ones, ending in CREAM.
const RAMP: [u32; 6] = [0x1d2b23, 0x31473a, 0x55705a, 0x7b957e, 0xb7c4a6, 0xece8cd];

type V3 = [f32; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(a: V3) -> V3 {
    let l = dot(a, a).sqrt();
    [a[0] / l, a[1] / l, a[2] / l]
}

#[derive(Clone, Copy, PartialEq)]
enum Mat {
    Skin,
    Canopy,
    Nozzle,
    Flame,
    Core,
}

struct Tri {
    a: V3,
    e1: V3,
    e2: V3,
    n: V3,
    mat: Mat,
}

#[derive(Default)]
struct Mesh(Vec<Tri>);

impl Mesh {
    fn tri(&mut self, a: V3, b: V3, c: V3, mat: Mat) {
        let (e1, e2) = (sub(b, a), sub(c, a));
        let n = cross(e1, e2);
        if dot(n, n) > 1e-8 {
            self.0.push(Tri { a, e1, e2, n: norm(n), mat });
        }
    }
    /// Convex polygon as a fan, plus its mirror across x = 0.
    fn poly(&mut self, pts: &[V3], mat: Mat) {
        for m in [1.0, -1.0] {
            let p: Vec<V3> = pts.iter().map(|v| [v[0] * m, v[1], v[2]]).collect();
            for i in 1..p.len() - 1 {
                self.tri(p[0], p[i], p[i + 1], mat);
            }
        }
    }
    /// Skin between consecutive cross-sections (each a closed ring).
    fn loft(&mut self, rings: &[Vec<V3>], mat: Mat) {
        for w in rings.windows(2) {
            let k = w[0].len();
            for i in 0..k {
                let j = (i + 1) % k;
                self.tri(w[0][i], w[0][j], w[1][j], mat);
                self.tri(w[0][i], w[1][j], w[1][i], mat);
            }
        }
    }
    /// Nearest hit along a ray (Möller–Trumbore).
    fn hit(&self, o: V3, d: V3, solid_only: bool) -> Option<&Tri> {
        let mut best: Option<(f32, &Tri)> = None;
        for t in &self.0 {
            if solid_only && matches!(t.mat, Mat::Flame | Mat::Core) {
                continue;
            }
            let p = cross(d, t.e2);
            let det = dot(t.e1, p);
            if det.abs() < 1e-9 {
                continue;
            }
            let s = sub(o, t.a);
            let u = dot(s, p) / det;
            if !(0.0..=1.0).contains(&u) {
                continue;
            }
            let q = cross(s, t.e1);
            let v = dot(d, q) / det;
            if v < 0.0 || u + v > 1.0 {
                continue;
            }
            let dist = dot(t.e2, q) / det;
            if dist > 1e-4 && best.is_none_or(|(b, _)| dist < b) {
                best = Some((dist, t));
            }
        }
        best.map(|(_, t)| t)
    }
}

/// Low-poly F-16 in metres: x towards the right wing, y forward, z up.
fn f16() -> Mesh {
    let mut m = Mesh::default();
    // Fuselage stations: (y, half width, top, bottom).
    let stations = [
        (7.8, 0.02, 0.02, 0.02),
        (6.8, 0.35, 0.35, 0.3),
        (5.5, 0.55, 0.55, 0.55),
        (4.0, 0.68, 0.62, 0.8),
        (2.0, 0.85, 0.6, 0.85),
        (0.0, 0.92, 0.58, 0.75),
        (-3.0, 0.92, 0.55, 0.7),
        (-5.5, 0.75, 0.5, 0.6),
        (-7.0, 0.55, 0.45, 0.45),
        (-7.6, 0.45, 0.4, 0.4),
    ];
    let ring = |&(y, w, t, b): &(f32, f32, f32, f32)| {
        vec![
            [0.0, y, t],
            [0.75 * w, y, 0.8 * t],
            [w, y, 0.0],
            [0.75 * w, y, -0.8 * b],
            [0.0, y, -b],
            [-0.75 * w, y, -0.8 * b],
            [-w, y, 0.0],
            [-0.75 * w, y, 0.8 * t],
        ]
    };
    let rings: Vec<Vec<V3>> = stations.iter().map(ring).collect();
    m.loft(&rings, Mat::Skin);
    let tail = rings.last().unwrap();
    for i in 1..tail.len() - 1 {
        m.tri(tail[0], tail[i + 1], tail[i], Mat::Nozzle);
    }

    // Bubble canopy sitting on the spine: (y, half width, height, base z).
    let canopy: Vec<Vec<V3>> = [(6.3, 0.03, 0.0, 0.42), (5.6, 0.3, 0.32, 0.5), (4.6, 0.4, 0.5, 0.6), (3.5, 0.36, 0.42, 0.62), (2.6, 0.12, 0.08, 0.6)]
        .iter()
        .map(|&(y, w, h, z)| vec![[0.0, y, z + h], [0.7 * w, y, z + 0.7 * h], [w, y, z - 0.05], [-w, y, z - 0.05], [-0.7 * w, y, z + 0.7 * h]])
        .collect();
    m.loft(&canopy, Mat::Canopy);

    // Wings with leading-edge root extensions, anhedral stabilators, fin.
    for z in [0.04, -0.04] {
        m.poly(&[[1.4, 0.4, z], [4.6, -2.3, z], [4.6, -3.3, z], [1.1, -4.1, z]], Mat::Skin);
        m.poly(&[[0.6, 4.8, z], [0.9, 2.2, z], [1.5, 0.4, z], [0.8, 0.4, z]], Mat::Skin);
        m.poly(&[[0.8, -4.8, z], [3.0, -6.3, z - 0.35], [3.0, -7.0, z - 0.35], [0.7, -7.1, z]], Mat::Skin);
    }
    for x in [0.07, -0.07] {
        let fin = [[x, -3.2, 0.5], [x, -5.6, 3.3], [x, -6.6, 3.3], [x, -7.2, 0.45]];
        m.tri(fin[0], fin[1], fin[2], Mat::Skin);
        m.tri(fin[0], fin[2], fin[3], Mat::Skin);
    }
    // Wingtip missiles as square prisms with a pointed nose.
    for side in [1.0, -1.0] {
        let x = 4.7 * side;
        let r = 0.16;
        let sq = |y: f32| vec![[x - r, y, r], [x + r, y, r], [x + r, y, -r], [x - r, y, -r]];
        m.loft(&[sq(-3.9), sq(-0.7)], Mat::Skin);
        let s = sq(-0.7);
        for i in 0..4 {
            m.tri(s[i], s[(i + 1) % 4], [x, 0.1, 0.0], Mat::Skin);
        }
    }
    // Afterburner plume: two nested cones.
    for (len, rad, mat) in [(3.2, 0.42, Mat::Flame), (2.0, 0.26, Mat::Core)] {
        let base: Vec<V3> = (0..6)
            .map(|i| {
                let a = i as f32 * std::f32::consts::TAU / 6.0;
                [rad * a.cos(), -7.65, rad * a.sin()]
            })
            .collect();
        for i in 0..6 {
            m.tri(base[i], base[(i + 1) % 6], [0.0, -7.65 - len, 0.0], mat);
        }
    }
    m
}

struct Camera {
    pos: V3,
    fwd: V3,
    right: V3,
    up: V3,
    tan: f32,
}

impl Camera {
    /// Orbit camera: azimuth from behind the tail, elevation above the
    /// horizon. A narrow lens gives the near-isometric, mildly 3D look.
    fn orbit(target: V3, azimuth: f32, elevation: f32, dist: f32, fov: f32) -> Self {
        let (sa, ca) = azimuth.to_radians().sin_cos();
        let (se, ce) = elevation.to_radians().sin_cos();
        let pos = [target[0] + dist * ce * sa, target[1] - dist * ce * ca, target[2] + dist * se];
        let fwd = norm(sub(target, pos));
        let right = norm(cross(fwd, [0.0, 0.0, 1.0]));
        let up = cross(right, fwd);
        Camera { pos, fwd, right, up, tan: (fov.to_radians() * 0.5).tan() }
    }
    /// Ray through normalised screen coords (-1..1, y down).
    fn ray(&self, sx: f32, sy: f32) -> V3 {
        let (a, b) = (sx * self.tan, -sy * self.tan);
        norm([
            self.fwd[0] + a * self.right[0] + b * self.up[0],
            self.fwd[1] + a * self.right[1] + b * self.up[1],
            self.fwd[2] + a * self.right[2] + b * self.up[2],
        ])
    }
    fn ground(&self, sx: f32, sy: f32) -> [f32; 2] {
        let d = self.ray(sx, sy);
        let t = (GROUND_Z - self.pos[2]) / d[2];
        [self.pos[0] + d[0] * t, self.pos[1] + d[1] * t]
    }
}

const GROUND_Z: f32 = -3.2;
const GRID_STEP: f32 = 3.2;

/// RGBA8 icon of `size` x `size` pixels, row-major, top row first.
pub fn render(size: usize) -> Vec<u8> {
    let up = size.div_ceil(MAX_ART);
    let n = size / up;
    let art = draw(n);
    let mut out = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let c = art[(y / up).min(n - 1) * n + (x / up).min(n - 1)];
            let i = (y * size + x) * 4;
            if let Some(c) = c {
                out[i..i + 4].copy_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]);
            }
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq)]
enum Px {
    Plane(u32),
    Shadow,
    Ground,
}

fn shade(t: &Tri, ray: V3) -> u32 {
    let n = if dot(t.n, ray) > 0.0 { [-t.n[0], -t.n[1], -t.n[2]] } else { t.n };
    match t.mat {
        Mat::Canopy if dot(n, norm([-0.3, 0.5, 0.8])) > 0.85 => ACID,
        Mat::Canopy | Mat::Flame => AMBER,
        Mat::Core => ACID,
        Mat::Nozzle => RAMP[1],
        Mat::Skin => {
            let l = dot(n, norm([-0.45, 0.35, 0.82])).max(0.0);
            RAMP[((0.25 + 0.75 * l) * (RAMP.len() - 1) as f32).round() as usize]
        }
    }
}

fn draw(n: usize) -> Vec<Option<u32>> {
    let nf = n as f32;
    let mesh = f16();
    // Small icons zoom in and drop the grid so the jet stays readable.
    let cam = Camera::orbit([0.0, -0.8, -0.8], 52.0, 50.0, 70.0, if n < 24 { 13.5 } else { 14.5 });
    let ss = if n <= 32 { 4 } else { 3 };

    let mut px = vec![Px::Ground; n * n];
    for y in 0..n {
        for x in 0..n {
            let mut votes: Vec<(Px, u32)> = Vec::new();
            for j in 0..ss {
                for i in 0..ss {
                    let sx = (x as f32 + (i as f32 + 0.5) / ss as f32) / nf * 2.0 - 1.0;
                    let sy = (y as f32 + (j as f32 + 0.5) / ss as f32) / nf * 2.0 - 1.0;
                    let d = cam.ray(sx, sy);
                    let p = match mesh.hit(cam.pos, d, false) {
                        Some(t) => Px::Plane(shade(t, d)),
                        None => {
                            // Sun straight overhead: a crisp planform shadow.
                            let g = cam.ground(sx, sy);
                            if mesh.hit([g[0], g[1], GROUND_Z], [0.0, 0.0, 1.0], true).is_some() {
                                Px::Shadow
                            } else {
                                Px::Ground
                            }
                        }
                    };
                    match votes.iter_mut().find(|v| v.0 == p) {
                        Some(v) => v.1 += 1,
                        None => votes.push((p, 1)),
                    }
                }
            }
            // Plane pixels need half coverage; then the most common shade wins.
            let on_plane = |v: &&(Px, u32)| matches!(v.0, Px::Plane(_));
            let plane: u32 = votes.iter().filter(on_plane).map(|v| v.1).sum();
            let pick = if plane * 2 >= (ss * ss) as u32 {
                votes.iter().filter(on_plane).max_by_key(|v| v.1)
            } else {
                votes.iter().filter(|v| !on_plane(v)).max_by_key(|v| v.1)
            };
            px[y * n + x] = pick.map_or(Px::Ground, |v| v.0);
        }
    }

    let is_plane = |x: i32, y: i32| {
        x >= 0 && y >= 0 && x < n as i32 && y < n as i32 && matches!(px[y as usize * n + x as usize], Px::Plane(_))
    };
    let edge = |v: i32| v.min(n as i32 - 1 - v);
    let border = if n >= 24 { 2 } else { 1 };
    let bracket = n as i32 / 5;
    let mut out = vec![None; n * n];
    for y in 0..n {
        for x in 0..n {
            let (xi, yi) = (x as i32, y as i32);
            let (ex, ey) = (edge(xi), edge(yi));
            // Chamfered corners, then a menu-style frame: thin line with
            // bright corner brackets and a dark gap inside it.
            if ex + ey < border {
                continue;
            }
            let c = if ex == 0 || ey == 0 {
                if n >= 24 && ex < bracket && ey < bracket { ACID } else { LINE_HOT }
            } else if border == 2 && (ex == 1 || ey == 1) {
                INK
            } else if let Px::Plane(c) = px[y * n + x] {
                c
            } else if px[y * n + x] == Px::Shadow || (-1..=1).any(|dy| (-1..=1).any(|dx| is_plane(xi + dx, yi + dy))) {
                INK
            } else {
                if n < 24 { PANEL } else { ground(n, &cam, x, y) }
            };
            out[y * n + x] = Some(c);
        }
    }
    out
}

/// Panel colour with perspective grid lines that fade with distance.
fn ground(n: usize, cam: &Camera, x: usize, y: usize) -> u32 {
    let c = |v: usize| (v as f32 + 0.5) / n as f32 * 2.0 - 1.0;
    let corners = [cam.ground(c(x), c(y)), cam.ground(c(x) + 2.0 / n as f32, c(y)), cam.ground(c(x), c(y) + 2.0 / n as f32)];
    // A line passes here when the next pixel right or down is in another cell.
    let crosses = (0..2).any(|k| {
        let cell = |p: &[f32; 2]| (p[k] / GRID_STEP).floor();
        corners.iter().any(|p| cell(p) != cell(&corners[0]))
    });
    let dist = (corners[0][0].powi(2) + corners[0][1].powi(2)).sqrt();
    match (crosses, dist) {
        (true, d) if d < 10.0 => LINE_HOT,
        (true, d) if d < 17.0 => LINE,
        _ => PANEL,
    }
}
