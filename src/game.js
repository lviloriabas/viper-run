/* =========================================================================
   VIPER RUN // F-16C COCKPIT SIMULATOR
   A self-contained WebGL2 + Canvas2D retro flight experience.
   No external libraries, no network requests.
   ========================================================================= */
(() => {
  "use strict";

  // ---------------------------------------------------------------- constants
  const TAU = Math.PI * 2;
  const DEG = Math.PI / 180;
  const RAD = 180 / Math.PI;
  const G0 = 9.80665;
  const FIXED_DT = 1 / 120;
  const MAX_FRAME_DT = 0.05;
  const KM = 1000;

  const clamp = (v, a, b) => (v < a ? a : v > b ? b : v);
  const mix = (a, b, t) => a + (b - a) * t;
  const smoothstep = (a, b, x) => {
    const t = clamp((x - a) / (b - a), 0, 1);
    return t * t * (3 - 2 * t);
  };
  const wrap = (v, m) => ((v % m) + m) % m;
  const approach = (v, target, delta) =>
    v < target ? Math.min(v + delta, target) : Math.max(v - delta, target);

  // --------------------------------------------------------------- vector math
  const v3 = (x = 0, y = 0, z = 0) => [x, y, z];
  const add = (a, b) => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
  const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
  const mul = (a, s) => [a[0] * s, a[1] * s, a[2] * s];
  const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  const cross = (a, b) => [
    a[1] * b[2] - a[2] * b[1],
    a[2] * b[0] - a[0] * b[2],
    a[0] * b[1] - a[1] * b[0],
  ];
  const len = (a) => Math.hypot(a[0], a[1], a[2]);
  const normalize = (a) => {
    const l = len(a);
    return l > 1e-9 ? [a[0] / l, a[1] / l, a[2] / l] : [0, 0, 0];
  };
  const lerp3 = (a, b, t) => [mix(a[0], b[0], t), mix(a[1], b[1], t), mix(a[2], b[2], t)];

  // ------------------------------------------------------------- quaternion math
  const qMul = (a, b) => [
    a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
    a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
    a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
    a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
  ];
  const qNorm = (q) => {
    const l = Math.hypot(q[0], q[1], q[2], q[3]) || 1;
    return [q[0] / l, q[1] / l, q[2] / l, q[3] / l];
  };
  const qAxis = (axis, angle) => {
    const a = normalize(axis);
    const s = Math.sin(angle * 0.5);
    return [a[0] * s, a[1] * s, a[2] * s, Math.cos(angle * 0.5)];
  };
  const qRotate = (q, v) => {
    const qv = [q[0], q[1], q[2]];
    const t = mul(cross(qv, v), 2);
    return add(add(v, mul(t, q[3])), cross(qv, t));
  };
  const qFromBasis = (right, up, back) => {
    const m00 = right[0], m10 = right[1], m20 = right[2];
    const m01 = up[0], m11 = up[1], m21 = up[2];
    const m02 = back[0], m12 = back[1], m22 = back[2];
    const trace = m00 + m11 + m22;
    let x, y, z, w, s;
    if (trace > 0) {
      s = Math.sqrt(trace + 1) * 2;
      w = 0.25 * s;
      x = (m21 - m12) / s;
      y = (m02 - m20) / s;
      z = (m10 - m01) / s;
    } else if (m00 > m11 && m00 > m22) {
      s = Math.sqrt(1 + m00 - m11 - m22) * 2;
      w = (m21 - m12) / s;
      x = 0.25 * s;
      y = (m01 + m10) / s;
      z = (m02 + m20) / s;
    } else if (m11 > m22) {
      s = Math.sqrt(1 + m11 - m00 - m22) * 2;
      w = (m02 - m20) / s;
      x = (m01 + m10) / s;
      y = 0.25 * s;
      z = (m12 + m21) / s;
    } else {
      s = Math.sqrt(1 + m22 - m00 - m11) * 2;
      w = (m10 - m01) / s;
      x = (m02 + m20) / s;
      y = (m12 + m21) / s;
      z = 0.25 * s;
    }
    return qNorm([x, y, z, w]);
  };
  const qFromForwardUp = (forward, upHint = [0, 1, 0]) => {
    const f = normalize(forward);
    let r = normalize(cross(f, upHint));
    if (len(r) < 1e-4) r = normalize(cross(f, [0, 0, 1]));
    const u = normalize(cross(r, f));
    return qFromBasis(r, u, mul(f, -1));
  };
  const axesFromQ = (q) => {
    const right = qRotate(q, [1, 0, 0]);
    const up = qRotate(q, [0, 1, 0]);
    const back = qRotate(q, [0, 0, 1]);
    return { right, up, back, forward: mul(back, -1) };
  };
  const qIntegrate = (q, omega, dt) => {
    const w = len(omega);
    if (w < 1e-8) return q;
    return qNorm(qMul(q, qAxis(omega, w * dt)));
  };

  // --------------------------------------------------------------- matrix math
  const mat4Perspective = (fovy, aspect, near, far) => {
    const f = 1 / Math.tan(fovy / 2);
    const nf = 1 / (near - far);
    return new Float32Array([
      f / aspect, 0, 0, 0,
      0, f, 0, 0,
      0, 0, (far + near) * nf, -1,
      0, 0, 2 * far * near * nf, 0,
    ]);
  };
  const mat4LookAt = (eye, center, up) => {
    const z = normalize(sub(eye, center));
    const x = normalize(cross(up, z));
    const y = cross(z, x);
    return new Float32Array([
      x[0], y[0], z[0], 0,
      x[1], y[1], z[1], 0,
      x[2], y[2], z[2], 0,
      -dot(x, eye), -dot(y, eye), -dot(z, eye), 1,
    ]);
  };
  const mat4Mul = (a, b) => {
    const out = new Float32Array(16);
    for (let c = 0; c < 4; c++) {
      for (let r = 0; r < 4; r++) {
        out[c * 4 + r] =
          a[r] * b[c * 4] +
          a[4 + r] * b[c * 4 + 1] +
          a[8 + r] * b[c * 4 + 2] +
          a[12 + r] * b[c * 4 + 3];
      }
    }
    return out;
  };
  const mat4Basis = (right, up, back, center) => new Float32Array([
    right[0], right[1], right[2], 0,
    up[0], up[1], up[2], 0,
    back[0], back[1], back[2], 0,
    center[0], center[1], center[2], 1,
  ]);

  // ---------------------------------------------------------------- terrain model
  // Kept in precise lock-step with the GLSL version below so terrain collision,
  // gate placement, the map display and the rendered world all agree.
  const fract = (x) => x - Math.floor(x);
  const hash21 = (x, y) => {
    let px = fract(x * 123.34);
    let py = fract(y * 456.21);
    const d = px * px + py * py + 45.32 * (px + py);
    px += d;
    py += d;
    return fract(px * py);
  };
  const vnoise = (x, y) => {
    const ix = Math.floor(x), iy = Math.floor(y);
    let fx = x - ix, fy = y - iy;
    fx = fx * fx * (3 - 2 * fx);
    fy = fy * fy * (3 - 2 * fy);
    const a = hash21(ix, iy);
    const b = hash21(ix + 1, iy);
    const c = hash21(ix, iy + 1);
    const d = hash21(ix + 1, iy + 1);
    return mix(mix(a, b, fx), mix(c, d, fx), fy);
  };
  const fbm = (x, y, octaves = 5) => {
    let sum = 0, amp = 0.5;
    for (let i = 0; i < octaves; i++) {
      sum += amp * vnoise(x, y);
      x = x * 2.03 + 17.3;
      y = y * 2.03 + 9.2;
      amp *= 0.5;
    }
    return sum;
  };
  const riverX = (z) => 1500 * Math.sin(z / 5200) + 420 * Math.sin(z / 1900 + 0.7);

  const terrainAt = (x, z) => {
    const wx = (fbm(x / 17000 + 4.2, z / 17000 + 8.1) - 0.5) * 1800;
    const wz = (fbm(x / 14000 - 3.3, z / 14000 + 5.8) - 0.5) * 1400;
    const xx = x + wx;
    const zz = z + wz;
    const base = fbm(xx / 13000 + 2.0, zz / 13000 + 9.4);
    const ridgeN = fbm(xx / 4600 + 1.7, zz / 4600 - 3.2);
    const ridge = 1 - Math.abs(ridgeN * 2 - 1);
    let h = -380 + base * 1050 + Math.pow(ridge, 2.3) * 2550;
    h += (fbm(x / 1000 + 8.0, z / 1000 + 3.0) - 0.5) * 110;
    h += (fbm(x / 300 + 3.0, z / 300 + 7.0) - 0.5) * 22;
    const coast = smoothstep(-13500, -9000, z);
    const sea = -320 + (fbm(x / 5000, z / 5000) - 0.5) * 80;
    h = mix(sea, h, coast);
    const d = Math.abs(x - riverX(z));
    const riverN = fbm(x / 1300 + 5.5, z / 1300 + 1.7);
    const floor = 300 + riverN * 130 + 0.012 * (z + 3000);
    const wall = smoothstep(420, 1650, d);
    h = mix(floor, h, wall);
    return h;
  };

  // ------------------------------------------------------------------- mission
  const ROUTE_Z = [7800, 6000, 4200, 2400, 500, -1500, -3500, -5400, -7200, -9000];
  const ROUTE_OFFSET = [0, 115, -80, 160, -120, 145, -170, 90, -70, 0];
  const ROUTE_CLEARANCE = [170, 135, 185, 125, 160, 140, 190, 130, 170, 150];
  const GATE_RADIUS = 47;
  const MISSION_TIME = 150;
  const GATE_BONUS = 8;

  const AIRCRAFT_SPEC = {
    mass: 11400,
    wingArea: 27.87,
    dryThrust: 76000,
    abThrust: 129000,
    fuelMax: 3200,
    fuelRateDry: 0.9,
    fuelRateAB: 3.1,
    clMax: 1.62,
    cd0: 0.022,
    inducedK: 0.105,
  };

  function buildMission() {
    const gates = [];
    for (let i = 0; i < ROUTE_Z.length; i++) {
      const z = ROUTE_Z[i];
      const x = riverX(z) + ROUTE_OFFSET[i];
      const ground = terrainAt(x, z);
      const y = ground + ROUTE_CLEARANCE[i];
      gates.push({
        pos: [x, y, z],
        ground,
        radius: GATE_RADIUS,
        axis: [0, 0, -1],
        passed: false,
      });
    }

    // Ring planes face the average course direction through each gate.
    for (let i = 0; i < gates.length; i++) {
      const a = gates[Math.max(0, i - 1)].pos;
      const b = gates[Math.min(gates.length - 1, i + 1)].pos;
      const ax = b[0] - a[0];
      const az = b[2] - a[2];
      gates[i].axis = normalize([ax, 0, az]);
    }

    // Start 2.4 km short of the first gate, looking down its throat.
    const first = gates[0];
    const back = mul(first.axis, -2400);
    const start = [first.pos[0] + back[0], 0, first.pos[2] + back[2]];
    start[1] = Math.max(first.pos[1] + 150, terrainAt(start[0], start[2]) + 180);
    const forward = normalize(sub(first.pos, start));
    const q = qFromForwardUp(forward, [0, 1, 0]);
    const initialSpeed = 218;

    const mapBounds = { xMin: -4200, xMax: 4200, zMin: -11200, zMax: 13000 };

    return {
      gates,
      gateIndex: 0,
      timeLeft: MISSION_TIME,
      elapsed: 0,
      score: 0,
      passed: 0,
      bestGate: -1,
      message: "COLD RIVER // CLEARED HOT",
      messageTimer: 3.2,
      mapBounds,
      start: {
        pos: start,
        q,
        velocity: mul(forward, initialSpeed),
        throttle: 0.82,
      },
    };
  }

  // ---------------------------------------------------------------- flight model
  function makeAircraft(mission) {
    return {
      pos: [...mission.start.pos],
      vel: [...mission.start.velocity],
      q: [...mission.start.q],
      omega: [0, 0, 0],
      throttle: mission.start.throttle,
      thrustLevel: mission.start.throttle * 0.75,
      ab: false,
      brake: 0,
      fuel: AIRCRAFT_SPEC.fuelMax,
      speed: 0,
      airspeedKt: 0,
      altitudeM: mission.start.pos[1],
      agl: 0,
      alpha: 0,
      alphaDeg: 0,
      beta: 0,
      mach: 0,
      gLoad: 1,
      gSmooth: 1,
      vs: 0,
      stall: 0,
      buffet: 0,
      afterburnerVisual: 0,
      ground: 0,
      heading: 0,
      pitch: 0,
      bank: 0,
      maxQ: 0,
      crashed: false,
      lastPos: [...mission.start.pos],
      loadAccel: 0,
    };
  }

  function axisSet(q) {
    return axesFromQ(q);
  }

  function updateAircraft(a, dt, input) {
    if (a.crashed) return;

    a.lastPos = [...a.pos];
    const ax = axisSet(a.q);
    const rho = 1.225 * Math.exp(-clamp(a.pos[1], 0, 20000) / 8500);
    const sound = 340.3 - 0.004 * clamp(a.pos[1], 0, 12000);
    const speed = len(a.vel);
    const qbar = 0.5 * rho * speed * speed;
    const localV = qRotate([-a.q[0], -a.q[1], -a.q[2], a.q[3]], a.vel);
    const forwardSpeed = Math.max(-localV[2], 0.001);
    let alpha = Math.atan2(-localV[1], Math.max(forwardSpeed, 2));
    let beta = Math.atan2(localV[0], Math.max(forwardSpeed, 2));
    if (speed < 3) {
      alpha = 0;
      beta = 0;
    }
    alpha = clamp(alpha, -1.15, 1.15);
    beta = clamp(beta, -0.75, 0.75);

    // Pilot inputs -----------------------------------------------------------
    let pitchIn = 0, rollIn = 0, yawIn = 0;
    if (input.pitchUp) pitchIn += 1;
    if (input.pitchDown) pitchIn -= 1;
    if (input.rollRight) rollIn += 1;
    if (input.rollLeft) rollIn -= 1;
    if (input.yawRight) yawIn += 1;
    if (input.yawLeft) yawIn -= 1;

    if (input.throttleUp) a.throttle = clamp(a.throttle + dt * 0.62, 0, 1);
    if (input.throttleDown) a.throttle = clamp(a.throttle - dt * 0.62, 0, 1);

    const wantAB = input.afterburner && a.throttle > 0.55 && a.fuel > 1;
    a.ab = wantAB;
    a.brake = approach(a.brake, input.airbrake ? 1 : 0, dt * 2.5);

    // Fly-by-wire-ish rate demand. The limiter keeps the jet from pulling
    // absurd angles of attack or G at high speed, while leaving it lively.
    const absAlpha = Math.abs(alpha);
    let pitchLimit = 1.0;
    if (pitchIn > 0) {
      // FBW alpha limiter: ease the commanded rate to zero around 27 degrees.
      pitchLimit *= clamp((0.50 - alpha) / 0.075, 0, 1);
      const gTurnLimit = speed > 55 ? (87.8 / speed) : 1.15;
      pitchLimit = Math.min(pitchLimit, Math.max(0.17, gTurnLimit));
    } else if (pitchIn < 0) {
      pitchLimit *= clamp((alpha + 0.38) / 0.11, 0, 1);
    }
    const maxPitchRate = 1.12 * clamp(speed / 110, 0.22, 1) * pitchLimit;
    const maxRollRate = clamp(0.75 + speed / 140, 1.1, 3.1);
    const maxYawRate = 0.5;
    const targetPitch = pitchIn * maxPitchRate;
    const targetRoll = -rollIn * maxRollRate;
    const targetYaw = -yawIn * maxYawRate - beta * 1.4 - rollIn * 0.018 * Math.min(speed / 220, 1);
    const response = clamp(dt * (speed > 20 ? 7 : 2), 0, 1);
    a.omega[0] += (targetPitch - a.omega[0]) * response;
    a.omega[1] += (targetYaw - a.omega[1]) * response * 0.72;
    a.omega[2] += (targetRoll - a.omega[2]) * response;

    // Record low-altitude roughness for camera shake, but never let it wander
    // the flight path or accumulate as a bank angle.
    a.turbulence = 1 - smoothstep(300, 1700, a.pos[1] - Math.max(0, terrainAt(a.pos[0], a.pos[2])));
    a.q = qIntegrate(a.q, a.omega, dt);

    // Aerodynamics -----------------------------------------------------------
    let cl = 0;
    if (absAlpha < 0.44) {
      cl = 3.75 * absAlpha + 0.035;
    } else {
      cl = Math.max(0.4, 1.685 - (absAlpha - 0.44) * 2.35);
    }
    cl = Math.min(cl, AIRCRAFT_SPEC.clMax + 0.06);
    if (alpha < 0) cl = -cl;
    const stall = smoothstep(0.46, 0.76, absAlpha);
    a.stall = stall;
    a.buffet = Math.max(stall, clamp((absAlpha - 0.36) / 0.16, 0, 1));

    // Ground effect is tiny for the Viper, but it is a nice low-level touch.
    const agl = a.pos[1] - Math.max(0, terrainAt(a.pos[0], a.pos[2]));
    const groundEffect = 1 + 0.045 * Math.exp(-Math.max(0, agl) / 18);
    const lift = qbar * AIRCRAFT_SPEC.wingArea * cl * groundEffect;
    let cd = AIRCRAFT_SPEC.cd0 + AIRCRAFT_SPEC.inducedK * cl * cl;
    cd += a.brake * 0.045;
    const mach = speed / sound;
    cd += smoothstep(0.84, 1.08, mach) * 0.017;
    cd += stall * 0.09;
    const drag = qbar * AIRCRAFT_SPEC.wingArea * cd;
    const side = -qbar * AIRCRAFT_SPEC.wingArea * 1.25 * beta;

    let liftDirLocal = cross([1, 0, 0], normalize(localV));
    if (len(liftDirLocal) < 0.001) liftDirLocal = [0, 1, 0];
    liftDirLocal = normalize(liftDirLocal);
    const liftWorld = mul(qRotate(a.q, liftDirLocal), lift);
    const sideWorld = qRotate(a.q, [side, 0, 0]);
    const dragWorld = speed > 0.5 ? mul(a.vel, -drag / speed) : [0, 0, 0];

    // Engine -----------------------------------------------------------------
    const densityRatio = rho / 1.225;
    const targetThrustLevel = a.throttle * (wantAB ? 1 : 0.58);
    a.thrustLevel += (targetThrustLevel - a.thrustLevel) * clamp(dt * 4.5, 0, 1);
    const thrustMax = wantAB ? AIRCRAFT_SPEC.abThrust : AIRCRAFT_SPEC.dryThrust;
    const thrust = a.thrustLevel * thrustMax * Math.pow(densityRatio, 0.72);
    const thrustWorld = mul(ax.forward, thrust);
    const gravity = [0, -AIRCRAFT_SPEC.mass * G0, 0];

    const force = add(add(add(add(liftWorld, dragWorld), sideWorld), thrustWorld), gravity);
    const accel = mul(force, 1 / AIRCRAFT_SPEC.mass);
    a.loadAccel = dot(add(add(liftWorld, dragWorld), sideWorld), ax.up) / (AIRCRAFT_SPEC.mass * G0);
    a.gLoad = a.loadAccel;
    a.gSmooth += (a.gLoad - a.gSmooth) * clamp(dt * 4, 0, 1);

    a.vel = add(a.vel, mul(accel, dt));
    a.pos = add(a.pos, mul(a.vel, dt));

    // Fuel burn is visual but persistent for the sortie.
    const burn = (wantAB ? AIRCRAFT_SPEC.fuelRateAB : AIRCRAFT_SPEC.fuelRateDry * a.throttle) * dt;
    a.fuel = Math.max(0, a.fuel - burn);

    // Derived data for instruments.
    a.speed = len(a.vel);
    a.airspeedKt = a.speed * 1.94384;
    a.altitudeM = a.pos[1];
    a.agl = a.pos[1] - Math.max(0, terrainAt(a.pos[0], a.pos[2]));
    a.alpha = alpha;
    a.alphaDeg = alpha * RAD;
    a.beta = beta;
    a.mach = a.speed / sound;
    a.vs = a.vel[1];
    a.heading = wrap(Math.atan2(ax.forward[0], -ax.forward[2]) * RAD, 360);
    a.pitch = Math.asin(clamp(ax.forward[1], -1, 1)) * RAD;
    a.bank = Math.atan2(-ax.right[1], Math.max(0.001, ax.up[1])) * RAD;
    a.afterburnerVisual += ((wantAB ? 1 : 0) - a.afterburnerVisual) * clamp(dt * 5, 0, 1);

    // Crash detection.
    const floor = Math.max(0, terrainAt(a.pos[0], a.pos[2]));
    if (a.pos[1] < floor + 1.3) {
      a.pos[1] = floor + 1.3;
      a.crashed = true;
      a.impactSpeed = a.speed;
      a.impactVS = a.vel[1];
    }
  }

  // --------------------------------------------------------------------- audio
  class Sound {
    constructor() {
      this.ctx = null;
      this.master = null;
      this.muted = false;
      this.warnClock = 0;
      this.ready = false;
    }

    init() {
      if (this.ctx) {
        if (this.ctx.state === "suspended") this.ctx.resume().catch(() => {});
        return;
      }
      const AC = window.AudioContext || window.webkitAudioContext;
      if (!AC) return;
      try {
        const ctx = new AC();
        this.ctx = ctx;
        const master = ctx.createGain();
        master.gain.value = this.muted ? 0 : 0.5;
        master.connect(ctx.destination);
        this.master = master;

        const seconds = 2;
        const buffer = ctx.createBuffer(1, ctx.sampleRate * seconds, ctx.sampleRate);
        const data = buffer.getChannelData(0);
        let last = 0;
        for (let i = 0; i < data.length; i++) {
          const white = Math.random() * 2 - 1;
          last = last * 0.965 + white * 0.035;
          data[i] = last * 5.5;
        }
        const noise = ctx.createBufferSource();
        noise.buffer = buffer;
        noise.loop = true;

        const engFilter = ctx.createBiquadFilter();
        engFilter.type = "lowpass";
        engFilter.frequency.value = 380;
        engFilter.Q.value = 0.6;
        const engGain = ctx.createGain();
        engGain.gain.value = 0;
        noise.connect(engFilter).connect(engGain).connect(master);

        const windFilter = ctx.createBiquadFilter();
        windFilter.type = "bandpass";
        windFilter.frequency.value = 900;
        windFilter.Q.value = 0.35;
        const windGain = ctx.createGain();
        windGain.gain.value = 0;
        const wind = ctx.createBufferSource();
        wind.buffer = buffer;
        wind.loop = true;
        wind.playbackRate.value = 1.35;
        wind.connect(windFilter).connect(windGain).connect(master);

        const rumble = ctx.createOscillator();
        rumble.type = "sawtooth";
        rumble.frequency.value = 47;
        const rumbleFilter = ctx.createBiquadFilter();
        rumbleFilter.type = "lowpass";
        rumbleFilter.frequency.value = 120;
        const rumbleGain = ctx.createGain();
        rumbleGain.gain.value = 0;
        rumble.connect(rumbleFilter).connect(rumbleGain).connect(master);

        const whine = ctx.createOscillator();
        whine.type = "sine";
        whine.frequency.value = 740;
        const whineGain = ctx.createGain();
        whineGain.gain.value = 0;
        whine.connect(whineGain).connect(master);

        noise.start();
        wind.start();
        rumble.start();
        whine.start();

        this.engFilter = engFilter;
        this.engGain = engGain;
        this.windFilter = windFilter;
        this.windGain = windGain;
        this.rumble = rumble;
        this.rumbleGain = rumbleGain;
        this.whine = whine;
        this.whineGain = whineGain;
        this.ready = true;
      } catch (err) {
        console.warn("Audio unavailable", err);
      }
    }

    update(a, active) {
      if (!this.ready) return;
      const ctx = this.ctx;
      const now = ctx.currentTime;
      const throttle = active ? a.throttle : 0;
      const speed = active ? a.speed : 0;
      const ab = active ? a.afterburnerVisual : 0;
      const eng = clamp(0.025 + throttle * 0.07 + ab * 0.075, 0, 0.2);
      const wind = clamp(speed / 330 * 0.055, 0, 0.07);
      this.engGain.gain.setTargetAtTime(eng, now, 0.08);
      this.engFilter.frequency.setTargetAtTime(280 + speed * 5 + throttle * 600 + ab * 800, now, 0.08);
      this.windGain.gain.setTargetAtTime(wind, now, 0.12);
      this.windFilter.frequency.setTargetAtTime(500 + speed * 14, now, 0.1);
      this.rumbleGain.gain.setTargetAtTime(active ? 0.008 + throttle * 0.012 + ab * 0.015 : 0, now, 0.1);
      this.whineGain.gain.setTargetAtTime(active ? 0.0015 + throttle * 0.003 : 0, now, 0.1);
      this.whine.frequency.setTargetAtTime(620 + throttle * 560 + ab * 250, now, 0.1);
      this.rumble.frequency.setTargetAtTime(39 + throttle * 22 + ab * 15, now, 0.1);
    }

    tone(freq, duration, volume = 0.04, type = "square", delay = 0) {
      if (!this.ctx || !this.ready) return;
      const ctx = this.ctx;
      const t0 = ctx.currentTime + delay;
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = type;
      osc.frequency.setValueAtTime(freq, t0);
      gain.gain.setValueAtTime(0, t0);
      gain.gain.linearRampToValueAtTime(volume, t0 + 0.012);
      gain.gain.exponentialRampToValueAtTime(0.0001, t0 + duration);
      osc.connect(gain).connect(this.master);
      osc.start(t0);
      osc.stop(t0 + duration + 0.03);
    }

    gate() {
      this.tone(660, 0.1, 0.045, "square");
      this.tone(990, 0.17, 0.04, "square", 0.09);
    }

    warn(kind) {
      if (!this.ctx) return;
      if (kind === "pullup") {
        this.tone(640, 0.12, 0.05, "sawtooth");
        this.tone(520, 0.12, 0.045, "sawtooth", 0.15);
      } else {
        this.tone(410, 0.15, 0.035, "square");
      }
    }

    crash() {
      if (!this.ready) return;
      const ctx = this.ctx;
      const now = ctx.currentTime;
      const buffer = ctx.createBuffer(1, ctx.sampleRate * 1.4, ctx.sampleRate);
      const data = buffer.getChannelData(0);
      for (let i = 0; i < data.length; i++) {
        const t = i / data.length;
        data[i] = (Math.random() * 2 - 1) * Math.pow(1 - t, 2.2) * 0.8;
      }
      const src = ctx.createBufferSource();
      src.buffer = buffer;
      const filter = ctx.createBiquadFilter();
      filter.type = "lowpass";
      filter.frequency.setValueAtTime(2800, now);
      filter.frequency.exponentialRampToValueAtTime(120, now + 1.1);
      const gain = ctx.createGain();
      gain.gain.value = 0.5;
      src.connect(filter).connect(gain).connect(this.master);
      src.start(now);
    }

    toggleMute() {
      this.muted = !this.muted;
      if (this.master && this.ctx) {
        this.master.gain.setTargetAtTime(this.muted ? 0 : 0.5, this.ctx.currentTime, 0.03);
      }
      return this.muted;
    }
  }

  // ------------------------------------------------------------------ GLSL kit
  const GLSL_NOISE = `
    float hash21(vec2 p) {
      p = fract(p * vec2(123.34, 456.21));
      p += dot(p, p + 45.32);
      return fract(p.x * p.y);
    }
    float vnoise(vec2 p) {
      vec2 i = floor(p);
      vec2 f = fract(p);
      f = f * f * (3.0 - 2.0 * f);
      float a = hash21(i);
      float b = hash21(i + vec2(1.0, 0.0));
      float c = hash21(i + vec2(0.0, 1.0));
      float d = hash21(i + vec2(1.0, 1.0));
      return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
    }
    float fbm5(vec2 p) {
      float v = 0.0;
      float a = 0.5;
      for (int i = 0; i < 5; i++) {
        v += a * vnoise(p);
        p = p * 2.03 + vec2(17.3, 9.2);
        a *= 0.5;
      }
      return v;
    }
    float riverX(float z) {
      return 1500.0 * sin(z / 5200.0) + 420.0 * sin(z / 1900.0 + 0.7);
    }
    float terrainH(vec2 p) {
      float wx = (fbm5(p / 17000.0 + vec2(4.2, 8.1)) - 0.5) * 1800.0;
      float wz = (fbm5(p / 14000.0 + vec2(-3.3, 5.8)) - 0.5) * 1400.0;
      vec2 q = p + vec2(wx, wz);
      float base = fbm5(q / 13000.0 + vec2(2.0, 9.4));
      float ridgeN = fbm5(q / 4600.0 + vec2(1.7, -3.2));
      float ridge = 1.0 - abs(ridgeN * 2.0 - 1.0);
      float h = -380.0 + base * 1050.0 + pow(ridge, 2.3) * 2550.0;
      h += (fbm5(p / 1000.0 + vec2(8.0, 3.0)) - 0.5) * 110.0;
      h += (fbm5(p / 300.0 + vec2(3.0, 7.0)) - 0.5) * 22.0;
      float coast = smoothstep(-13500.0, -9000.0, p.y);
      float sea = -320.0 + (fbm5(p / 5000.0) - 0.5) * 80.0;
      h = mix(sea, h, coast);
      float d = abs(p.x - riverX(p.y));
      float riverN = fbm5(p / 1300.0 + vec2(5.5, 1.7));
      float floorY = 300.0 + riverN * 130.0 + 0.012 * (p.y + 3000.0);
      float wall = smoothstep(420.0, 1650.0, d);
      h = mix(floorY, h, wall);
      return h;
    }
  `;

  const VS_FULL = `#version 300 es
    out vec2 vUv;
    void main() {
      vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
      vUv = p;
      gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
    }
  `;

  const FS_SKY = `#version 300 es
    precision highp float;
    in vec2 vUv;
    uniform vec3 u_camPos;
    uniform vec3 u_camRight;
    uniform vec3 u_camUp;
    uniform vec3 u_camForward;
    uniform vec3 u_sunDir;
    uniform float u_tanHalfFov;
    uniform float u_aspect;
    uniform float u_time;
    out vec4 outColor;
    ${GLSL_NOISE}
    void main() {
      vec2 ndc = vUv * 2.0 - 1.0;
      vec3 dir = normalize(u_camForward
        + u_camRight * ndc.x * u_tanHalfFov * u_aspect
        + u_camUp * ndc.y * u_tanHalfFov);
      float y = dir.y;
      float h = pow(clamp(y, 0.0, 1.0), 0.58);
      vec3 horizon = vec3(0.71, 0.65, 0.52);
      vec3 zenith = vec3(0.10, 0.26, 0.43);
      vec3 sky = mix(horizon, zenith, h);
      float sun = clamp(dot(dir, u_sunDir), 0.0, 1.0);
      sky += vec3(1.0, 0.53, 0.21) * pow(sun, 5.0) * 0.11;
      sky += vec3(1.0, 0.76, 0.43) * pow(sun, 44.0) * 0.35;
      sky += vec3(1.0, 0.96, 0.80) * smoothstep(0.9992, 0.9997, sun) * 5.0;

      if (y < 0.0) {
        vec3 under = mix(vec3(0.62, 0.59, 0.48), vec3(0.12, 0.18, 0.17), clamp(-y * 7.0, 0.0, 1.0));
        sky = mix(horizon, under, clamp(-y * 18.0, 0.15, 1.0));
      }

      if (y > 0.008) {
        float t = clamp((3600.0 - u_camPos.y) / y, 0.0, 60000.0);
        vec2 cp = u_camPos.xz + dir.xz * t;
        float n1 = fbm5(cp * 0.00038 + vec2(u_time * 0.0009, 0.0));
        float n2 = fbm5(cp * 0.00105 - vec2(u_time * 0.0014, u_time * 0.0004));
        float cover = smoothstep(0.54, 0.72, n1 + n2 * 0.2);
        float fade = smoothstep(0.0, 0.10, y) * smoothstep(60000.0, 9000.0, t);
        vec3 cloudCol = mix(vec3(0.67, 0.70, 0.66), vec3(1.0, 0.93, 0.77), pow(sun, 2.0));
        sky = mix(sky, cloudCol, cover * fade * 0.5);
      }

      float horizonHaze = 1.0 - smoothstep(0.0, 0.13, abs(y - 0.025));
      sky = mix(sky, vec3(0.70, 0.66, 0.53), horizonHaze * 0.15);
      outColor = vec4(sky, 1.0);
    }
  `;

  const VS_TERRAIN = `#version 300 es
    precision highp float;
    in vec2 aPos;
    uniform mat4 u_vp;
    uniform vec2 u_center;
    out vec3 vWorld;
    ${GLSL_NOISE}
    void main() {
      vec2 p = u_center + aPos;
      float h = terrainH(p);
      vWorld = vec3(p.x, h, p.y);
      gl_Position = u_vp * vec4(vWorld, 1.0);
    }
  `;

  const FS_TERRAIN = `#version 300 es
    precision highp float;
    in vec3 vWorld;
    uniform vec3 u_camPos;
    uniform vec3 u_sunDir;
    uniform vec3 u_fogColor;
    out vec4 outColor;
    ${GLSL_NOISE}
    void main() {
      vec3 n = normalize(cross(dFdx(vWorld), dFdy(vWorld)));
      if (n.y < 0.0) n = -n;
      float h = vWorld.y;
      float slope = 1.0 - n.y;
      float grain = vnoise(vWorld.xz * 0.031) * 0.55 + vnoise(vWorld.xz * 0.29) * 0.3;

      vec3 low = vec3(0.235, 0.275, 0.145);
      vec3 scrub = vec3(0.32, 0.32, 0.16);
      vec3 rock = vec3(0.33, 0.30, 0.25);
      vec3 highRock = vec3(0.43, 0.41, 0.38);
      vec3 snow = vec3(0.79, 0.80, 0.76);
      vec3 sand = vec3(0.48, 0.40, 0.25);

      vec3 col = low;
      col = mix(col, sand, smoothstep(25.0, 8.0, h));
      col = mix(col, scrub, smoothstep(360.0, 760.0, h));
      col = mix(col, rock, smoothstep(800.0, 1450.0, h));
      col = mix(col, highRock, smoothstep(1650.0, 2300.0, h));
      float snowMask = smoothstep(2050.0, 2450.0, h) * (1.0 - smoothstep(0.42, 0.68, slope));
      col = mix(col, snow, snowMask);
      col = mix(col, rock, smoothstep(0.23, 0.52, slope) * 0.72);
      col *= 0.82 + grain * 0.36;

      float sunLight = max(dot(n, u_sunDir), 0.0);
      float skyLight = 0.33 + 0.34 * (0.5 + 0.5 * n.y);
      vec3 lit = col * (skyLight + sunLight * 0.92);
      lit += vec3(0.10, 0.09, 0.06) * pow(1.0 - n.y, 2.0);
      lit = mix(lit, vec3(0.12, 0.16, 0.13), smoothstep(0.70, 1.0, slope) * 0.22);

      vec3 view = vWorld - u_camPos;
      float dist = length(view);
      float fog = 1.0 - exp(-dist * 0.000058);
      fog = clamp(fog + smoothstep(18000.0, 32000.0, dist) * 0.5, 0.0, 1.0);
      vec3 fogCol = mix(u_fogColor, vec3(0.58, 0.62, 0.57), smoothstep(3000.0, 0.0, vWorld.y) * 0.25);
      lit = mix(lit, fogCol, fog);
      outColor = vec4(lit, 1.0);
    }
  `;

  const VS_OCEAN = `#version 300 es
    precision highp float;
    in vec2 aPos;
    uniform mat4 u_vp;
    uniform vec2 u_center;
    out vec3 vWorld;
    void main() {
      vec2 p = u_center + aPos;
      vWorld = vec3(p.x, 0.0, p.y);
      gl_Position = u_vp * vec4(vWorld, 1.0);
    }
  `;

  const FS_OCEAN = `#version 300 es
    precision highp float;
    in vec3 vWorld;
    uniform vec3 u_camPos;
    uniform vec3 u_sunDir;
    uniform vec3 u_fogColor;
    uniform float u_time;
    out vec4 outColor;
    void main() {
      vec2 p = vWorld.xz;
      float t = u_time;
      float dx = cos(p.x * 0.075 + p.y * 0.043 + t * 1.15) * 0.055
               + cos(p.x * 0.021 - p.y * 0.034 + t * 0.62) * 0.05;
      float dz = sin(p.y * 0.068 + p.x * 0.047 - t * 1.02) * 0.05
               + sin(p.y * 0.019 + p.x * 0.028 + t * 0.71) * 0.05;
      vec3 n = normalize(vec3(-dx, 1.0, -dz));
      vec3 view = normalize(vWorld - u_camPos);
      float fresnel = pow(1.0 - clamp(dot(n, -view), 0.0, 1.0), 3.0);
      float sun = pow(max(dot(reflect(view, n), u_sunDir), 0.0), 110.0);
      vec3 deep = vec3(0.018, 0.095, 0.115);
      vec3 shallow = vec3(0.055, 0.24, 0.25);
      vec3 col = mix(deep, shallow, fresnel * 0.72 + 0.08);
      col += vec3(1.0, 0.84, 0.54) * sun * 1.55;
      col += vec3(0.13, 0.22, 0.19) * max(dot(n, u_sunDir), 0.0) * 0.18;
      float dist = length(vWorld - u_camPos);
      float fog = clamp(1.0 - exp(-dist * 0.000052), 0.0, 1.0);
      col = mix(col, u_fogColor, fog);
      outColor = vec4(col, 1.0);
    }
  `;

  const VS_CLOUD = `#version 300 es
    precision highp float;
    in vec3 aCenter;
    in vec2 aCorner;
    in float aSize;
    in float aAlpha;
    uniform mat4 u_vp;
    uniform vec3 u_camRight;
    uniform vec3 u_camUp;
    out vec2 vUv;
    out float vAlpha;
    out vec3 vWorld;
    void main() {
      vec3 world = aCenter + u_camRight * aCorner.x * aSize + u_camUp * aCorner.y * aSize * 0.5;
      vWorld = world;
      vUv = aCorner * 0.5 + 0.5;
      vAlpha = aAlpha;
      gl_Position = u_vp * vec4(world, 1.0);
    }
  `;

  const FS_CLOUD = `#version 300 es
    precision highp float;
    in vec2 vUv;
    in float vAlpha;
    in vec3 vWorld;
    uniform sampler2D u_tex;
    uniform vec3 u_camPos;
    uniform vec3 u_sunDir;
    uniform vec3 u_fogColor;
    out vec4 outColor;
    void main() {
      float a = texture(u_tex, vUv).a * vAlpha;
      if (a < 0.008) discard;
      float dist = length(vWorld - u_camPos);
      float fog = clamp(1.0 - exp(-dist * 0.000048), 0.0, 0.95);
      vec3 col = mix(vec3(0.87, 0.86, 0.78), vec3(1.0, 0.92, 0.73), 0.35);
      col = mix(col, u_fogColor, fog * 0.9);
      outColor = vec4(col, a * (1.0 - fog * 0.78));
    }
  `;

  const VS_GATE = `#version 300 es
    precision highp float;
    in vec3 aPos;
    in vec3 aNormal;
    in float aU;
    uniform mat4 u_vp;
    uniform mat4 u_model;
    uniform float u_inflate;
    out vec3 vWorld;
    out vec3 vNormal;
    out float vU;
    void main() {
      vec3 local = aPos + aNormal * u_inflate;
      vec4 world = u_model * vec4(local, 1.0);
      vWorld = world.xyz;
      vNormal = mat3(u_model) * aNormal;
      vU = aU;
      gl_Position = u_vp * world;
    }
  `;

  const FS_GATE = `#version 300 es
    precision highp float;
    in vec3 vWorld;
    in vec3 vNormal;
    in float vU;
    uniform vec3 u_camPos;
    uniform vec3 u_color;
    uniform vec3 u_fogColor;
    uniform float u_time;
    uniform float u_glow;
    out vec4 outColor;
    void main() {
      vec3 n = normalize(vNormal);
      float band = 0.70 + 0.30 * step(0.48, fract(vU * 12.0));
      float pulse = 0.80 + 0.20 * sin(u_time * 3.4 + vU * 18.0);
      vec3 col = u_color * (0.70 + 0.6 * max(dot(n, normalize(vec3(0.4, 0.6, 0.3))), 0.0));
      col *= band * pulse;
      float dist = length(vWorld - u_camPos);
      float fog = clamp(1.0 - exp(-dist * 0.000045), 0.0, 0.92);
      col = mix(col, u_fogColor, fog);
      if (u_glow > 0.5) {
        col *= 1.4;
        outColor = vec4(col, 0.13);
      } else {
        outColor = vec4(col, 1.0);
      }
    }
  `;

  const FS_POST = `#version 300 es
    precision highp float;
    in vec2 vUv;
    uniform sampler2D u_tex;
    uniform vec2 u_resolution;
    out vec4 outColor;
    float h21(vec2 p) {
      p = fract(p * vec2(123.34, 456.21));
      p += dot(p, p + 45.32);
      return fract(p.x * p.y);
    }
    void main() {
      vec2 cc = vUv - 0.5;
      float r2 = dot(cc, cc);
      vec2 uv = 0.5 + cc * (1.0 + r2 * 0.018);
      vec2 chroma = cc * 0.00045;
      vec3 col;
      col.r = texture(u_tex, uv + chroma).r;
      col.g = texture(u_tex, uv).g;
      col.b = texture(u_tex, uv - chroma).b;

      float scan = 0.955 + 0.045 * sin(gl_FragCoord.y * 2.0943951);
      col *= mix(1.0, scan, 0.55);
      float vign = smoothstep(0.98, 0.26, length(uv - 0.5));
      col *= mix(0.68, 1.0, vign);
      float grain = h21(gl_FragCoord.xy + floor(u_resolution.x * 0.713));
      col += (grain - 0.5) * 0.022;
      col *= vec3(1.025, 1.0, 0.955);
      col = clamp(col, 0.0, 1.0);
      outColor = vec4(col, 1.0);
    }
  `;

  // ------------------------------------------------------------------ renderer
  class Renderer {
    constructor(canvas) {
      this.canvas = canvas;
      const gl = canvas.getContext("webgl2", {
        antialias: false,
        alpha: false,
        depth: true,
        powerPreference: "high-performance",
        preserveDrawingBuffer: false,
      });
      if (!gl) throw new Error("WebGL 2 is not available in this browser.");
      this.gl = gl;
      this.dpr = 1;
      this.cssW = 0;
      this.cssH = 0;
      this.renderW = 0;
      this.renderH = 0;
      this.time = 0;

      this.programs = {
        sky: this.program(VS_FULL, FS_SKY, "sky"),
        terrain: this.program(VS_TERRAIN, FS_TERRAIN, "terrain"),
        ocean: this.program(VS_OCEAN, FS_OCEAN, "ocean"),
        cloud: this.program(VS_CLOUD, FS_CLOUD, "cloud"),
        gate: this.program(VS_GATE, FS_GATE, "gate"),
        post: this.program(VS_FULL, FS_POST, "post"),
      };
      this.uniforms = {};
      for (const [name, p] of Object.entries(this.programs)) {
        this.uniforms[name] = this.uniformMap(p.program);
      }

      this.createTerrain();
      this.createOcean();
      this.createGate();
      this.createClouds();
      this.createPost();

      gl.disable(gl.CULL_FACE);
      gl.clearColor(0.1, 0.14, 0.16, 1);
      this.sunDir = normalize([0.52, 0.34, -0.78]);
      this.fogColor = [0.67, 0.64, 0.53];
    }

    program(vsSource, fsSource, label) {
      const gl = this.gl;
      const compile = (type, source) => {
        const shader = gl.createShader(type);
        gl.shaderSource(shader, source);
        gl.compileShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
          const info = gl.getShaderInfoLog(shader);
          throw new Error(`${label} shader failed:\n${info}`);
        }
        return shader;
      };
      const p = gl.createProgram();
      gl.attachShader(p, compile(gl.VERTEX_SHADER, vsSource));
      gl.attachShader(p, compile(gl.FRAGMENT_SHADER, fsSource));
      gl.linkProgram(p);
      if (!gl.getProgramParameter(p, gl.LINK_STATUS)) {
        throw new Error(`${label} program failed:\n${gl.getProgramInfoLog(p)}`);
      }
      return { program: p };
    }

    uniformMap(program) {
      const gl = this.gl;
      const map = new Map();
      const count = gl.getProgramParameter(program, gl.ACTIVE_UNIFORMS);
      for (let i = 0; i < count; i++) {
        const info = gl.getActiveUniform(program, i);
        if (!info) continue;
        const name = info.name.replace(/\[0\]$/, "");
        map.set(name, gl.getUniformLocation(program, info.name));
      }
      return map;
    }

    buffer(target, data, usage = this.gl.STATIC_DRAW) {
      const gl = this.gl;
      const buf = gl.createBuffer();
      gl.bindBuffer(target, buf);
      gl.bufferData(target, data, usage);
      return buf;
    }

    createTerrain() {
      const gl = this.gl;
      const rings = 180;
      const segments = 192;
      const positions = new Float32Array(rings * segments * 2);
      const radii = [];
      let p = 0;
      for (let i = 0; i < rings; i++) {
        const t = i / (rings - 1);
        const r = 20 * Math.pow(1400, t);
        radii.push(r);
        for (let j = 0; j < segments; j++) {
          const a = (j / segments) * TAU;
          positions[p++] = Math.cos(a) * r;
          positions[p++] = Math.sin(a) * r;
        }
      }
      // Rotate the first ring slightly so its spokes never line up with the
      // broadside direction for too long.
      const indices = new Uint32Array((rings - 1) * segments * 6);
      let k = 0;
      for (let i = 0; i < rings - 1; i++) {
        for (let j = 0; j < segments; j++) {
          const a = i * segments + j;
          const b = i * segments + ((j + 1) % segments);
          const c = (i + 1) * segments + j;
          const d = (i + 1) * segments + ((j + 1) % segments);
          indices[k++] = a; indices[k++] = c; indices[k++] = b;
          indices[k++] = b; indices[k++] = c; indices[k++] = d;
        }
      }
      this.terrainVAO = gl.createVertexArray();
      gl.bindVertexArray(this.terrainVAO);
      this.terrainPos = this.buffer(gl.ARRAY_BUFFER, positions);
      const loc = gl.getAttribLocation(this.programs.terrain.program, "aPos");
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
      this.terrainIndex = this.buffer(gl.ELEMENT_ARRAY_BUFFER, indices);
      this.terrainCount = indices.length;
      gl.bindVertexArray(null);
    }

    createOcean() {
      const gl = this.gl;
      const segments = 192;
      const pos = new Float32Array((segments + 1) * 2);
      for (let i = 0; i < segments; i++) {
        const a = (i / segments) * TAU;
        pos[(i + 1) * 2] = Math.cos(a) * 65000;
        pos[(i + 1) * 2 + 1] = Math.sin(a) * 65000;
      }
      const idx = new Uint32Array(segments * 3);
      for (let i = 0; i < segments; i++) {
        idx[i * 3] = 0;
        idx[i * 3 + 1] = 1 + i;
        idx[i * 3 + 2] = 1 + ((i + 1) % segments);
      }
      this.oceanVAO = gl.createVertexArray();
      gl.bindVertexArray(this.oceanVAO);
      this.buffer(gl.ARRAY_BUFFER, pos);
      const loc = gl.getAttribLocation(this.programs.ocean.program, "aPos");
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
      this.buffer(gl.ELEMENT_ARRAY_BUFFER, idx);
      this.oceanCount = idx.length;
      gl.bindVertexArray(null);
    }

    createGate() {
      const gl = this.gl;
      const major = 48;
      const tube = 10;
      const R = GATE_RADIUS;
      const r = 3.2;
      const verts = [];
      const indices = [];
      for (let i = 0; i <= major; i++) {
        const u = (i / major) * TAU;
        for (let j = 0; j <= tube; j++) {
          const v = (j / tube) * TAU;
          const rr = R + r * Math.cos(v);
          const x = Math.cos(u) * rr;
          const y = Math.sin(u) * rr;
          const z = r * Math.sin(v);
          const nx = Math.cos(u) * Math.cos(v);
          const ny = Math.sin(u) * Math.cos(v);
          const nz = Math.sin(v);
          verts.push(x, y, z, nx, ny, nz, i / major);
        }
      }
      for (let i = 0; i < major; i++) {
        for (let j = 0; j < tube; j++) {
          const a = i * (tube + 1) + j;
          const b = a + tube + 1;
          indices.push(a, b, a + 1, a + 1, b, b + 1);
        }
      }
      const vao = gl.createVertexArray();
      gl.bindVertexArray(vao);
      const vbo = this.buffer(gl.ARRAY_BUFFER, new Float32Array(verts));
      const stride = 7 * 4;
      const aPos = gl.getAttribLocation(this.programs.gate.program, "aPos");
      const aNormal = gl.getAttribLocation(this.programs.gate.program, "aNormal");
      const aU = gl.getAttribLocation(this.programs.gate.program, "aU");
      gl.enableVertexAttribArray(aPos);
      gl.vertexAttribPointer(aPos, 3, gl.FLOAT, false, stride, 0);
      gl.enableVertexAttribArray(aNormal);
      gl.vertexAttribPointer(aNormal, 3, gl.FLOAT, false, stride, 12);
      gl.enableVertexAttribArray(aU);
      gl.vertexAttribPointer(aU, 1, gl.FLOAT, false, stride, 24);
      this.buffer(gl.ELEMENT_ARRAY_BUFFER, new Uint16Array(indices));
      this.gateVAO = vao;
      this.gateCount = indices.length;
      gl.bindVertexArray(null);
    }

    cloudTexture() {
      const c = document.createElement("canvas");
      c.width = 256;
      c.height = 128;
      const ctx = c.getContext("2d");
      ctx.clearRect(0, 0, c.width, c.height);
      let seed = 9127;
      const rnd = () => {
        seed = (seed * 1664525 + 1013904223) >>> 0;
        return seed / 4294967296;
      };
      ctx.filter = "blur(3px)";
      for (let i = 0; i < 34; i++) {
        const x = 35 + rnd() * 186;
        const spread = 1 - Math.abs(x - 128) / 150;
        const y = 68 + (rnd() - 0.5) * 38 * spread;
        const r = 18 + rnd() * 32 * (0.6 + spread);
        const grad = ctx.createRadialGradient(x, y, 0, x, y, r);
        grad.addColorStop(0, `rgba(255,255,255,${0.24 + rnd() * 0.22})`);
        grad.addColorStop(0.55, "rgba(255,255,255,0.10)");
        grad.addColorStop(1, "rgba(255,255,255,0)");
        ctx.fillStyle = grad;
        ctx.beginPath();
        ctx.arc(x, y, r, 0, TAU);
        ctx.fill();
      }
      return c;
    }

    createClouds() {
      const gl = this.gl;
      let seed = 4317;
      const rnd = () => {
        seed = (seed * 1664525 + 1013904223) >>> 0;
        return seed / 4294967296;
      };
      const clouds = [];
      for (let i = 0; i < 58; i++) {
        const x = (rnd() * 2 - 1) * 19000;
        const z = 13500 - rnd() * 29000;
        const ground = terrainAt(x, z);
        const y = Math.max(ground + 420, 1250) + rnd() * 1600;
        const size = 1000 + rnd() * 2100;
        const alpha = 0.32 + rnd() * 0.34;
        clouds.push({ pos: [x, y, z], size, alpha });
      }
      this.clouds = clouds;
      this.cloudData = new Float32Array(clouds.length * 6 * 7);
      this.cloudVAO = gl.createVertexArray();
      gl.bindVertexArray(this.cloudVAO);
      const vbo = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
      gl.bufferData(gl.ARRAY_BUFFER, this.cloudData.byteLength, gl.DYNAMIC_DRAW);
      const stride = 7 * 4;
      const aCenter = gl.getAttribLocation(this.programs.cloud.program, "aCenter");
      const aCorner = gl.getAttribLocation(this.programs.cloud.program, "aCorner");
      const aSize = gl.getAttribLocation(this.programs.cloud.program, "aSize");
      const aAlpha = gl.getAttribLocation(this.programs.cloud.program, "aAlpha");
      gl.enableVertexAttribArray(aCenter);
      gl.vertexAttribPointer(aCenter, 3, gl.FLOAT, false, stride, 0);
      gl.enableVertexAttribArray(aCorner);
      gl.vertexAttribPointer(aCorner, 2, gl.FLOAT, false, stride, 12);
      gl.enableVertexAttribArray(aSize);
      gl.vertexAttribPointer(aSize, 1, gl.FLOAT, false, stride, 20);
      gl.enableVertexAttribArray(aAlpha);
      gl.vertexAttribPointer(aAlpha, 1, gl.FLOAT, false, stride, 24);
      gl.bindVertexArray(null);
      this.cloudVBO = vbo;

      const tex = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, this.cloudTexture());
      gl.generateMipmap(gl.TEXTURE_2D);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      this.cloudTex = tex;
    }

    createPost() {
      const gl = this.gl;
      this.postVAO = gl.createVertexArray();
      this.fbo = gl.createFramebuffer();
      this.fboTexture = gl.createTexture();
      this.fboDepth = gl.createRenderbuffer();
      gl.bindTexture(gl.TEXTURE_2D, this.fboTexture);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbo);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, this.fboTexture, 0);
      gl.bindRenderbuffer(gl.RENDERBUFFER, this.fboDepth);
      gl.renderbufferStorage(gl.RENDERBUFFER, gl.DEPTH_COMPONENT24, 1, 1);
      gl.framebufferRenderbuffer(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.RENDERBUFFER, this.fboDepth);
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    }

    resize(cssW, cssH) {
      const gl = this.gl;
      this.cssW = cssW;
      this.cssH = cssH;
      const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
      this.dpr = dpr;
      const cw = Math.max(1, Math.floor(cssW * dpr));
      const ch = Math.max(1, Math.floor(cssH * dpr));
      this.canvas.width = cw;
      this.canvas.height = ch;
      this.canvas.style.width = cssW + "px";
      this.canvas.style.height = cssH + "px";
      let targetW = Math.round(clamp(cssW * 0.58, 560, 1240));
      let targetH = Math.max(1, Math.round(targetW * (cssH / cssW)));
      if (targetH > 900) {
        targetH = 900;
        targetW = Math.max(1, Math.round(targetH * (cssW / cssH)));
      }
      this.renderW = targetW;
      this.renderH = targetH;
      gl.bindTexture(gl.TEXTURE_2D, this.fboTexture);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, this.renderW, this.renderH, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      gl.bindRenderbuffer(gl.RENDERBUFFER, this.fboDepth);
      gl.renderbufferStorage(gl.RENDERBUFFER, gl.DEPTH_COMPONENT24, this.renderW, this.renderH);
      gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbo);
      gl.checkFramebufferStatus(gl.FRAMEBUFFER);
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    }

    use(p) {
      const gl = this.gl;
      gl.useProgram(p.program);
      return this.uniforms[p === this.programs.sky ? "sky" :
        p === this.programs.terrain ? "terrain" :
        p === this.programs.ocean ? "ocean" :
        p === this.programs.cloud ? "cloud" :
        p === this.programs.gate ? "gate" : "post"];
    }

    u(map, name, value, type) {
      const gl = this.gl;
      const loc = map.get(name);
      if (loc == null) return;
      if (type === "m4") gl.uniformMatrix4fv(loc, false, value);
      else if (type === "v3") gl.uniform3fv(loc, value);
      else if (type === "v2") gl.uniform2fv(loc, value);
      else if (type === "f") gl.uniform1f(loc, value);
      else if (type === "i") gl.uniform1i(loc, value);
      else gl.uniform1f(loc, value);
    }

    updateCloudBuffer(cam) {
      const gl = this.gl;
      const sorted = this.clouds
        .map((c) => {
          const d = (c.pos[0] - cam.eye[0]) ** 2 + (c.pos[1] - cam.eye[1]) ** 2 + (c.pos[2] - cam.eye[2]) ** 2;
          return { c, d };
        })
        .sort((a, b) => b.d - a.d);
      const arr = this.cloudData;
      let o = 0;
      const corners = [[-1, -1], [1, -1], [-1, 1], [-1, 1], [1, -1], [1, 1]];
      for (const { c } of sorted) {
        for (const corner of corners) {
          arr[o++] = c.pos[0]; arr[o++] = c.pos[1]; arr[o++] = c.pos[2];
          arr[o++] = corner[0]; arr[o++] = corner[1];
          arr[o++] = c.size; arr[o++] = c.alpha;
        }
      }
      gl.bindBuffer(gl.ARRAY_BUFFER, this.cloudVBO);
      gl.bufferSubData(gl.ARRAY_BUFFER, 0, arr);
    }

    render(cam, sim) {
      const gl = this.gl;
      this.time = sim.time;
      const vp = mat4Mul(
        mat4Perspective(cam.fov, this.cssW / this.cssH, 0.7, 70000),
        mat4LookAt(cam.eye, add(cam.eye, cam.forward), cam.up)
      );
      const tanHalf = Math.tan(cam.fov / 2);
      const center = [cam.eye[0], 0, cam.eye[2]];

      // --- world target -------------------------------------------------------
      gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbo);
      gl.viewport(0, 0, this.renderW, this.renderH);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      gl.enable(gl.DEPTH_TEST);
      gl.depthFunc(gl.LEQUAL);
      gl.depthMask(true);
      gl.disable(gl.BLEND);

      // sky
      let p = this.programs.sky;
      let u = this.use(p);
      gl.depthMask(false);
      gl.disable(gl.DEPTH_TEST);
      this.u(u, "u_camPos", cam.eye, "v3");
      this.u(u, "u_camRight", cam.right, "v3");
      this.u(u, "u_camUp", cam.up, "v3");
      this.u(u, "u_camForward", cam.forward, "v3");
      this.u(u, "u_sunDir", this.sunDir, "v3");
      this.u(u, "u_tanHalfFov", tanHalf, "f");
      this.u(u, "u_aspect", this.cssW / this.cssH, "f");
      this.u(u, "u_time", sim.time, "f");
      gl.bindVertexArray(this.postVAO);
      gl.drawArrays(gl.TRIANGLES, 0, 3);

      // terrain
      gl.enable(gl.DEPTH_TEST);
      gl.depthMask(true);
      p = this.programs.terrain;
      u = this.use(p);
      this.u(u, "u_vp", vp, "m4");
      this.u(u, "u_center", [cam.eye[0], cam.eye[2]], "v2");
      this.u(u, "u_camPos", cam.eye, "v3");
      this.u(u, "u_sunDir", this.sunDir, "v3");
      this.u(u, "u_fogColor", this.fogColor, "v3");
      gl.bindVertexArray(this.terrainVAO);
      gl.drawElements(gl.TRIANGLES, this.terrainCount, gl.UNSIGNED_INT, 0);

      // ocean
      p = this.programs.ocean;
      u = this.use(p);
      this.u(u, "u_vp", vp, "m4");
      this.u(u, "u_center", [cam.eye[0], cam.eye[2]], "v2");
      this.u(u, "u_camPos", cam.eye, "v3");
      this.u(u, "u_sunDir", this.sunDir, "v3");
      this.u(u, "u_fogColor", this.fogColor, "v3");
      this.u(u, "u_time", sim.time, "f");
      gl.bindVertexArray(this.oceanVAO);
      gl.drawElements(gl.TRIANGLES, this.oceanCount, gl.UNSIGNED_INT, 0);

      // clouds
      this.updateCloudBuffer(cam);
      p = this.programs.cloud;
      u = this.use(p);
      this.u(u, "u_vp", vp, "m4");
      this.u(u, "u_camRight", cam.right, "v3");
      this.u(u, "u_camUp", cam.up, "v3");
      this.u(u, "u_camPos", cam.eye, "v3");
      this.u(u, "u_sunDir", this.sunDir, "v3");
      this.u(u, "u_fogColor", this.fogColor, "v3");
      this.u(u, "u_tex", 0, "i");
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, this.cloudTex);
      gl.enable(gl.BLEND);
      gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
      gl.depthMask(false);
      gl.bindVertexArray(this.cloudVAO);
      gl.drawArrays(gl.TRIANGLES, 0, this.clouds.length * 6);
      gl.depthMask(true);
      gl.disable(gl.BLEND);

      // gates
      p = this.programs.gate;
      u = this.use(p);
      this.u(u, "u_vp", vp, "m4");
      this.u(u, "u_camPos", cam.eye, "v3");
      this.u(u, "u_fogColor", this.fogColor, "v3");
      this.u(u, "u_time", sim.time, "f");
      gl.bindVertexArray(this.gateVAO);
      const gates = sim.mission.gates;
      for (let i = 0; i < gates.length; i++) {
        const gate = gates[i];
        const dist = len(sub(gate.pos, cam.eye));
        if (dist > 24000) continue;
        const ax = normalize(gate.axis);
        const right = normalize(cross(ax, [0, 1, 0]));
        const up = cross(right, ax);
        const model = mat4Basis(right, up, ax, gate.pos);
        const isNext = i === sim.mission.gateIndex;
        const passed = gate.passed;
        let color;
        if (passed) color = [0.18, 0.5, 0.26];
        else if (isNext) color = [1.0, 0.66, 0.22];
        else color = [0.30, 0.79, 0.74];
        this.u(u, "u_model", model, "m4");
        this.u(u, "u_color", color, "v3");
        this.u(u, "u_inflate", 0, "f");
        this.u(u, "u_glow", 0, "f");
        gl.drawElements(gl.TRIANGLES, this.gateCount, gl.UNSIGNED_SHORT, 0);

        if (isNext || !passed) {
          gl.enable(gl.BLEND);
          gl.blendFunc(gl.SRC_ALPHA, gl.ONE);
          gl.depthMask(false);
          this.u(u, "u_inflate", 4.0, "f");
          this.u(u, "u_glow", 1, "f");
          gl.drawElements(gl.TRIANGLES, this.gateCount, gl.UNSIGNED_SHORT, 0);
          this.u(u, "u_glow", 0, "f");
          this.u(u, "u_inflate", 0, "f");
          gl.depthMask(true);
          gl.disable(gl.BLEND);
        }
      }
      gl.bindVertexArray(null);

      // --- post --------------------------------------------------------------
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);
      gl.viewport(0, 0, this.canvas.width, this.canvas.height);
      gl.disable(gl.DEPTH_TEST);
      gl.depthMask(true);
      gl.disable(gl.BLEND);
      p = this.programs.post;
      u = this.use(p);
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, this.fboTexture);
      this.u(u, "u_tex", 0, "i");
      this.u(u, "u_resolution", [this.canvas.width, this.canvas.height], "v2");
      gl.bindVertexArray(this.postVAO);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
      gl.bindVertexArray(null);
    }
  }

  // Export a tiny seam for automated smoke tests / curious pilots.
  window.__VIPER_CORE = {
    terrainAt,
    riverX,
    buildMission,
    makeAircraft,
    updateAircraft,
    axisSet,
    qFromForwardUp,
    qMultiply: qMul,
    qRotate,
    qAxisAngle: qAxis,
    qFromDirectionUp: qFromForwardUp,
    axesFromQuaternion: axesFromQ,
    normalize3: normalize,
    clamp,
    smoothstep,
    GATE_RADIUS,
    GATE_BONUS,
    MISSION_TIME,
    AIRCRAFT_SPEC,
    mat4Mul,
    mat4Perspective,
    mat4LookAt,
    Sound,
    Renderer,
  };
})();
