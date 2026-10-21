//! Flight model and mission. A faithful port of the original game-tuned F-16C
//! Block 50 / F110-GE-129 model: ISA atmosphere, compressible pitot, lift-
//! dependent drag, wave drag, ground effect, stall, FBW G/alpha limiting,
//! engine spool and afterburner lag, thrust lapse, ram effect and fuel mass.
use crate::math::*;
use crate::terrain::{self, river_x, terrain_at};

pub const G0: f64 = 9.80665;
pub const FIXED_DT: f64 = 1.0 / 120.0;

pub const GATE_RADIUS: f64 = 47.0;
pub const MISSION_TIME: f64 = 150.0;
pub const GATE_BONUS: f64 = 8.0;
const ROUTE_Z: [f64; 10] = [7800.0, 6000.0, 4200.0, 2400.0, 500.0, -1500.0, -3500.0, -5400.0, -7200.0, -9000.0];
const ROUTE_OFFSET: [f64; 10] = [0.0, 115.0, -80.0, 160.0, -120.0, 145.0, -170.0, 90.0, -70.0, 0.0];
const ROUTE_CLEARANCE: [f64; 10] = [170.0, 135.0, 185.0, 125.0, 160.0, 140.0, 190.0, 130.0, 170.0, 150.0];

/// Public F-16C Block 50 / F110-GE-129 figures, rounded for a game-scale model.
pub mod spec {
    pub const EMPTY_MASS: f64 = 8273.0;
    pub const STORES_MASS: f64 = 450.0;
    pub const WING_AREA: f64 = 27.87;
    pub const DRY_THRUST: f64 = 76300.0;
    pub const AB_THRUST: f64 = 128922.0;
    pub const FUEL_MAX: f64 = 3200.0;
    pub const FUEL_RATE_IDLE: f64 = 0.18;
    pub const FUEL_RATE_DRY: f64 = 1.45;
    pub const FUEL_RATE_AB: f64 = 5.2;
    pub const CL_MAX: f64 = 1.63;
    pub const ALPHA_STALL: f64 = 0.445;
    pub const ALPHA_LIMIT: f64 = 0.455;
    pub const CD0: f64 = 0.019;
    pub const INDUCED_K: f64 = 0.118;
    pub const G_LIMIT: f64 = 9.0;
    pub const NEG_G_LIMIT: f64 = -2.4;
    pub const ROLL_RATE_MAX: f64 = 4.45;
    pub const MIN_ROLL_AUTHORITY: f64 = 0.18;
}
use spec::*;

#[derive(Clone, Copy, Debug)]
pub struct Atmosphere {
    pub rho: f64,
    pub sound: f64,
    pub pressure: f64,
}

/// ISA density, pressure and speed of sound up to 20 km.
pub fn atmosphere_at(altitude_m: f64) -> Atmosphere {
    let h = clamp(altitude_m, 0.0, 20000.0);
    let (rho, sound, pressure);
    if h <= 11000.0 {
        let theta = 1.0 - 2.25577e-5 * h;
        rho = 1.225 * theta.powf(4.2559);
        pressure = 101325.0 * theta.powf(5.2559);
        sound = (1.4 * 287.05287 * (288.15 - 0.0065 * h)).sqrt();
    } else {
        rho = 0.3639 * (-(h - 11000.0) / 6341.62).exp();
        sound = 295.1;
        pressure = 22632.1 * (-(h - 11000.0) / 6341.62).exp();
    }
    Atmosphere { rho: rho.max(0.0001), sound, pressure }
}

/// Compressible pitot impact pressure ratio, with a normal shock above Mach 1.
fn impact_ratio(mach: f64) -> f64 {
    if mach <= 1.0 {
        (1.0 + 0.2 * mach * mach).powf(3.5) - 1.0
    } else {
        (1.2 * mach * mach).powf(3.5) * (2.4 / (2.8 * mach * mach - 0.4)).powf(2.5) - 1.0
    }
}

pub fn calibrated_airspeed(speed: f64, atm: &Atmosphere) -> f64 {
    let impact = atm.pressure * impact_ratio(speed / atm.sound) / 101325.0;
    let (mut low, mut high) = (0.0, 5.0);
    for _ in 0..32 {
        let mid = (low + high) / 2.0;
        if impact_ratio(mid) < impact {
            low = mid;
        } else {
            high = mid;
        }
    }
    (low + high) * 0.5 * (1.4f64 * 287.05287 * 288.15).sqrt()
}

pub struct ContactPoint {
    pub name: &'static str,
    pub local: V3,
    pub margin: f64,
}

/// Body-space contact points sampled against the terrain every step, so a
/// banked wing can clip a canyon wall before the centre of mass gets there.
pub const COLLISION_POINTS: [ContactPoint; 10] = [
    ContactPoint { name: "nose", local: v3(0.0, 0.08, -6.95), margin: 0.10 },
    ContactPoint { name: "canopy", local: v3(0.0, 1.58, -3.35), margin: 0.10 },
    ContactPoint { name: "tail", local: v3(0.0, 0.03, 7.30), margin: 0.15 },
    ContactPoint { name: "left wing", local: v3(-4.98, 0.0, 2.45), margin: 0.08 },
    ContactPoint { name: "right wing", local: v3(4.98, 0.0, 2.45), margin: 0.08 },
    ContactPoint { name: "intake", local: v3(0.0, -1.24, -3.40), margin: 0.08 },
    ContactPoint { name: "belly", local: v3(0.0, -1.08, 0.0), margin: 0.10 },
    ContactPoint { name: "fin", local: v3(0.0, 3.34, 6.70), margin: 0.10 },
    ContactPoint { name: "left stabilator", local: v3(-2.79, -0.31, 7.25), margin: 0.06 },
    ContactPoint { name: "right stabilator", local: v3(2.79, -0.31, 7.25), margin: 0.06 },
];

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub pitch_up: bool,
    pub pitch_down: bool,
    pub roll_left: bool,
    pub roll_right: bool,
    pub yaw_left: bool,
    pub yaw_right: bool,
    pub pitch_axis: f64,
    pub roll_axis: f64,
    pub yaw_axis: f64,
    pub throttle_up: bool,
    pub throttle_down: bool,
    pub afterburner: bool,
    pub airbrake: bool,
}

impl Input {
    pub fn pitch(&self) -> f64 {
        clamp(self.pitch_axis + self.pitch_up as i32 as f64 - self.pitch_down as i32 as f64, -1.0, 1.0)
    }
    pub fn roll(&self) -> f64 {
        clamp(self.roll_axis + self.roll_right as i32 as f64 - self.roll_left as i32 as f64, -1.0, 1.0)
    }
    pub fn yaw(&self) -> f64 {
        clamp(self.yaw_axis + self.yaw_right as i32 as f64 - self.yaw_left as i32 as f64, -1.0, 1.0)
    }
}

#[derive(Clone, Debug)]
pub struct Gate {
    pub pos: V3,
    pub radius: f64,
    pub axis: V3,
    pub passed: bool,
}

#[derive(Clone, Debug)]
pub struct Mission {
    pub gates: Vec<Gate>,
    pub gate_index: usize,
    pub time_left: f64,
    pub elapsed: f64,
    pub score: f64,
    pub passed: usize,
    pub start_pos: V3,
    pub start_q: Quat,
    pub start_vel: V3,
    pub start_throttle: f64,
}

impl Mission {
    pub fn new() -> Mission {
        let mut gates: Vec<Gate> = (0..ROUTE_Z.len())
            .map(|i| {
                let z = ROUTE_Z[i];
                let x = river_x(z) + ROUTE_OFFSET[i];
                let ground = terrain_at(x, z);
                Gate { pos: v3(x, ground + ROUTE_CLEARANCE[i], z), radius: GATE_RADIUS, axis: v3(0.0, 0.0, -1.0), passed: false }
            })
            .collect();
        // Ring planes face the average course direction through each gate.
        for i in 0..gates.len() {
            let a = gates[i.saturating_sub(1)].pos;
            let b = gates[(i + 1).min(gates.len() - 1)].pos;
            gates[i].axis = v3(b.x - a.x, 0.0, b.z - a.z).norm();
        }
        // Start 2.4 km short of the first gate, looking down its throat.
        let first = &gates[0];
        let back = first.axis * -2400.0;
        let mut start = v3(first.pos.x + back.x, 0.0, first.pos.z + back.z);
        start.y = (first.pos.y + 150.0).max(terrain_at(start.x, start.z) + 180.0);
        let forward = (first.pos - start).norm();
        // Close to 1 G trim: a small nose-up attitude gives the wing its lift.
        let q = Quat::from_forward_up(forward, V3::Y).mul(Quat::axis_angle(V3::X, 2.35 * DEG));
        Mission {
            gates,
            gate_index: 0,
            time_left: MISSION_TIME,
            elapsed: 0.0,
            score: 0.0,
            passed: 0,
            start_pos: start,
            start_q: q,
            start_vel: forward * 218.0,
            start_throttle: 0.82,
        }
    }

    pub fn final_score(&self) -> f64 {
        (self.passed as f64 * 1000.0 + self.time_left.max(0.0) * 25.0).round()
    }

    pub fn current_gate(&self) -> &Gate {
        &self.gates[self.gate_index.min(self.gates.len() - 1)]
    }
}

#[derive(Clone, Debug)]
pub struct Aircraft {
    pub pos: V3,
    pub vel: V3,
    pub q: Quat,
    pub omega: V3,
    pub throttle: f64,
    pub engine_spool: f64,
    pub ab_level: f64,
    pub ab: bool,
    pub brake: f64,
    pub fuel: f64,
    pub mass: f64,
    pub speed: f64,
    pub airspeed_kt: f64,
    pub tas_kt: f64,
    pub eas_kt: f64,
    pub agl: f64,
    pub clearance: f64,
    pub alpha: f64,
    pub alpha_deg: f64,
    pub beta: f64,
    pub mach: f64,
    pub qbar: f64,
    pub g_load: f64,
    pub g_smooth: f64,
    pub max_g: f64,
    pub g_avail: f64,
    pub vs: f64,
    pub stall: f64,
    pub buffet: f64,
    pub turbulence: f64,
    pub afterburner_visual: f64,
    pub heading: f64,
    pub pitch: f64,
    pub bank: f64,
    pub crashed: bool,
    pub impact_speed: f64,
    pub impact_water: bool,
    pub collision_label: &'static str,
    pub last_pos: V3,
    pub cl: f64,
    pub cd: f64,
    pub lift_n: f64,
    pub drag_n: f64,
    pub thrust_n: f64,
    pub fuel_flow: f64,
    pub rpm: f64,
    pub ftit: f64,
    pub nozzle: f64,
    pub oil_psi: f64,
    /// Smoothed control-surface commands, for the exterior model.
    pub surf_pitch: f64,
    pub surf_roll: f64,
    pub surf_yaw: f64,
}

impl Aircraft {
    pub fn new(m: &Mission) -> Aircraft {
        let mut a = Aircraft {
            pos: m.start_pos,
            vel: m.start_vel,
            q: m.start_q,
            omega: V3::ZERO,
            throttle: m.start_throttle,
            engine_spool: m.start_throttle,
            ab_level: 0.0,
            ab: false,
            brake: 0.0,
            fuel: FUEL_MAX,
            mass: EMPTY_MASS + STORES_MASS + FUEL_MAX,
            speed: m.start_vel.len(),
            airspeed_kt: 0.0,
            tas_kt: 0.0,
            eas_kt: 0.0,
            agl: 0.0,
            clearance: 0.0,
            alpha: 0.0,
            alpha_deg: 0.0,
            beta: 0.0,
            mach: 0.0,
            qbar: 0.0,
            g_load: 1.0,
            g_smooth: 1.0,
            max_g: 1.0,
            g_avail: G_LIMIT,
            vs: 0.0,
            stall: 0.0,
            buffet: 0.0,
            turbulence: 0.0,
            afterburner_visual: 0.0,
            heading: 0.0,
            pitch: 0.0,
            bank: 0.0,
            crashed: false,
            impact_speed: 0.0,
            impact_water: false,
            collision_label: "",
            last_pos: m.start_pos,
            cl: 0.0,
            cd: 0.0,
            lift_n: 0.0,
            drag_n: 0.0,
            thrust_n: 0.0,
            fuel_flow: 0.0,
            rpm: 0.0,
            ftit: 0.0,
            nozzle: 0.0,
            oil_psi: 0.0,
            surf_pitch: 0.0,
            surf_roll: 0.0,
            surf_yaw: 0.0,
        };
        a.update(0.0, &Input::default());
        a
    }

    fn detect_terrain_collision(&mut self) {
        let mut min_clearance = f64::INFINITY;
        let mut hit: Option<(&ContactPoint, V3, f64)> = None;
        let mut hit_floor = 0.0;
        for point in COLLISION_POINTS.iter() {
            let p = self.pos + self.q.rotate(point.local);
            let floor = terrain::floor(p.x, p.z);
            let clearance = p.y - floor - point.margin;
            if clearance < min_clearance {
                min_clearance = clearance;
                hit_floor = floor;
            }
            if clearance < 0.0 && hit.is_none() {
                hit = Some((point, p, floor));
                hit_floor = floor;
            }
        }
        self.clearance = min_clearance.max(0.0);
        if let Some((point, _p, floor)) = hit {
            self.crashed = true;
            self.impact_speed = self.vel.len();
            self.impact_water = hit_floor < 0.5;
            self.collision_label = point.name;
            self.vel = self.vel * 0.08;
            self.omega = V3::ZERO;
            self.pos.y = self.pos.y.max(floor + 0.8);
        }
    }

    pub fn update(&mut self, dt: f64, input: &Input) {
        if self.crashed {
            return;
        }
        self.last_pos = self.pos;
        let ax = self.q.axes();
        let atm = atmosphere_at(self.pos.y);
        let rho = atm.rho;
        let speed = self.vel.len();
        let qbar = 0.5 * rho * speed * speed;
        let local_v = self.q.conj().rotate(self.vel);
        let forward_speed = (-local_v.z).max(0.001);
        let mut alpha = (-local_v.y).atan2(forward_speed.max(2.0));
        let mut beta = local_v.x.atan2(forward_speed.max(2.0));
        if speed < 3.0 {
            alpha = 0.0;
            beta = 0.0;
        }
        alpha = clamp(alpha, -1.15, 1.15);
        beta = clamp(beta, -0.75, 0.75);

        // Keyboard and mouse axes are additive.
        let pitch_in = input.pitch();
        let roll_in = input.roll();
        let yaw_in = input.yaw();

        if input.throttle_up {
            self.throttle = clamp(self.throttle + dt * 0.62, 0.0, 1.0);
        }
        if input.throttle_down {
            self.throttle = clamp(self.throttle - dt * 0.62, 0.0, 1.0);
        }

        let fueled = self.fuel > 0.0;
        let want_ab = input.afterburner && self.throttle >= 0.95 && fueled;
        self.ab = want_ab;
        self.brake = approach(self.brake, if input.airbrake { 1.0 } else { 0.0 }, dt * 2.5);
        self.mass = EMPTY_MASS + STORES_MASS + self.fuel;

        let agl_center = self.pos.y - terrain::floor(self.pos.x, self.pos.z);
        let ground_effect = 1.0 + 0.045 * (-agl_center.max(0.0) / 18.0).exp();
        let mach = speed / atm.sound;
        let cl_max_eff = CL_MAX * (1.0 - 0.28 * smoothstep(0.85, 1.25, mach)).max(0.82);
        let lift_capacity = qbar * WING_AREA * cl_max_eff * ground_effect;
        let g_available = lift_capacity / (self.mass * G0);
        self.g_avail = G_LIMIT.min(g_available.max(0.0));

        // FBW pitch: the stick commands a load factor. The feed-forward rate
        // turns the flight path as fast as that load factor does; the alpha
        // term builds (or sheds) the lift for it in about 0.3 s instead of
        // waiting for the airframe's own one-second alpha response, so the G
        // follows the stick both on the pull and on the release. With the
        // stick centred the steady state is the same path hold as before.
        let n_limit = 1.12f64.max(G_LIMIT.min(g_available));
        let n_demand = if pitch_in >= 0.0 { 1.0 + pitch_in * (n_limit - 1.0) } else { 1.0 + pitch_in * 2.2 };
        let n_target = clamp(n_demand - 1.0 + ax.up.y, NEG_G_LIMIT, n_limit);
        let lift_per_alpha = qbar * WING_AREA * ground_effect * cl_max_eff / ALPHA_STALL;
        // Centred, the jet never pitches itself into buffet to hold the path.
        let alpha_cap = mix(0.62 * ALPHA_STALL, ALPHA_LIMIT, pitch_in.max(0.0));
        let alpha_cmd = clamp(n_target * self.mass * G0 / lift_per_alpha.max(1.0), -0.38, alpha_cap);
        let mut pitch_target = G0 * (n_demand - 1.0) / speed.max(70.0) + 4.0 * (alpha_cmd - alpha);
        if pitch_target > 0.0 {
            pitch_target *= clamp((ALPHA_LIMIT - alpha.max(0.0)) / 0.07, 0.0, 1.0);
            if self.g_load > G_LIMIT + 0.12 {
                pitch_target = 0.0;
            }
        } else {
            pitch_target *= clamp((alpha + 0.38) / 0.08, 0.0, 1.0);
            if self.g_load < NEG_G_LIMIT - 0.1 {
                pitch_target = 0.0;
            }
        }
        pitch_target *= 1.0 - 0.18 * smoothstep(0.9, 1.6, mach);
        pitch_target = clamp(pitch_target, -0.9, 0.9);

        let q_ref = 0.5 * 1.225 * 200.0 * 200.0;
        let roll_authority = clamp((qbar.max(0.0) / q_ref).sqrt(), MIN_ROLL_AUTHORITY, 1.18);
        let max_roll_rate = 4.6f64.min(ROLL_RATE_MAX * roll_authority);
        let target_roll = -roll_in * max_roll_rate;
        let yaw_authority = clamp((qbar.max(0.0) / q_ref).sqrt(), 0.12, 1.05);
        let target_yaw = -yaw_in * 0.62 * yaw_authority - beta * 1.9 - roll_in * 0.018 * (speed / 220.0).min(1.0);

        let pitch_response = clamp(dt * (3.8 + qbar / 6500.0), 0.0, 1.0);
        let yaw_response = clamp(dt * (3.0 + qbar / 16000.0), 0.0, 1.0);
        let roll_response = clamp(dt * (4.5 + qbar / 12000.0), 0.0, 1.0);
        self.omega.x += (pitch_target - self.omega.x) * pitch_response;
        self.omega.y += (target_yaw - self.omega.y) * yaw_response;
        self.omega.z += (target_roll - self.omega.z) * roll_response;

        let surf = clamp(dt * 9.0, 0.0, 1.0);
        self.surf_pitch += (pitch_in - self.surf_pitch) * surf;
        self.surf_roll += (roll_in - self.surf_roll) * surf;
        self.surf_yaw += (yaw_in - self.surf_yaw) * surf;

        // Low-altitude roughness is recorded for camera shake and audio only.
        self.turbulence = 1.0 - smoothstep(300.0, 1700.0, self.pos.y - terrain::floor(self.pos.x, self.pos.z));
        self.q = self.q.integrate(self.omega, dt);

        // Aerodynamics --------------------------------------------------------
        let abs_alpha = alpha.abs();
        let mut cl = if abs_alpha <= ALPHA_STALL {
            cl_max_eff * (abs_alpha / ALPHA_STALL)
        } else {
            0.42f64.max(cl_max_eff - 1.95 * (abs_alpha - ALPHA_STALL))
        };
        cl = clamp(cl, 0.0, cl_max_eff);
        if alpha < 0.0 {
            cl = -cl;
        }
        let stall = smoothstep(ALPHA_STALL - 0.015, ALPHA_STALL + 0.11, abs_alpha);
        let mach_buffet = smoothstep(0.92, 1.08, mach) * (1.0 - smoothstep(1.16, 1.4, mach)) * 0.6;
        self.stall = stall;
        self.buffet = clamp(stall.max((abs_alpha - 0.32) / 0.15) + mach_buffet, 0.0, 1.0);

        let lift = qbar * WING_AREA * cl * ground_effect;
        let mut cd = CD0 + INDUCED_K * cl * cl;
        cd += self.brake * 0.05;
        let wave_drag = 0.004
            + 0.032 * (-((mach - 1.04) / 0.22).powi(2)).exp()
            + 0.018 * smoothstep(1.15, 1.85, mach);
        cd += wave_drag + stall * 0.10;
        let drag = qbar * WING_AREA * cd;
        let side = -qbar * WING_AREA * 1.25 * beta;

        let mut lift_dir_local = V3::X.cross(local_v.norm());
        if lift_dir_local.len() < 0.001 {
            lift_dir_local = V3::Y;
        }
        let lift_world = self.q.rotate(lift_dir_local.norm()) * lift;
        let side_world = self.q.rotate(v3(side, 0.0, 0.0));
        let drag_world = if speed > 0.5 { self.vel * (-drag / speed) } else { V3::ZERO };

        // Engine: spool and afterburner light-up are not instant; density lapse
        // and ram effect give less thrust upstairs and more at speed.
        let target_spool = if fueled {
            if want_ab {
                1.0
            } else {
                self.throttle.max(0.08)
            }
        } else {
            0.0
        };
        self.engine_spool += (target_spool - self.engine_spool) * clamp(dt * if want_ab { 2.0 } else { 1.4 }, 0.0, 1.0);
        self.ab_level += ((if want_ab { 1.0 } else { 0.0 }) - self.ab_level) * clamp(dt * 3.5, 0.0, 1.0);
        let density_ratio = rho / 1.225;
        let lapse = density_ratio.powf(0.72);
        let ram = ((1.0 + 0.22 * mach.min(1.6) - 0.06 * (mach - 1.6).max(0.0)) * (1.0 - 0.22 * smoothstep(2.0, 2.4, mach))).max(0.55);
        let mil_thrust = DRY_THRUST * (0.055 + 0.945 * self.throttle) * self.engine_spool;
        let ab_extra = (AB_THRUST - DRY_THRUST) * self.ab_level;
        self.thrust_n = if fueled { (mil_thrust + ab_extra) * lapse * ram } else { 0.0 };
        let thrust_world = ax.forward * self.thrust_n;
        let gravity = v3(0.0, -self.mass * G0, 0.0);

        let aero_force = lift_world + drag_world + side_world;
        let force = aero_force + thrust_world + gravity;
        let accel = force * (1.0 / self.mass);
        self.g_load = (aero_force + thrust_world).dot(ax.up) / (self.mass * G0);
        self.g_smooth += (self.g_load - self.g_smooth) * clamp(dt * 4.0, 0.0, 1.0);
        self.max_g = self.max_g.max(self.g_smooth);

        self.vel += accel * dt;
        self.pos += self.vel * dt;

        let fuel_flow = if want_ab { FUEL_RATE_AB } else { FUEL_RATE_IDLE + FUEL_RATE_DRY * clamp(self.throttle, 0.0, 1.0) };
        self.fuel_flow = if fueled { fuel_flow } else { 0.0 };
        self.fuel = (self.fuel - self.fuel_flow * dt).max(0.0);
        self.rpm = if self.fuel > 0.0 { 61.0 + 38.0 * self.engine_spool } else { 99.0 * self.engine_spool };
        self.ftit = if self.fuel > 0.0 {
            420.0 + 480.0 * self.engine_spool + 130.0 * self.ab_level
        } else {
            20.0 + 400.0 * self.engine_spool
        };
        self.nozzle = 15.0 + 80.0 * self.ab_level + 60.0 * (1.0 - self.engine_spool * 2.0).max(0.0);
        self.oil_psi = (self.rpm * 0.65).max(0.0);

        // Derived data for instruments.
        self.speed = self.vel.len();
        let atm2 = atmosphere_at(self.pos.y);
        self.airspeed_kt = calibrated_airspeed(self.speed, &atm2) * 1.94384;
        self.tas_kt = self.speed * 1.94384;
        self.eas_kt = self.tas_kt * (atm2.rho / 1.225).sqrt();
        self.agl = self.pos.y - terrain::floor(self.pos.x, self.pos.z);
        self.alpha = alpha;
        self.alpha_deg = alpha * RAD;
        self.beta = beta;
        self.mach = self.speed / atm2.sound;
        self.qbar = 0.5 * rho * self.speed * self.speed;
        self.cl = cl;
        self.cd = cd;
        self.lift_n = lift;
        self.drag_n = drag;
        self.vs = self.vel.y;
        let now = self.q.axes();
        self.heading = wrap(now.forward.x.atan2(-now.forward.z) * RAD, 360.0);
        self.pitch = clamp(now.forward.y, -1.0, 1.0).asin() * RAD;
        self.bank = (-now.right.y).atan2(now.up.y) * RAD;
        self.afterburner_visual += (self.ab_level - self.afterburner_visual) * clamp(dt * 5.0, 0.0, 1.0);

        self.detect_terrain_collision();
    }
}

/// True when the step from `last_pos` to `pos` crossed the gate disc.
pub fn gate_crossed(a: &Aircraft, gate: &Gate) -> bool {
    let d0 = (a.last_pos - gate.pos).dot(gate.axis);
    let d1 = (a.pos - gate.pos).dot(gate.axis);
    if !(d0 < 0.0 && d1 >= 0.0) {
        return false;
    }
    let denom = d0 - d1;
    let t = d0 / if denom == 0.0 { 1.0 } else { denom };
    let crossing = a.last_pos.lerp(a.pos, t);
    let rel = crossing - gate.pos;
    let lateral = rel - gate.axis * rel.dot(gate.axis);
    lateral.len() <= gate.radius * 0.96
}

#[cfg(test)]
mod tests {
    use super::*;
    const DT: f64 = 1.0 / 120.0;

    fn aircraft(alt: f64, speed: f64) -> Aircraft {
        let mut a = Aircraft::new(&Mission::new());
        a.pos = v3(0.0, alt, 0.0);
        a.vel = v3(0.0, 0.0, -speed);
        a.q = Quat::from_forward_up(v3(0.0, 0.0, -1.0), V3::Y).mul(Quat::axis_angle(V3::X, 2.35 * DEG));
        a
    }
    fn run(mut a: Aircraft, seconds: f64, input: Input) -> Aircraft {
        for _ in 0..(seconds / DT).round() as usize {
            a.update(DT, &input);
        }
        a
    }
    fn full() -> Input {
        Input { roll_right: true, afterburner: true, throttle_up: true, ..Default::default() }
    }

    #[test]
    fn initial_instruments_are_valid_before_the_first_frame() {
        let a = Aircraft::new(&Mission::new());
        assert!(a.speed > 200.0 && a.airspeed_kt > 300.0);
        assert!(a.mach > 0.5 && a.agl > 100.0);
        assert!(a.rpm > 80.0);
    }

    #[test]
    fn isa_sea_level_and_tropopause() {
        let sea = atmosphere_at(0.0);
        let high = atmosphere_at(11000.0);
        assert!((sea.rho - 1.225).abs() < 0.001);
        assert!((sea.sound - 340.294).abs() < 0.1);
        assert!((high.pressure - 22632.0).abs() < 10.0);
        assert!((high.sound - 295.07).abs() < 0.2);
    }

    #[test]
    fn compressible_calibrated_airspeed() {
        for speed in [50.0, 200.0, 340.0, 500.0, 680.0] {
            assert!((calibrated_airspeed(speed, &atmosphere_at(0.0)) - speed).abs() < 0.001);
        }
        assert!(calibrated_airspeed(300.0, &atmosphere_at(10000.0)) < 200.0);
        let a = run(aircraft(10000.0, 300.0), DT, Input::default());
        assert!(a.airspeed_kt < a.tas_kt);
        assert!(a.mach > 1.0);
    }

    #[test]
    fn altitude_reduces_qbar_lift_roll_and_thrust() {
        let low = run(aircraft(3000.0, 220.0), 0.5, full());
        let high = run(aircraft(10000.0, 220.0), 0.5, full());
        assert!(high.qbar < low.qbar * 0.5);
        assert!(high.g_avail < low.g_avail);
        assert!(high.omega.z.abs() < low.omega.z.abs());
        assert!(high.thrust_n < low.thrust_n);
    }

    #[test]
    fn airspeed_changes_attainable_g_and_roll() {
        let i = Input { roll_right: true, pitch_up: true, ..Default::default() };
        let slow = run(aircraft(3000.0, 100.0), 0.25, i);
        let fast = run(aircraft(3000.0, 250.0), 0.25, i);
        assert!(slow.g_avail < fast.g_avail);
        assert!(slow.omega.z.abs() < fast.omega.z.abs());
    }

    #[test]
    fn fuel_exhaustion_removes_thrust_and_flow() {
        let mut a = aircraft(3000.0, 220.0);
        a.fuel = 0.0;
        let a = run(a, 1.0, Input { afterburner: true, throttle_up: true, ..Default::default() });
        assert_eq!(a.thrust_n, 0.0);
        assert_eq!(a.fuel_flow, 0.0);
        assert!(!a.ab);
        assert_eq!(a.fuel, 0.0);
    }

    #[test]
    fn afterburner_needs_military_throttle_and_lags() {
        let mut a = aircraft(3000.0, 220.0);
        a.throttle = 0.7;
        let ab = Input { afterburner: true, ..Default::default() };
        a.update(DT, &ab);
        assert!(!a.ab);
        a.throttle = 1.0;
        a.update(DT, &ab);
        assert!(a.ab);
        assert!(a.ab_level > 0.0 && a.ab_level < 0.1);
    }

    #[test]
    fn inverted_bank_reads_near_180() {
        let mut a = aircraft(3000.0, 220.0);
        a.q = a.q.mul(Quat::axis_angle(V3::Z, PI));
        let a = run(a, DT, Input::default());
        assert!(a.bank.abs() > 170.0);
    }

    #[test]
    fn load_factor_follows_the_stick_quickly() {
        let mut a = Aircraft::new(&Mission::new());
        let pull = Input { pitch_up: true, ..Default::default() };
        for _ in 0..48 {
            a.update(DT, &pull);
        }
        assert!(a.g_load > 5.5, "0.4 s into a full pull: {:.2} g", a.g_load);
        for _ in 0..72 {
            a.update(DT, &pull);
        }
        assert!(a.g_load > 8.0 && a.g_load < G_LIMIT + 0.3, "sustained pull: {:.2} g", a.g_load);
        for _ in 0..60 {
            a.update(DT, &Input::default());
        }
        assert!(a.g_load < 2.0, "0.5 s after release: {:.2} g", a.g_load);
    }

    #[test]
    fn airbrake_adds_drag() {
        let clean = run(aircraft(3000.0, 250.0), 2.0, Input::default());
        let braked = run(aircraft(3000.0, 250.0), 2.0, Input { airbrake: true, ..Default::default() });
        assert!(braked.drag_n > clean.drag_n);
        assert!(braked.speed < clean.speed);
    }

    #[test]
    fn engine_instruments_follow_spool_and_consumption() {
        let a0 = aircraft(3000.0, 220.0);
        let fuel = a0.fuel;
        let a = run(a0, 3.0, Input { throttle_up: true, afterburner: true, ..Default::default() });
        assert!(a.fuel < fuel);
        assert!(a.rpm > 90.0 && a.ftit > 800.0 && a.nozzle > 80.0 && a.oil_psi > 50.0);
    }

    #[test]
    fn simulation_stays_finite_across_the_envelope() {
        for alt in [1500.0, 6000.0, 12000.0, 16000.0] {
            for speed in [90.0, 220.0, 400.0, 600.0] {
                let a = run(aircraft(alt, speed), 12.0, Input { pitch_axis: 0.25, roll_axis: 0.07, ..Default::default() });
                assert!(a.pos.is_finite() && a.vel.is_finite());
                assert!(a.g_load.is_finite() && a.alpha.is_finite() && a.airspeed_kt.is_finite());
                assert!((a.q.len() - 1.0).abs() < 1e-6);
                assert!(a.fuel >= 0.0 && a.g_avail <= 9.0);
            }
        }
    }

    /// Game-model calibration, not a claim of validated F-16 performance tables:
    /// full-afterburner level-flight equilibrium Mach at 40,000 ft must stay near
    /// the published Block 50 short-duration figure of Mach 2.05.
    #[test]
    fn full_afterburner_equilibrium_matches_reference() {
        let equilibrium = |altitude: f64, mach: f64| {
            let atm = atmosphere_at(altitude);
            let speed = mach * atm.sound;
            let q = 0.5 * atm.rho * speed * speed;
            let mut a = Aircraft::new(&Mission::new());
            a.pos = v3(0.0, altitude, 0.0);
            a.vel = v3(0.0, 0.0, -speed);
            let cl_max = CL_MAX * (1.0 - 0.28 * smoothstep(0.85, 1.25, mach)).max(0.82);
            let alpha = a.mass * G0 / (q * WING_AREA * cl_max) * ALPHA_STALL;
            a.q = Quat::from_forward_up(v3(0.0, 0.0, -1.0), V3::Y).mul(Quat::axis_angle(V3::X, alpha));
            a.throttle = 1.0;
            a.engine_spool = 1.0;
            a.ab_level = 1.0;
            a.update(0.0, &Input { afterburner: true, ..Default::default() });
            a.thrust_n * alpha.cos() - a.drag_n
        };
        let mut mach = 0.0;
        for feet in [10000.0, 20000.0, 30000.0, 40000.0] {
            let (mut low, mut high) = (1.1, 2.6);
            for _ in 0..35 {
                let mid = (low + high) / 2.0;
                if equilibrium(feet * 0.3048, mid) > 0.0 {
                    low = mid;
                } else {
                    high = mid;
                }
            }
            mach = (low + high) / 2.0;
            println!("{feet} ft: level full-AB Mach {mach:.3}");
        }
        assert!((mach - 2.05).abs() < 0.1, "equilibrium Mach {mach}");
    }

    #[test]
    fn gate_counts_only_forward_crossings_inside_the_ring() {
        let m = Mission::new();
        let g = &m.gates[0];
        let mut a = Aircraft::new(&m);
        let side = g.axis.cross(V3::Y).norm();
        let mut cross = |offset: V3, forward: bool| {
            let (from, to) = if forward { (-1.0, 1.0) } else { (1.0, -1.0) };
            a.last_pos = g.pos + offset + g.axis * from;
            a.pos = g.pos + offset + g.axis * to;
            gate_crossed(&a, g)
        };
        assert!(cross(V3::ZERO, true));
        assert!(cross(side * (GATE_RADIUS * 0.9), true));
        assert!(!cross(side * (GATE_RADIUS * 1.02), true));
        assert!(!cross(V3::ZERO, false));
    }

    #[test]
    fn score_counts_gates_and_remaining_time() {
        let mut m = Mission::new();
        m.passed = 4;
        m.time_left = 37.4;
        assert_eq!(m.final_score(), (4000.0f64 + 37.4 * 25.0).round());
        m.time_left = -3.0;
        assert_eq!(m.final_score(), 4000.0);
    }

    #[test]
    fn mission_route_has_ten_gates_above_the_gorge_floor() {
        let m = Mission::new();
        assert_eq!(m.gates.len(), 10);
        for g in &m.gates {
            assert!(g.pos.y - terrain_at(g.pos.x, g.pos.z) > 100.0);
        }
        assert!(m.start_pos.y > terrain_at(m.start_pos.x, m.start_pos.z) + 150.0);
    }
}
