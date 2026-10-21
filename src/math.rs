//! Small vector, quaternion and matrix kit. Simulation runs in f64, GPU data in f32.
use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

pub const PI: f64 = std::f64::consts::PI;
pub const TAU: f64 = std::f64::consts::TAU;
pub const DEG: f64 = PI / 180.0;
pub const RAD: f64 = 180.0 / PI;

#[inline]
pub fn clamp(v: f64, a: f64, b: f64) -> f64 {
    if v < a {
        a
    } else if v > b {
        b
    } else {
        v
    }
}
#[inline]
pub fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
#[inline]
pub fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    let t = clamp((x - a) / (b - a), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
#[inline]
pub fn wrap(v: f64, m: f64) -> f64 {
    ((v % m) + m) % m
}
#[inline]
pub fn wrap180(d: f64) -> f64 {
    wrap(d + 180.0, 360.0) - 180.0
}
#[inline]
pub fn approach(v: f64, target: f64, delta: f64) -> f64 {
    if v < target {
        (v + delta).min(target)
    } else {
        (v - delta).max(target)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[inline]
pub const fn v3(x: f64, y: f64, z: f64) -> V3 {
    V3 { x, y, z }
}

impl V3 {
    pub const ZERO: V3 = v3(0.0, 0.0, 0.0);
    pub const X: V3 = v3(1.0, 0.0, 0.0);
    pub const Y: V3 = v3(0.0, 1.0, 0.0);
    pub const Z: V3 = v3(0.0, 0.0, 1.0);
    #[inline]
    pub fn dot(self, b: V3) -> f64 {
        self.x * b.x + self.y * b.y + self.z * b.z
    }
    #[inline]
    pub fn cross(self, b: V3) -> V3 {
        v3(
            self.y * b.z - self.z * b.y,
            self.z * b.x - self.x * b.z,
            self.x * b.y - self.y * b.x,
        )
    }
    #[inline]
    pub fn len(self) -> f64 {
        self.dot(self).sqrt()
    }
    #[inline]
    pub fn norm(self) -> V3 {
        let l = self.len();
        if l > 1e-9 {
            self * (1.0 / l)
        } else {
            V3::ZERO
        }
    }
    #[inline]
    pub fn lerp(self, b: V3, t: f64) -> V3 {
        self + (b - self) * t
    }
    #[inline]
    pub fn f32(self) -> [f32; 3] {
        [self.x as f32, self.y as f32, self.z as f32]
    }
    #[cfg(test)]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

impl Add for V3 {
    type Output = V3;
    #[inline]
    fn add(self, b: V3) -> V3 {
        v3(self.x + b.x, self.y + b.y, self.z + b.z)
    }
}
impl Sub for V3 {
    type Output = V3;
    #[inline]
    fn sub(self, b: V3) -> V3 {
        v3(self.x - b.x, self.y - b.y, self.z - b.z)
    }
}
impl Mul<f64> for V3 {
    type Output = V3;
    #[inline]
    fn mul(self, s: f64) -> V3 {
        v3(self.x * s, self.y * s, self.z * s)
    }
}
impl Div<f64> for V3 {
    type Output = V3;
    #[inline]
    fn div(self, s: f64) -> V3 {
        v3(self.x / s, self.y / s, self.z / s)
    }
}
impl Neg for V3 {
    type Output = V3;
    #[inline]
    fn neg(self) -> V3 {
        v3(-self.x, -self.y, -self.z)
    }
}
impl AddAssign for V3 {
    #[inline]
    fn add_assign(&mut self, b: V3) {
        *self = *self + b;
    }
}
impl SubAssign for V3 {
    #[inline]
    fn sub_assign(&mut self, b: V3) {
        *self = *self - b;
    }
}
impl MulAssign<f64> for V3 {
    #[inline]
    fn mul_assign(&mut self, s: f64) {
        *self = *self * s;
    }
}

/// Rotation quaternion, (x, y, z) vector part and w scalar part.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

/// Body axes of an orientation. The body frame is right (+x), up (+y), aft (+z).
#[derive(Clone, Copy, Debug)]
pub struct Axes {
    pub right: V3,
    pub up: V3,
    pub back: V3,
    pub forward: V3,
}

impl Quat {
    pub fn mul(self, b: Quat) -> Quat {
        let a = self;
        Quat {
            x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
            y: a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
            z: a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
            w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
        }
    }
    pub fn norm(self) -> Quat {
        let mut l = (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt();
        if l == 0.0 {
            l = 1.0;
        }
        Quat { x: self.x / l, y: self.y / l, z: self.z / l, w: self.w / l }
    }
    #[cfg(test)]
    pub fn len(self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt()
    }
    pub fn axis_angle(axis: V3, angle: f64) -> Quat {
        let a = axis.norm();
        let s = (angle * 0.5).sin();
        Quat { x: a.x * s, y: a.y * s, z: a.z * s, w: (angle * 0.5).cos() }
    }
    pub fn conj(self) -> Quat {
        Quat { x: -self.x, y: -self.y, z: -self.z, w: self.w }
    }
    pub fn rotate(self, v: V3) -> V3 {
        let qv = v3(self.x, self.y, self.z);
        let t = qv.cross(v) * 2.0;
        v + t * self.w + qv.cross(t)
    }
    pub fn from_basis(right: V3, up: V3, back: V3) -> Quat {
        let (m00, m10, m20) = (right.x, right.y, right.z);
        let (m01, m11, m21) = (up.x, up.y, up.z);
        let (m02, m12, m22) = (back.x, back.y, back.z);
        let trace = m00 + m11 + m22;
        let (x, y, z, w);
        if trace > 0.0 {
            let s = (trace + 1.0).sqrt() * 2.0;
            w = 0.25 * s;
            x = (m21 - m12) / s;
            y = (m02 - m20) / s;
            z = (m10 - m01) / s;
        } else if m00 > m11 && m00 > m22 {
            let s = (1.0 + m00 - m11 - m22).sqrt() * 2.0;
            w = (m21 - m12) / s;
            x = 0.25 * s;
            y = (m01 + m10) / s;
            z = (m02 + m20) / s;
        } else if m11 > m22 {
            let s = (1.0 + m11 - m00 - m22).sqrt() * 2.0;
            w = (m02 - m20) / s;
            x = (m01 + m10) / s;
            y = 0.25 * s;
            z = (m12 + m21) / s;
        } else {
            let s = (1.0 + m22 - m00 - m11).sqrt() * 2.0;
            w = (m10 - m01) / s;
            x = (m02 + m20) / s;
            y = (m12 + m21) / s;
            z = 0.25 * s;
        }
        Quat { x, y, z, w }.norm()
    }
    pub fn from_forward_up(forward: V3, up_hint: V3) -> Quat {
        let f = forward.norm();
        let mut r = f.cross(up_hint).norm();
        if r.len() < 1e-4 {
            r = f.cross(V3::Z).norm();
        }
        let u = r.cross(f).norm();
        Quat::from_basis(r, u, -f)
    }
    pub fn axes(self) -> Axes {
        let right = self.rotate(V3::X);
        let up = self.rotate(V3::Y);
        let back = self.rotate(V3::Z);
        Axes { right, up, back, forward: -back }
    }
    /// Integrates a body-frame angular velocity.
    pub fn integrate(self, omega: V3, dt: f64) -> Quat {
        let w = omega.len();
        if w < 1e-8 {
            return self;
        }
        self.mul(Quat::axis_angle(omega, w * dt)).norm()
    }
}

/// Column-major 4x4 matrix.
pub type M4 = [f32; 16];

pub fn perspective(fovy: f64, aspect: f64, near: f64, far: f64) -> M4 {
    let f = 1.0 / (fovy / 2.0).tan();
    let nf = 1.0 / (near - far);
    [
        (f / aspect) as f32, 0.0, 0.0, 0.0,
        0.0, f as f32, 0.0, 0.0,
        0.0, 0.0, ((far + near) * nf) as f32, -1.0,
        0.0, 0.0, (2.0 * far * near * nf) as f32, 0.0,
    ]
}

pub fn mat_mul(a: &M4, b: &M4) -> M4 {
    let mut out = [0f32; 16];
    for c in 0..4 {
        for r in 0..4 {
            out[c * 4 + r] = a[r] * b[c * 4]
                + a[4 + r] * b[c * 4 + 1]
                + a[8 + r] * b[c * 4 + 2]
                + a[12 + r] * b[c * 4 + 3];
        }
    }
    out
}

/// Rotation-only view matrix. Geometry is translated by the eye before it is
/// sent through this, which keeps f32 precision high far from the origin.
pub fn view_rot(right: V3, up: V3, forward: V3) -> M4 {
    let z = -forward;
    [
        right.x as f32, up.x as f32, z.x as f32, 0.0,
        right.y as f32, up.y as f32, z.y as f32, 0.0,
        right.z as f32, up.z as f32, z.z as f32, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]
}

pub fn basis(right: V3, up: V3, back: V3, center: V3) -> M4 {
    [
        right.x as f32, right.y as f32, right.z as f32, 0.0,
        up.x as f32, up.y as f32, up.z as f32, 0.0,
        back.x as f32, back.y as f32, back.z as f32, 0.0,
        center.x as f32, center.y as f32, center.z as f32, 1.0,
    ]
}

/// Small deterministic generator for procedural content.
pub struct Lcg(pub u32);
impl Lcg {
    pub fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        self.0 as f64 / 4294967296.0
    }
}
