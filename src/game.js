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

  // Public F-16C Block 50 / F110-GE-129 figures, rounded for a game-scale
  // simulation. The model is still a deliberately simplified, game-tuned
  // aero model, but it follows the real aircraft's mass, wing, thrust, fuel,
  // load-factor, roll-rate and high-altitude trends.
  const AIRCRAFT_SPEC = {
    emptyMass: 8273,
    storesMass: 450,
    mass: 11923,
    wingArea: 27.87,
    wingspan: 9.96,
    dryThrust: 76300,
    abThrust: 128922,
    fuelMax: 3200,
    fuelRateIdle: 0.18,
    fuelRateDry: 1.45,
    fuelRateAB: 5.2,
    clMax: 1.63,
    clAlpha: 3.55,
    alphaStall: 0.445,
    alphaLimit: 0.455,
    cd0: 0.019,
    inducedK: 0.118,
    gLimit: 9.0,
    negGLimit: -2.4,
    rollRateMax: 4.45,
    minRollAuthority: 0.18,
  };

  // ISA density and speed of sound. The density lapse matters: a jet at
  // 8,000 ft has noticeably more lift and thrust than one at 30,000 ft.
  function atmosphereAt(altitudeM) {
    const h = clamp(altitudeM, 0, 20000);
    let rho, sound, temperature, pressure;
    if (h <= 11000) {
      const theta = 1 - 2.25577e-5 * h;
      rho = 1.225 * Math.pow(theta, 4.2559);
      temperature = 288.15 - 0.0065 * h;
      pressure = 101325 * Math.pow(theta, 5.2559);
      sound = Math.sqrt(1.4 * 287.05287 * temperature);
    } else {
      rho = 0.3639 * Math.exp(-(h - 11000) / 6341.62);
      sound = 295.1;
      temperature = 216.65;
      pressure = 22632.1 * Math.exp(-(h - 11000) / 6341.62);
    }
    return { rho: Math.max(rho, 0.0001), sound, temperature, pressure };
  }

  // Compressible pitot impact pressure, including a normal shock above Mach 1.
  function impactRatio(mach) {
    if (mach <= 1) return Math.pow(1 + 0.2 * mach * mach, 3.5) - 1;
    return Math.pow(1.2 * mach * mach, 3.5) * Math.pow(2.4 / (2.8 * mach * mach - 0.4), 2.5) - 1;
  }

  function calibratedAirspeed(speed, atmosphere) {
    const impact = atmosphere.pressure * impactRatio(speed / atmosphere.sound) / 101325;
    let low = 0, high = 5;
    for (let i = 0; i < 32; i++) {
      const mid = (low + high) / 2;
      if (impactRatio(mid) < impact) low = mid; else high = mid;
    }
    return (low + high) * 0.5 * Math.sqrt(1.4 * 287.05287 * 288.15);
  }

  // Body-space contact points: nose, canopy, tail, wingtips, belly and fin.
  // Sampling the whole airframe means a banked wing can clip a canyon wall
  // before the centre of mass reaches the terrain.
  const COLLISION_POINTS = [
    { name: "nose", local: [0, 0.02, -8.35], margin: 0.10 },
    { name: "canopy", local: [0, 1.32, -3.0], margin: 0.16 },
    { name: "tail", local: [0, 0.48, 7.4], margin: 0.15 },
    { name: "left wing", local: [-4.92, -0.02, 1.1], margin: 0.08 },
    { name: "right wing", local: [4.92, -0.02, 1.1], margin: 0.08 },
    { name: "belly", local: [0, -1.08, 0.2], margin: 0.10 },
    { name: "fin", local: [0, 2.85, 6.7], margin: 0.10 },
  ];

  function terrainFloor(x, z) {
    return Math.max(0, terrainAt(x, z));
  }

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
    // Start very close to 1G trim: a small nose-up body attitude relative to
    // the velocity vector gives the wing the lift it needs without an instant
    // sink at mission start.
    let q = qFromForwardUp(forward, [0, 1, 0]);
    q = qMul(q, qAxis([1, 0, 0], 2.35 * DEG));
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
    const aircraft = {
      pos: [...mission.start.pos],
      vel: [...mission.start.velocity],
      q: [...mission.start.q],
      omega: [0, 0, 0],
      throttle: mission.start.throttle,
      engineSpool: mission.start.throttle,
      abLevel: 0,
      thrustLevel: mission.start.throttle,
      ab: false,
      brake: 0,
      fuel: AIRCRAFT_SPEC.fuelMax,
      mass: AIRCRAFT_SPEC.emptyMass + AIRCRAFT_SPEC.storesMass + AIRCRAFT_SPEC.fuelMax,
      speed: len(mission.start.velocity),
      airspeedKt: calibratedAirspeed(len(mission.start.velocity), atmosphereAt(mission.start.pos[1])) * 1.94384,
      altitudeM: mission.start.pos[1],
      agl: 0,
      clearance: 0,
      alpha: 0,
      alphaDeg: 0,
      beta: 0,
      mach: 0,
      qbar: 0,
      gLoad: 1,
      gSmooth: 1,
      maxG: 1,
      gAvail: AIRCRAFT_SPEC.gLimit,
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
      impactType: null,
      impactPoint: null,
      lastPos: [...mission.start.pos],
      loadAccel: 0,
      cl: 0,
      cd: 0,
      liftN: 0,
      dragN: 0,
      thrustN: 0,
    };
    updateAircraft(aircraft, 0, {});
    return aircraft;
  }

  function axisSet(q) {
    return axesFromQ(q);
  }

  function detectTerrainCollision(a) {
    let minClearance = Infinity;
    let hit = null;
    let hitFloor = 0;
    for (const point of COLLISION_POINTS) {
      const p = add(a.pos, qRotate(a.q, point.local));
      const floor = terrainFloor(p[0], p[2]);
      const clearance = p[1] - floor - point.margin;
      if (clearance < minClearance) {
        minClearance = clearance;
        hitFloor = floor;
      }
      if (clearance < 0 && !hit) {
        hit = { point, p, floor, clearance };
        hitFloor = floor;
      }
    }
    a.clearance = Math.max(0, minClearance);
    if (hit) {
      a.crashed = true;
      a.impactSpeed = len(a.vel);
      a.impactVS = a.vel[1];
      a.impactType = hitFloor < 0.5 ? "WATER" : "TERRAIN";
      a.impactPoint = [...hit.p];
      a.collisionLabel = hit.point.name;
      a.vel = mul(a.vel, 0.08);
      a.omega = [0, 0, 0];
      a.pos[1] = Math.max(a.pos[1], hit.floor + 0.8);
    }
    return a.clearance;
  }

  function updateAircraft(a, dt, input) {
    if (a.crashed) return;

    a.lastPos = [...a.pos];
    const ax = axisSet(a.q);
    const atmosphere = atmosphereAt(a.pos[1]);
    const rho = atmosphere.rho;
    const sound = atmosphere.sound;
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

    // Pilot inputs. Keyboard and mouse axes are additive so a player can use
    // the mouse for fine pitch/yaw and the keyboard for deliberate roll.
    const pitchIn = clamp((input.pitchAxis || 0) + (input.pitchUp ? 1 : 0) - (input.pitchDown ? 1 : 0), -1, 1);
    const rollIn = clamp((input.rollAxis || 0) + (input.rollRight ? 1 : 0) - (input.rollLeft ? 1 : 0), -1, 1);
    const yawIn = clamp((input.yawAxis || 0) + (input.yawRight ? 1 : 0) - (input.yawLeft ? 1 : 0), -1, 1);

    if (input.throttleUp) a.throttle = clamp(a.throttle + dt * 0.62, 0, 1);
    if (input.throttleDown) a.throttle = clamp(a.throttle - dt * 0.62, 0, 1);

    const fueled = a.fuel > 0;
    const wantAB = Boolean(input.afterburner) && a.throttle >= 0.95 && fueled;
    a.ab = wantAB;
    a.brake = approach(a.brake, input.airbrake ? 1 : 0, dt * 2.5);
    a.mass = AIRCRAFT_SPEC.emptyMass + AIRCRAFT_SPEC.storesMass + a.fuel;

    const aglCenter = a.pos[1] - terrainFloor(a.pos[0], a.pos[2]);
    const groundEffect = 1 + 0.045 * Math.exp(-Math.max(0, aglCenter) / 18);
    const mach = speed / sound;
    const clMaxEff = AIRCRAFT_SPEC.clMax * Math.max(0.82, 1 - 0.28 * smoothstep(0.85, 1.25, mach));
    const liftCapacity = qbar * AIRCRAFT_SPEC.wingArea * clMaxEff * groundEffect;
    const gAvailable = liftCapacity / (a.mass * G0);
    a.gAvail = Math.min(AIRCRAFT_SPEC.gLimit, Math.max(0, gAvailable));

    // FBW pitch command: ask for a load factor, then convert that demand into
    // a pitch rate for the current speed. At high altitude the lower dynamic
    // pressure reduces both available G and pitch response; at low speed the
    // alpha limiter protects the jet from departing.
    let pitchTarget = 0;
    if (pitchIn !== 0) {
      if (pitchIn > 0) {
        const nLimit = Math.max(1.12, Math.min(AIRCRAFT_SPEC.gLimit, gAvailable));
        const nDemand = 1 + pitchIn * (nLimit - 1);
        const baseRate = G0 * (nDemand - 1) / Math.max(speed, 70);
        const alphaMargin = clamp((AIRCRAFT_SPEC.alphaLimit - Math.max(0, alpha)) / 0.07, 0, 1);
        pitchTarget = baseRate * alphaMargin;
        if (a.gLoad > AIRCRAFT_SPEC.gLimit + 0.12) pitchTarget = Math.min(pitchTarget, 0);
      } else {
        const nDemand = 1 + pitchIn * 2.2;
        const baseRate = G0 * (nDemand - 1) / Math.max(speed, 70);
        const alphaMargin = clamp((alpha + 0.38) / 0.08, 0, 1);
        pitchTarget = baseRate * alphaMargin;
        if (a.gLoad < AIRCRAFT_SPEC.negGLimit - 0.1) pitchTarget = Math.max(pitchTarget, 0);
      }
      pitchTarget *= 1 - 0.18 * smoothstep(0.9, 1.6, mach);
      pitchTarget = clamp(pitchTarget, -0.9, 0.9);
    }

    const qRef = 0.5 * 1.225 * 200 * 200;
    const rollAuthority = clamp(Math.sqrt(Math.max(0, qbar) / qRef), AIRCRAFT_SPEC.minRollAuthority, 1.18);
    const maxRollRate = Math.min(4.6, AIRCRAFT_SPEC.rollRateMax * rollAuthority);
    const targetRoll = -rollIn * maxRollRate;
    const yawAuthority = clamp(Math.sqrt(Math.max(0, qbar) / qRef), 0.12, 1.05);
    const targetYaw = -yawIn * 0.62 * yawAuthority - beta * 1.9 - rollIn * 0.018 * Math.min(speed / 220, 1);

    const pitchResponse = clamp(dt * (3.8 + qbar / 6500), 0, 1);
    const yawResponse = clamp(dt * (3.0 + qbar / 16000), 0, 1);
    const rollResponse = clamp(dt * (4.5 + qbar / 12000), 0, 1);
    a.omega[0] += (pitchTarget - a.omega[0]) * pitchResponse;
    a.omega[1] += (targetYaw - a.omega[1]) * yawResponse;
    a.omega[2] += (targetRoll - a.omega[2]) * rollResponse;

    // Low-altitude roughness is recorded for camera shake and audio only.
    a.turbulence = 1 - smoothstep(300, 1700, a.pos[1] - terrainFloor(a.pos[0], a.pos[2]));
    a.q = qIntegrate(a.q, a.omega, dt);

    // Aerodynamics -----------------------------------------------------------
    const absAlpha = Math.abs(alpha);
    let cl;
    if (absAlpha <= AIRCRAFT_SPEC.alphaStall) {
      cl = clMaxEff * (absAlpha / AIRCRAFT_SPEC.alphaStall);
    } else {
      cl = Math.max(0.42, clMaxEff - 1.95 * (absAlpha - AIRCRAFT_SPEC.alphaStall));
    }
    cl = clamp(cl, 0, clMaxEff);
    if (alpha < 0) cl = -cl;
    const stall = smoothstep(AIRCRAFT_SPEC.alphaStall - 0.015, AIRCRAFT_SPEC.alphaStall + 0.11, absAlpha);
    const machBuffet = smoothstep(0.92, 1.08, mach) * (1 - smoothstep(1.16, 1.4, mach)) * 0.6;
    a.stall = stall;
    a.buffet = clamp(Math.max(stall, (absAlpha - 0.32) / 0.15) + machBuffet, 0, 1);

    const lift = qbar * AIRCRAFT_SPEC.wingArea * cl * groundEffect;
    let cd = AIRCRAFT_SPEC.cd0 + AIRCRAFT_SPEC.inducedK * cl * cl;
    cd += a.brake * 0.05;
    const waveDrag = 0.004
      + 0.032 * Math.exp(-Math.pow((mach - 1.04) / 0.22, 2))
      + 0.018 * smoothstep(1.15, 1.85, mach);
    cd += waveDrag + stall * 0.10;
    const drag = qbar * AIRCRAFT_SPEC.wingArea * cd;
    const side = -qbar * AIRCRAFT_SPEC.wingArea * 1.25 * beta;

    let liftDirLocal = cross([1, 0, 0], normalize(localV));
    if (len(liftDirLocal) < 0.001) liftDirLocal = [0, 1, 0];
    liftDirLocal = normalize(liftDirLocal);
    const liftWorld = mul(qRotate(a.q, liftDirLocal), lift);
    const sideWorld = qRotate(a.q, [side, 0, 0]);
    const dragWorld = speed > 0.5 ? mul(a.vel, -drag / speed) : [0, 0, 0];

    // Engine -----------------------------------------------------------------
    // F110 spool and afterburner light-up are intentionally not instant. The
    // density lapse and ram effect reproduce the most useful real-world trend:
    // less thrust upstairs, more thrust at speed.
    const targetSpool = fueled ? (wantAB ? 1 : Math.max(0.08, a.throttle)) : 0;
    a.engineSpool += (targetSpool - a.engineSpool) * clamp(dt * (wantAB ? 2.0 : 1.4), 0, 1);
    a.abLevel += ((wantAB ? 1 : 0) - a.abLevel) * clamp(dt * 3.5, 0, 1);
    const densityRatio = rho / 1.225;
    const lapse = Math.pow(densityRatio, 0.72);
    const ram = Math.max(0.55, (1 + 0.22 * Math.min(mach, 1.6) - 0.06 * Math.max(0, mach - 1.6))
      * (1 - 0.22 * smoothstep(2.0, 2.4, mach)));
    const milThrust = AIRCRAFT_SPEC.dryThrust * (0.055 + 0.945 * a.throttle) * a.engineSpool;
    const abExtra = (AIRCRAFT_SPEC.abThrust - AIRCRAFT_SPEC.dryThrust) * a.abLevel;
    a.thrustN = fueled ? (milThrust + abExtra) * lapse * ram : 0;
    const thrustWorld = mul(ax.forward, a.thrustN);
    const gravity = [0, -a.mass * G0, 0];

    const aeroForce = add(add(liftWorld, dragWorld), sideWorld);
    const force = add(add(aeroForce, thrustWorld), gravity);
    const accel = mul(force, 1 / a.mass);
    a.loadAccel = dot(add(aeroForce, thrustWorld), ax.up) / (a.mass * G0);
    a.gLoad = a.loadAccel;
    a.gSmooth += (a.gLoad - a.gSmooth) * clamp(dt * 4, 0, 1);
    a.maxG = Math.max(a.maxG, a.gSmooth);

    a.vel = add(a.vel, mul(accel, dt));
    a.pos = add(a.pos, mul(a.vel, dt));

    const fuelFlow = wantAB
      ? AIRCRAFT_SPEC.fuelRateAB
      : AIRCRAFT_SPEC.fuelRateIdle + AIRCRAFT_SPEC.fuelRateDry * clamp(a.throttle, 0, 1);
    a.fuelFlow = fueled ? fuelFlow : 0;
    a.fuel = Math.max(0, a.fuel - a.fuelFlow * dt);
    a.rpm = a.fuel > 0 ? 61 + 38 * a.engineSpool : 99 * a.engineSpool;
    a.ftit = a.fuel > 0 ? 420 + 480 * a.engineSpool + 130 * a.abLevel : 20 + 400 * a.engineSpool;
    a.nozzle = 15 + 80 * a.abLevel + 60 * Math.max(0, 1 - a.engineSpool * 2);
    a.oilPsi = Math.max(0, a.rpm * 0.65);

    // Derived data for instruments.
    a.speed = len(a.vel);
    const newAtmosphere = atmosphereAt(a.pos[1]);
    a.airspeedKt = calibratedAirspeed(a.speed, newAtmosphere) * 1.94384;
    a.tasKt = a.speed * 1.94384;
    a.easKt = a.tasKt * Math.sqrt(newAtmosphere.rho / 1.225);
    a.altitudeM = a.pos[1];
    a.agl = a.pos[1] - terrainFloor(a.pos[0], a.pos[2]);
    a.alpha = alpha;
    a.alphaDeg = alpha * RAD;
    a.beta = beta;
    a.mach = a.speed / newAtmosphere.sound;
    a.qbar = 0.5 * rho * a.speed * a.speed;
    a.cl = cl;
    a.cd = cd;
    a.liftN = lift;
    a.dragN = drag;
    a.vs = a.vel[1];
    const axNow = axisSet(a.q);
    a.heading = wrap(Math.atan2(axNow.forward[0], -axNow.forward[2]) * RAD, 360);
    a.pitch = Math.asin(clamp(axNow.forward[1], -1, 1)) * RAD;
    a.bank = Math.atan2(-axNow.right[1], axNow.up[1]) * RAD;
    a.afterburnerVisual += (a.abLevel - a.afterburnerVisual) * clamp(dt * 5, 0, 1);

    // Collision is evaluated against the whole airframe, not just its centre.
    detectTerrainCollision(a);
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

  const VS_AIRCRAFT = `#version 300 es
    precision highp float;
    in vec3 aPos;
    in vec3 aNormal;
    in vec3 aColor;
    uniform mat4 u_vp;
    uniform mat4 u_model;
    out vec3 vWorld;
    out vec3 vNormal;
    out vec3 vColor;
    void main() {
      vec4 world = u_model * vec4(aPos, 1.0);
      vWorld = world.xyz;
      vNormal = mat3(u_model) * aNormal;
      vColor = aColor;
      gl_Position = u_vp * world;
    }
  `;

  const FS_AIRCRAFT = `#version 300 es
    precision highp float;
    in vec3 vWorld;
    in vec3 vNormal;
    in vec3 vColor;
    uniform vec3 u_camPos;
    uniform vec3 u_sunDir;
    uniform vec3 u_fogColor;
    uniform float u_alpha;
    uniform float u_glow;
    out vec4 outColor;
    void main() {
      vec3 n = normalize(vNormal);
      float lit = max(abs(dot(n, normalize(u_sunDir))), 0.0);
      float sky = max(dot(n, vec3(0.0, 1.0, 0.0)), 0.0);
      vec3 col = vColor * (0.56 + 0.48 * lit);
      col += vec3(0.035, 0.05, 0.045) * sky;
      vec3 viewDir = normalize(u_camPos - vWorld);
      float rim = pow(1.0 - max(dot(n, viewDir), 0.0), 3.0);
      col += vec3(0.16, 0.2, 0.18) * rim * 0.35;
      if (u_glow > 0.5) col *= 1.8;
      float dist = length(vWorld - u_camPos);
      float fog = clamp(1.0 - exp(-dist * 0.000045), 0.0, 0.9);
      col = mix(col, u_fogColor, fog);
      outColor = vec4(col, u_alpha * (1.0 - fog * 0.8));
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
        aircraft: this.program(VS_AIRCRAFT, FS_AIRCRAFT, "aircraft"),
        post: this.program(VS_FULL, FS_POST, "post"),
      };
      this.uniforms = {};
      for (const [name, p] of Object.entries(this.programs)) {
        this.uniforms[name] = this.uniformMap(p.program);
      }

      this.createTerrain();
      this.createOcean();
      this.createGate();
      this.createAircraft();
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

    createAircraft() {
      const gl = this.gl;
      const verts = [];
      const flameVerts = [];
      const push = (arr, p, n, c) => {
        arr.push(p[0], p[1], p[2], n[0], n[1], n[2], c[0], c[1], c[2]);
      };
      const tri = (arr, a, b, c, color, normal) => {
        let n = normal;
        if (!n) {
          n = normalize(cross(sub(b, a), sub(c, a)));
          if (len(n) < 1e-5) n = [0, 1, 0];
        }
        push(arr, a, n, color);
        push(arr, b, n, color);
        push(arr, c, n, color);
      };
      const quad = (arr, a, b, c, d, color, normal) => {
        tri(arr, a, b, c, color, normal);
        tri(arr, a, c, d, color, normal);
      };
      const box = (arr, center, size, color, topColor) => {
        const [x, y, z] = center;
        const [sx, sy, sz] = size;
        const x0 = x - sx / 2, x1 = x + sx / 2;
        const y0 = y - sy / 2, y1 = y + sy / 2;
        const z0 = z - sz / 2, z1 = z + sz / 2;
        const p = [
          [x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0],
          [x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1],
        ];
        quad(arr, p[0], p[1], p[2], p[3], color, [0, 0, -1]);
        quad(arr, p[5], p[4], p[7], p[6], color, [0, 0, 1]);
        quad(arr, p[4], p[0], p[3], p[7], color, [-1, 0, 0]);
        quad(arr, p[1], p[5], p[6], p[2], color, [1, 0, 0]);
        quad(arr, p[3], p[2], p[6], p[7], topColor || color, [0, 1, 0]);
        quad(arr, p[4], p[5], p[1], p[0], color, [0, -1, 0]);
      };
      const addWing = (sign) => {
        const xr = 0.68 * sign, xt = 4.96 * sign;
        const yr = 0.03, yt = -0.035;
        const tr = 0.14, tt = 0.045;
        const rLE = [xr, yr + tr, -2.35], rTE = [xr, yr + tr, 3.72];
        const tLE = [xt, yt + tt, 1.62], tTE = [xt, yt + tt, 3.42];
        const rLEb = [xr, yr - tr, -2.35], rTEb = [xr, yr - tr, 3.72];
        const tLEb = [xt, yt - tt, 1.62], tTEb = [xt, yt - tt, 3.42];
        quad(verts, rLE, rTE, tTE, tLE, [0.47, 0.51, 0.46], [0, 1, 0]);
        quad(verts, rLEb, tLEb, tTEb, rTEb, [0.31, 0.35, 0.32], [0, -1, 0]);
        quad(verts, rTE, tTE, tTEb, rTEb, [0.39, 0.43, 0.39], [0, 0, 1]);
        quad(verts, tLE, rLE, rLEb, tLEb, [0.35, 0.39, 0.35], [0, 0, -1]);
        quad(verts, tLE, tTE, tTEb, tLEb, [0.39, 0.43, 0.39], [sign, 0, 0]);
        quad(verts, rLE, tLE, tLEb, rLEb, [0.30, 0.34, 0.31], [sign, 0, 0]);
        // Dark leading-edge strip.
        const leIn = [xr, yr + tr + 0.004, -2.23], leTipIn = [xt, yt + tt + 0.004, 1.74];
        quad(verts, rLE, leIn, leTipIn, tLE, [0.24, 0.28, 0.26], [0, 1, 0]);
      };
      const addStabilator = (sign) => {
        const xr = 0.48 * sign, xt = 2.92 * sign;
        const y = 0.08;
        const p1 = [xr, y + 0.06, 5.35], p2 = [xr, y + 0.06, 7.62];
        const p3 = [xt, y, 6.72], p4 = [xt, y, 7.68];
        const p1b = [xr, y - 0.06, 5.35], p2b = [xr, y - 0.06, 7.62];
        const p3b = [xt, y - 0.045, 6.72], p4b = [xt, y - 0.045, 7.68];
        quad(verts, p1, p2, p4, p3, [0.44, 0.48, 0.44], [0, 1, 0]);
        quad(verts, p1b, p3b, p4b, p2b, [0.30, 0.34, 0.31], [0, -1, 0]);
        quad(verts, p2, p4, p4b, p2b, [0.36, 0.40, 0.36], [0, 0, 1]);
        quad(verts, p1b, p3b, p3, p1, [0.36, 0.40, 0.36], [0, 0, -1]);
        quad(verts, p3, p4, p4b, p3b, [0.37, 0.41, 0.37], [sign, 0, 0]);
      };
      const addFin = () => {
        const x = 0.075;
        const rLE = [-x, 0.72, 5.02], rTE = [-x, 0.82, 7.72];
        const tLE = [-x, 3.02, 6.20], tTE = [-x, 2.86, 7.62];
        const RLE = [x, 0.72, 5.02], RTE = [x, 0.82, 7.72];
        const TLE = [x, 3.02, 6.20], TTE = [x, 2.86, 7.62];
        quad(verts, rLE, rTE, tTE, tLE, [0.40, 0.44, 0.40], [-1, 0, 0]);
        quad(verts, RLE, TLE, TTE, RTE, [0.46, 0.50, 0.45], [1, 0, 0]);
        quad(verts, rLE, tLE, TLE, RLE, [0.31, 0.35, 0.32], [0, 0, -1]);
        quad(verts, rTE, RTE, TTE, tTE, [0.33, 0.37, 0.33], [0, 0, 1]);
        quad(verts, tLE, tTE, TTE, TLE, [0.37, 0.41, 0.37], [0, 1, 0]);
        // Tail-code stripe.
        quad(verts, [-x - 0.002, 1.58, 6.72], [-x - 0.002, 1.62, 7.42],
          [-x - 0.002, 1.86, 7.39], [-x - 0.002, 1.82, 6.68], [0.16, 0.20, 0.18], [-1, 0, 0]);
      };
      const addVentralFin = (sign) => {
        const x = 0.92 * sign;
        const a = [x, -0.34, 5.6], b = [x, -0.34, 7.25];
        const c = [x + 0.16 * sign, -1.02, 6.85], d = [x + 0.16 * sign, -0.55, 6.95];
        quad(verts, a, b, d, c, [0.35, 0.39, 0.35], [sign, 0, 0]);
      };
      const addEllipsoid = (cx, cy, cz, rx, ry, rz, color, lat = 6, lon = 10) => {
        const pt = (i, j) => {
          const v = (i / lat) * Math.PI;
          const u = (j / lon) * TAU;
          return [cx + rx * Math.sin(v) * Math.cos(u), cy + ry * Math.cos(v), cz + rz * Math.sin(v) * Math.sin(u)];
        };
        for (let i = 0; i < lat; i++) {
          for (let j = 0; j < lon; j++) {
            const a = pt(i, j), b = pt(i, j + 1), c = pt(i + 1, j + 1), d = pt(i + 1, j);
            tri(verts, a, b, c, color);
            tri(verts, a, c, d, color);
          }
        }
      };
      const lathe = (stations, segments, color) => {
        const ring = (i, j) => {
          const st = stations[i];
          const a = (j / segments) * TAU;
          return [Math.cos(a) * st.rx, st.cy + Math.sin(a) * st.ry, st.z];
        };
        for (let i = 0; i < stations.length - 1; i++) {
          for (let j = 0; j < segments; j++) {
            const a = ring(i, j), b = ring(i, j + 1), c = ring(i + 1, j + 1), d = ring(i + 1, j);
            const st = stations[i];
            const col = st.color || color;
            quad(verts, a, b, c, d, col);
          }
        }
      };

      const body = [0.46, 0.50, 0.45];
      const dark = [0.30, 0.34, 0.31];
      const belly = [0.38, 0.42, 0.38];
      const radome = [0.18, 0.20, 0.19];
      const canopy = [0.07, 0.15, 0.15];
      const metal = [0.31, 0.34, 0.32];
      const nozzle = [0.24, 0.25, 0.23];

      // Fuselage and radome.
      lathe([
        { z: -8.55, rx: 0.015, ry: 0.015, cy: 0.28, color: radome },
        { z: -8.15, rx: 0.16, ry: 0.13, cy: 0.28, color: radome },
        { z: -7.55, rx: 0.33, ry: 0.28, cy: 0.25, color: radome },
        { z: -6.9, rx: 0.52, ry: 0.40, cy: 0.20, color: body },
        { z: -5.6, rx: 0.68, ry: 0.55, cy: 0.14, color: body },
        { z: -3.2, rx: 0.82, ry: 0.66, cy: 0.08, color: body },
        { z: 0.0, rx: 0.86, ry: 0.69, cy: 0.03, color: body },
        { z: 2.6, rx: 0.82, ry: 0.66, cy: 0.0, color: body },
        { z: 4.7, rx: 0.68, ry: 0.56, cy: 0.0, color: body },
        { z: 6.3, rx: 0.53, ry: 0.45, cy: 0.0, color: body },
        { z: 7.2, rx: 0.43, ry: 0.38, cy: 0.0, color: dark },
      ], 14, body);
      // Dorsal spine.
      box(verts, [0, 0.57, 1.9], [0.42, 0.16, 7.0], dark, [0.36, 0.40, 0.36]);
      // Intake.
      box(verts, [0, -0.83, -0.25], [1.30, 0.62, 4.15], belly, [0.42, 0.46, 0.42]);
      quad(verts, [-0.56, -1.11, -2.34], [0.56, -1.11, -2.34], [0.56, -0.57, -2.34], [-0.56, -0.57, -2.34], [0.035, 0.045, 0.04], [0, 0, -1]);
      // The first-person mesh keeps the exterior but omits the opaque bubble.
      const exteriorBeforeCanopy = verts.length;
      // Canopy bubble and rails.
      addEllipsoid(0, 0.90, -2.78, 0.55, 0.78, 1.50, canopy, 7, 12);
      box(verts, [0, 1.16, -3.95], [1.04, 0.055, 0.10], metal);
      box(verts, [0, 1.10, -1.58], [0.94, 0.05, 0.09], metal);
      box(verts, [-0.55, 0.62, -2.78], [0.045, 0.08, 2.5], dark);
      box(verts, [0.55, 0.62, -2.78], [0.045, 0.08, 2.5], dark);
      const exteriorAfterCanopy = verts.length;
      // Wings, tails and fin.
      addWing(1);
      addWing(-1);
      addStabilator(1);
      addStabilator(-1);
      addFin();
      addVentralFin(1);
      addVentralFin(-1);
      // Wingtip rails and navigation lights.
      box(verts, [4.82, 0.02, 2.9], [0.11, 0.10, 1.55], metal);
      box(verts, [-4.82, 0.02, 2.9], [0.11, 0.10, 1.55], metal);
      quad(verts, [4.94, 0.05, 3.36], [4.94, 0.15, 3.36], [4.94, 0.15, 3.50], [4.94, 0.05, 3.50], [0.08, 1.0, 0.22], [1, 0, 0]);
      quad(verts, [-4.94, 0.05, 3.36], [-4.94, 0.05, 3.50], [-4.94, 0.15, 3.50], [-4.94, 0.15, 3.36], [1.0, 0.08, 0.04], [-1, 0, 0]);
      // Nozzle ring and exhaust disc.
      lathe([
        { z: 6.72, rx: 0.44, ry: 0.37, cy: 0.0, color: dark },
        { z: 7.18, rx: 0.49, ry: 0.41, cy: 0.0, color: nozzle },
        { z: 7.78, rx: 0.46, ry: 0.39, cy: 0.0, color: nozzle },
      ], 14, nozzle);
      const ex = 7.78, exr = 0.40;
      for (let i = 0; i < 12; i++) {
        const a0 = (i / 12) * TAU, a1 = ((i + 1) / 12) * TAU;
        tri(verts, [0, 0, ex + 0.02], [Math.cos(a1) * exr, Math.sin(a1) * exr * 0.84, ex], [Math.cos(a0) * exr, Math.sin(a0) * exr * 0.84, ex], [0.025, 0.03, 0.028], [0, 0, 1]);
      }

      // Afterburner plume (drawn additively only in chase view).
      const flameStations = [
        { z: 7.84, rx: 0.40, ry: 0.33, cy: 0.0 },
        { z: 8.45, rx: 0.36, ry: 0.30, cy: 0.0 },
        { z: 9.5, rx: 0.22, ry: 0.18, cy: 0.0 },
        { z: 10.9, rx: 0.035, ry: 0.03, cy: 0.0 },
      ];
      const flameRing = (i, j, segs = 10) => {
        const st = flameStations[i];
        const a = (j / segs) * TAU;
        return [Math.cos(a) * st.rx, st.cy + Math.sin(a) * st.ry, st.z];
      };
      for (let i = 0; i < flameStations.length - 1; i++) {
        for (let j = 0; j < 10; j++) {
          const a = flameRing(i, j), b = flameRing(i, j + 1), c = flameRing(i + 1, j + 1), d = flameRing(i + 1, j);
          const col = i === 0 ? [1.0, 0.66, 0.20] : i === 1 ? [1.0, 0.40, 0.06] : [0.95, 0.22, 0.02];
          quad(flameVerts, a, b, c, d, col);
        }
      }

      const setup = (arr) => {
        const vao = gl.createVertexArray();
        gl.bindVertexArray(vao);
        this.buffer(gl.ARRAY_BUFFER, new Float32Array(arr));
        const stride = 9 * 4;
        const aPos = gl.getAttribLocation(this.programs.aircraft.program, "aPos");
        const aNormal = gl.getAttribLocation(this.programs.aircraft.program, "aNormal");
        const aColor = gl.getAttribLocation(this.programs.aircraft.program, "aColor");
        gl.enableVertexAttribArray(aPos);
        gl.vertexAttribPointer(aPos, 3, gl.FLOAT, false, stride, 0);
        gl.enableVertexAttribArray(aNormal);
        gl.vertexAttribPointer(aNormal, 3, gl.FLOAT, false, stride, 12);
        gl.enableVertexAttribArray(aColor);
        gl.vertexAttribPointer(aColor, 3, gl.FLOAT, false, stride, 24);
        gl.bindVertexArray(null);
        return { vao, count: arr.length / 9 };
      };
      this.aircraftMesh = setup(verts);
      this.cockpitExteriorMesh = setup([...verts.slice(0, exteriorBeforeCanopy), ...verts.slice(exteriorAfterCanopy)]);
      this.flameMesh = setup(flameVerts);
    }

    drawAircraft(vp, cam, sim) {
      const gl = this.gl;
      const a = sim.aircraft;
      if (!a) return;
      const ax = axisSet(a.q);
      const model = mat4Basis(ax.right, ax.up, ax.back, a.pos);
      const p = this.programs.aircraft;
      const u = this.use(p);
      this.u(u, "u_vp", vp, "m4");
      this.u(u, "u_model", model, "m4");
      this.u(u, "u_camPos", cam.eye, "v3");
      this.u(u, "u_sunDir", this.sunDir, "v3");
      this.u(u, "u_fogColor", this.fogColor, "v3");
      this.u(u, "u_alpha", 1, "f");
      this.u(u, "u_glow", 0, "f");
      gl.enable(gl.DEPTH_TEST);
      gl.depthMask(true);
      gl.disable(gl.BLEND);
      const mesh = sim.view === "cockpit" ? this.cockpitExteriorMesh : this.aircraftMesh;
      gl.bindVertexArray(mesh.vao);
      gl.drawArrays(gl.TRIANGLES, 0, mesh.count);

      if (sim.view === "chase" && a.afterburnerVisual > 0.03) {
        gl.enable(gl.BLEND);
        gl.blendFunc(gl.SRC_ALPHA, gl.ONE);
        gl.depthMask(false);
        this.u(u, "u_alpha", 0.26 + 0.48 * a.afterburnerVisual, "f");
        this.u(u, "u_glow", 1, "f");
        gl.bindVertexArray(this.flameMesh.vao);
        gl.drawArrays(gl.TRIANGLES, 0, this.flameMesh.count);
        gl.depthMask(true);
        gl.disable(gl.BLEND);
      }
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
        p === this.programs.gate ? "gate" :
        p === this.programs.aircraft ? "aircraft" : "post"];
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
        mat4Perspective(cam.fov, this.cssW / this.cssH, sim.view === "cockpit" ? 0.08 : 0.7, 70000),
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

      // Both cameras share the exterior. First person omits the opaque bubble.
      if ((sim.view === "chase" || sim.view === "cockpit") && sim.aircraft) {
        this.drawAircraft(vp, cam, sim);
      }

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
    atmosphereAt,
    calibratedAirspeed,
    mat4Mul,
    mat4Perspective,
    mat4LookAt,
    Sound,
    Renderer,
  };
})();
