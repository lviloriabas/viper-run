/* =========================================================================
   VIPER RUN // presentation, gameplay and cockpit systems
   ========================================================================= */
(() => {
  "use strict";

  const C = window.__VIPER_CORE;
  if (!C) {
    document.body.innerHTML = '<div class="error-card"><div class="inside"><h2>BOOT FAILURE</h2>Flight core did not load.</div></div>';
    return;
  }
  const {
    terrainAt, riverX, buildMission, makeAircraft, updateAircraft,
    axisSet, qFromForwardUp, qMultiply, qRotate, qAxisAngle,
    GATE_BONUS, MISSION_TIME, AIRCRAFT_SPEC, Sound, Renderer,
  } = C;

  // Small local vector kit (the simulation keeps its own private copy).
  const clamp = C.clamp;
  const smoothstep = C.smoothstep;
  const TAU = Math.PI * 2;
  const D2R = Math.PI / 180;
  const R2D = 180 / Math.PI;
  const add = (a, b) => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
  const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
  const mul = (a, s) => [a[0] * s, a[1] * s, a[2] * s];
  const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  const cross = (a, b) => [
    a[1] * b[2] - a[2] * b[1],
    a[2] * b[0] - a[0] * b[2],
    a[0] * b[1] - a[1] * b[0],
  ];
  const length = (a) => Math.hypot(a[0], a[1], a[2]);
  const norm = (a) => {
    const l = length(a) || 1;
    return [a[0] / l, a[1] / l, a[2] / l];
  };
  const lerp = (a, b, t) => a + (b - a) * t;
  const wrap180 = (d) => ((d + 540) % 360) - 180;
  const wrap360 = (d) => ((d % 360) + 360) % 360;
  const pad = (n, size) => String(Math.max(0, Math.round(n))).padStart(size, "0");
  const formatTime = (sec) => {
    const s = Math.max(0, Math.round(sec));
    return `${pad(Math.floor(s / 60), 2)}:${pad(s % 60, 2)}`;
  };
  const ft = (m) => m * 3.28084;
  const kt = (ms) => ms * 1.94384;
  const fpm = (ms) => ms * 196.8504;

  const HUD = "#a9ffb2";
  const HUD_SOFT = "rgba(169,255,178,0.68)";
  const HUD_DIM = "rgba(169,255,178,0.48)";
  const HUD_FAINT = "rgba(169,255,178,0.18)";
  const AMBER = "#ffc05b";
  const RED = "#ff6d5f";
  const CYAN = "#7be8df";
  const CREAM = "#ece8cd";
  const STORE_KEY = "viperRun.best.v1";

  const sound = new Sound();
  const keys = new Set();
  const ui = document.getElementById("ui");
  const boot = document.getElementById("boot");
  const canvas = document.getElementById("world");
  const cockpitCanvas = document.getElementById("cockpit");
  const ctx = cockpitCanvas.getContext("2d");

  let renderer = null;
  let mapCanvas = null;
  let previewMission = null;
  let lastFrame = performance.now();
  let dpr = 1;
  let cssW = 1;
  let cssH = 1;

  const game = {
    state: "attract",
    mission: null,
    aircraft: null,
    camera: null,
    time: 0,
    attractTime: 0,
    accumulator: 0,
    toast: "",
    toastTimer: 0,
    transitionTimer: 0,
    transitionKind: null,
    flash: 0,
    flashColor: "180,255,190",
    warnClock: 0,
    muted: false,
    best: loadBest(),
    lastResult: null,
  };

  function loadBest() {
    try {
      const raw = localStorage.getItem(STORE_KEY);
      if (!raw) return { score: 0, time: 0 };
      const parsed = JSON.parse(raw);
      return { score: Number(parsed.score) || 0, time: Number(parsed.time) || 0 };
    } catch (err) {
      return { score: 0, time: 0 };
    }
  }

  function storeBest(score, time) {
    if (score > game.best.score) game.best.score = Math.round(score);
    if (time > 0 && (!game.best.time || time < game.best.time)) game.best.time = time;
    try {
      localStorage.setItem(STORE_KEY, JSON.stringify(game.best));
    } catch (err) { /* file:// or private mode — best score simply won't persist */ }
  }

  // --------------------------------------------------------------------- UI
  function setBody(state) {
    document.body.className = state;
  }

  function hideUI() {
    ui.innerHTML = "";
    ui.style.pointerEvents = "none";
  }

  function mountScreen(html) {
    ui.innerHTML = html;
    ui.style.pointerEvents = "auto";
    setBody("menu");
    const focus = ui.querySelector("[data-autofocus]");
    if (focus) setTimeout(() => focus.focus({ preventScroll: true }), 20);
  }

  function bestLine() {
    if (!game.best.score && !game.best.time) return "NO QUALIFYING RUN ON FILE";
    return `BEST ${pad(game.best.score, 5)} PTS${game.best.time ? ` // ${formatTime(game.best.time)}` : ""}`;
  }

  function startScreen() {
    const best = bestLine();
    return `
      <section class="screen">
        <div class="menu-shell">
          <div class="menu-topline">
            <span>// TACTICAL AIR COMMAND · 0715 ZULU</span>
            <div class="lights"><i class="lamp"></i><i class="lamp dim"></i><i class="lamp red"></i><span>FLCS ONLINE</span></div>
          </div>
          <div class="menu-grid">
            <div>
              <div class="kicker">F-16C BLOCK 50 · LOW LEVEL QUALIFICATION</div>
              <h1 class="title">VIPER<span><span class="slash">/</span>RUN</span></h1>
              <div class="title-rule">COLD RIVER GORGE · SECTOR 07</div>
              <p class="mission-copy">
                You are already airborne, twenty clicks from the border, with
                ten navigation gates strung down a black river gorge. Thread
                the valley. Stay under the ridgeline. Make the coast before the
                clock runs out. <strong>The jet is loaded. The weather is perfect. The mountain is not.</strong>
              </p>
              <div class="cta-row">
                <button class="btn" data-action="start" data-autofocus>ENGAGE <small>ENTER</small></button>
                <button class="btn ghost" data-action="mute">${sound.muted ? "AUDIO OFF" : "AUDIO ON"}</button>
              </div>
            </div>
            <aside class="briefing">
              <div class="briefing-label">Mission data // COLD RIVER</div>
              <div class="brief-row"><span class="k">AIRFRAME</span><span class="v"><strong>F-16C FIGHTING FALCON</strong><br>F110-GE-129 / FLCS NORM</span></div>
              <div class="brief-row"><span class="k">ROUTE</span><span class="v"><strong>10 GATES</strong> · RIVER GORGE · COASTAL EGRESS</span></div>
              <div class="brief-row"><span class="k">CLOCK</span><span class="v"><em>02:30</em> + 8 SEC PER GATE</span></div>
              <div class="brief-row"><span class="k">HAZARD</span><span class="v"><em>TERRAIN</em> · STALL · G-LIMIT</span></div>
              <div class="controls-block">
                <div class="controls-title"><span>Flight controls</span><span>Keyboard</span></div>
                <div class="keys">
                  <div><kbd>S</kbd><kbd>↓</kbd> PULL</div>
                  <div><kbd>W</kbd><kbd>↑</kbd> PUSH</div>
                  <div><kbd>A</kbd><kbd>D</kbd> ROLL</div>
                  <div><kbd>Q</kbd><kbd>E</kbd> RUDDER</div>
                  <div><kbd>SHIFT</kbd> THROTTLE +</div>
                  <div><kbd>CTRL</kbd> THROTTLE −</div>
                  <div><kbd>SPACE</kbd> AFTERBURNER</div>
                  <div><kbd>B</kbd> AIRBRAKE</div>
                  <div><kbd>P</kbd> PAUSE</div>
                  <div><kbd>M</kbd> AUDIO</div>
                </div>
              </div>
            </aside>
          </div>
          <div class="menu-footer">
            <span class="best">${best}</span>
            <span>HOLD <kbd>S</kbd> TO PULL · RELEASE TO ROLL OUT · <kbd>P</kbd> PAUSE</span>
            <span class="coords">37°14′N 081°06′E<br>ALT 2,140 FT</span>
          </div>
        </div>
      </section>`;
  }

  function pauseScreen() {
    const a = game.aircraft;
    const m = game.mission;
    return `
      <section class="screen">
        <div class="menu-shell panel-narrow">
          <div class="status-kicker">// flight suspended · flcs hold</div>
          <h2 class="status-title">PAUSED</h2>
          <p class="status-sub">Your flight is frozen on the clock. Take a breath, check the route, and re-engage when ready.</p>
          <div class="stats">
            <div class="stat"><div class="k">CLEARED</div><div class="v green">${pad(m.passed, 2)} / ${pad(m.gates.length, 2)}</div></div>
            <div class="stat"><div class="k">TIME LEFT</div><div class="v amber">${formatTime(m.timeLeft)}</div></div>
            <div class="stat"><div class="k">AIRSPEED</div><div class="v">${pad(kt(a.speed), 3)} KT</div></div>
            <div class="stat"><div class="k">ALTITUDE</div><div class="v">${pad(ft(a.pos[1]), 5)} FT</div></div>
          </div>
          <div class="result-actions">
            <button class="btn" data-action="resume" data-autofocus>RESUME <small>P</small></button>
            <button class="btn ghost" data-action="restart">RESTART RUN <small>R</small></button>
            <button class="btn ghost" data-action="menu">ABORT TO MENU</button>
          </div>
          <div class="small-print">P / ESC RESUME · R RESTART · M AUDIO ${sound.muted ? "OFF" : "ON"}</div>
        </div>
      </section>`;
  }

  function winScreen() {
    const r = game.lastResult;
    const rank = r.time < 95 ? "ACE" : r.time < 125 ? "VIPER" : r.time < 155 ? "QUALIFIED" : "CADET";
    return `
      <section class="screen">
        <div class="menu-shell panel-narrow">
          <div class="status-kicker">// route complete · cold river gorge</div>
          <h2 class="status-title win">COURSE<br>COMPLETE</h2>
          <p class="status-sub">All ten gates cleared. You brought the jet and the terrain back in one piece. The evaluator has logged the run as <strong>${rank}</strong>.</p>
          <div class="stats">
            <div class="stat"><div class="k">RUN TIME</div><div class="v green">${formatTime(r.time)}</div></div>
            <div class="stat"><div class="k">GATES</div><div class="v">${r.gates} / 10</div></div>
            <div class="stat"><div class="k">SCORE</div><div class="v amber">${pad(r.score, 5)}</div></div>
            <div class="stat"><div class="k">RATING</div><div class="v">${rank}</div></div>
          </div>
          <div class="result-actions">
            <button class="btn" data-action="restart" data-autofocus>FLY AGAIN <small>R</small></button>
            <button class="btn ghost" data-action="menu">MISSION BOARD</button>
          </div>
          <div class="small-print">${bestLine()} · R RESTART · ESC MENU</div>
        </div>
      </section>`;
  }

  function crashScreen() {
    const r = game.lastResult;
    return `
      <section class="screen">
        <div class="menu-shell panel-narrow">
          <div class="status-kicker" style="color:var(--red)">// flight terminated · airframe lost</div>
          <h2 class="status-title fail">TERRAIN<br>WINS</h2>
          <p class="status-sub">The gorge is unforgiving. The flight recorder shows ${pad(r.gates, 2)} gates cleared before impact.</p>
          <div class="stats">
            <div class="stat"><div class="k">GATE</div><div class="v red">${pad(r.gates, 2)} / 10</div></div>
            <div class="stat"><div class="k">ELAPSED</div><div class="v">${formatTime(r.time)}</div></div>
            <div class="stat"><div class="k">IMPACT</div><div class="v red">${pad(kt(r.impact), 3)} KT</div></div>
            <div class="stat"><div class="k">BEST</div><div class="v">${game.best.score ? pad(game.best.score, 5) : "-----"}</div></div>
          </div>
          <div class="result-actions">
            <button class="btn" data-action="restart" data-autofocus>REARM &amp; RETRY <small>R</small></button>
            <button class="btn ghost" data-action="menu">MISSION BOARD</button>
          </div>
          <div class="small-print">TIP: EASE OFF THE PULL AT HIGH SPEED. LET THE GORGE OPEN BEFORE YOU ROLL IN.</div>
        </div>
      </section>`;
  }

  function timeoutScreen() {
    const r = game.lastResult;
    return `
      <section class="screen">
        <div class="menu-shell panel-narrow">
          <div class="status-kicker" style="color:var(--amber)">// clock expired · run invalid</div>
          <h2 class="status-title fail" style="color:var(--amber)">RUN<br>EXPIRED</h2>
          <p class="status-sub">The coast is still out there. You cleared ${pad(r.gates, 2)} gates before the clock ran dry.</p>
          <div class="stats">
            <div class="stat"><div class="k">GATE</div><div class="v amber">${pad(r.gates, 2)} / 10</div></div>
            <div class="stat"><div class="k">ELAPSED</div><div class="v">${formatTime(r.time)}</div></div>
            <div class="stat"><div class="k">SCORE</div><div class="v">${pad(r.score, 5)}</div></div>
            <div class="stat"><div class="k">BEST</div><div class="v">${game.best.score ? pad(game.best.score, 5) : "-----"}</div></div>
          </div>
          <div class="result-actions">
            <button class="btn" data-action="restart" data-autofocus>TRY AGAIN <small>R</small></button>
            <button class="btn ghost" data-action="menu">MISSION BOARD</button>
          </div>
          <div class="small-print">THE CLOCK IS A HARD CEILING. GATES ADD 8 SECONDS EACH.</div>
        </div>
      </section>`;
  }

  function action(name) {
    if (name === "start") {
      sound.init();
      startMission(true);
    } else if (name === "resume") {
      sound.init();
      resumeMission();
    } else if (name === "restart") {
      sound.init();
      startMission(true);
    } else if (name === "menu") {
      goMenu();
    } else if (name === "mute") {
      sound.init();
      game.muted = sound.toggleMute();
      if (game.state === "attract") mountScreen(startScreen());
      else if (game.state === "paused") mountScreen(pauseScreen());
      flashToast(game.muted ? "AUDIO MUTED" : "AUDIO ON", 1.6);
    }
  }

  // ----------------------------------------------------------------- gameplay
  function flashToast(text, duration = 2.2) {
    game.toast = text;
    game.toastTimer = duration;
  }

  function startMission(fromGesture = true) {
    const mission = buildMission();
    game.mission = mission;
    game.aircraft = makeAircraft(mission);
    game.state = "flying";
    game.accumulator = 0;
    game.transitionTimer = 0;
    game.flash = 0;
    game.toastTimer = 0;
    keys.clear();
    hideUI();
    setBody("playing");
    if (fromGesture && sound.ctx && sound.ctx.state === "suspended") sound.ctx.resume().catch(() => {});
    if (fromGesture) sound.init();
    flashToast("COLD RIVER // 10 GATES // CLEARED HOT", 3.1);
    lastFrame = performance.now();
  }

  function pauseMission() {
    if (game.state !== "flying") return;
    game.state = "paused";
    keys.clear();
    if (sound.ctx) sound.ctx.suspend().catch(() => {});
    mountScreen(pauseScreen());
  }

  function resumeMission() {
    if (game.state !== "paused") return;
    game.state = "flying";
    keys.clear();
    hideUI();
    setBody("playing");
    if (sound.ctx) sound.ctx.resume().catch(() => {});
    lastFrame = performance.now();
    game.accumulator = 0;
  }

  function goMenu() {
    game.state = "attract";
    game.attractTime = 0;
    keys.clear();
    hideUI();
    setBody("menu");
    mountScreen(startScreen());
    if (sound.ctx && sound.ctx.state === "running") sound.ctx.suspend().catch(() => {});
  }

  function finalScore(mission) {
    return Math.round(mission.passed * 1000 + Math.max(0, mission.timeLeft) * 25);
  }

  function beginFinish(kind) {
    if (game.state !== "flying") return;
    const mission = game.mission;
    const a = game.aircraft;
    mission.score = finalScore(mission);
    game.lastResult = {
      kind,
      time: mission.elapsed,
      gates: mission.passed,
      score: mission.score,
      impact: a.impactSpeed || a.speed,
      totalGates: mission.gates.length,
    };
    game.transitionKind = kind;
    game.transitionTimer = kind === "win" ? 1.35 : kind === "crash" ? 1.05 : 0.85;
    game.state = "transition";
    keys.clear();
    if (kind === "win") {
      game.flashColor = "210,255,180";
      game.flash = 0.6;
      flashToast("COURSE COMPLETE", 2);
      sound.tone(520, 0.18, 0.04, "square");
      sound.tone(780, 0.26, 0.04, "square", 0.17);
      sound.tone(1040, 0.36, 0.035, "square", 0.37);
    } else if (kind === "crash") {
      game.flashColor = "255,100,58";
      game.flash = 1;
      flashToast("", 0);
      sound.crash();
    } else {
      game.flashColor = "255,170,70";
      game.flash = 0.45;
      flashToast("TIME EXPIRED", 2);
      sound.warn("timeout");
    }
  }

  function completeTransition() {
    const kind = game.transitionKind;
    game.transitionKind = null;
    if (kind === "win") {
      game.state = "won";
      storeBest(game.lastResult.score, game.lastResult.time);
      mountScreen(winScreen());
    } else if (kind === "crash") {
      game.state = "crashed";
      mountScreen(crashScreen());
    } else {
      game.state = "timedout";
      mountScreen(timeoutScreen());
    }
  }

  function readInput() {
    return {
      pitchUp: keys.has("KeyS") || keys.has("ArrowDown"),
      pitchDown: keys.has("KeyW") || keys.has("ArrowUp"),
      rollLeft: keys.has("KeyA") || keys.has("ArrowLeft"),
      rollRight: keys.has("KeyD") || keys.has("ArrowRight"),
      yawLeft: keys.has("KeyQ"),
      yawRight: keys.has("KeyE"),
      throttleUp: keys.has("ShiftLeft") || keys.has("ShiftRight"),
      throttleDown: keys.has("ControlLeft") || keys.has("ControlRight"),
      afterburner: keys.has("Space"),
      airbrake: keys.has("KeyB"),
    };
  }

  function gateCrossed(a, gate) {
    const p0 = a.lastPos;
    const p1 = a.pos;
    const d0 = dot(sub(p0, gate.pos), gate.axis);
    const d1 = dot(sub(p1, gate.pos), gate.axis);
    if (!(d0 < 0 && d1 >= 0)) return false;
    const t = d0 / (d0 - d1 || 1);
    const crossing = [
      lerp(p0[0], p1[0], t),
      lerp(p0[1], p1[1], t),
      lerp(p0[2], p1[2], t),
    ];
    const rel = sub(crossing, gate.pos);
    const along = mul(gate.axis, dot(rel, gate.axis));
    const lateral = sub(rel, along);
    return length(lateral) <= gate.radius * 0.96;
  }

  function stepSimulation(dt) {
    const mission = game.mission;
    const a = game.aircraft;
    mission.elapsed += dt;
    mission.timeLeft = Math.max(0, mission.timeLeft - dt);
    updateAircraft(a, dt, readInput());
    mission.score = finalScore(mission);

    if (mission.gateIndex < mission.gates.length) {
      const gate = mission.gates[mission.gateIndex];
      if (gateCrossed(a, gate)) {
        gate.passed = true;
        mission.passed += 1;
        mission.bestGate = mission.gateIndex;
        mission.gateIndex += 1;
        mission.timeLeft += GATE_BONUS;
        mission.score = finalScore(mission);
        game.flashColor = "185,255,170";
        game.flash = 0.36;
        flashToast(`GATE ${pad(mission.passed, 2)} CLEAR // +${GATE_BONUS} SEC`, 2.3);
        sound.gate();
        if (mission.gateIndex >= mission.gates.length) {
          beginFinish("win");
          return;
        }
      }
    }

    if (a.crashed) {
      beginFinish("crash");
      return;
    }
    if (mission.timeLeft <= 0) {
      beginFinish("timeout");
    }
  }

  // ------------------------------------------------------------------- camera
  function attractCamera(t) {
    const travel = (t * 72) % 18500;
    const z = 9300 - travel;
    const x = riverX(z) + Math.sin(t * 0.14) * 90;
    const aheadZ = z - 900;
    const aheadX = riverX(aheadZ) + Math.sin(t * 0.14 + 0.7) * 140;
    const ground = terrainAt(x, z);
    const aheadGround = terrainAt(aheadX, aheadZ);
    const y = Math.max(ground + 235, 470) + Math.sin(t * 0.27) * 38;
    const aheadY = Math.max(aheadGround + 270, 500);
    const forward = norm([aheadX - x, (aheadY - y) * 0.22, aheadZ - z]);
    const base = qFromForwardUp(forward);
    const bank = Math.sin(t * 0.095) * 0.13 + Math.sin(t * 0.037) * 0.05;
    const ax = axisSet(base);
    const rq = qAxisAngle(ax.forward, bank);
    const right = qRotate(rq, ax.right);
    const up = qRotate(rq, ax.up);
    const fwd = qRotate(rq, ax.forward);
    return {
      eye: [x, y, z],
      forward: fwd,
      right,
      up,
      fov: 61 * D2R,
    };
  }

  function flightCamera() {
    const a = game.aircraft;
    const ax = axisSet(a.q);
    let q = a.q;
    const active = game.state === "flying" || game.state === "transition";
    if (active) {
      const t = game.time;
      let shake = 0.00055 + a.turbulence * 0.0012 + a.buffet * 0.004 + a.afterburnerVisual * 0.0007;
      if (a.mach > 0.92) shake += 0.0007;
      if (game.state === "transition" && game.transitionKind === "crash") {
        shake += Math.max(0, game.transitionTimer) * 0.02;
      }
      const sp = Math.sin(t * 26.3) * 0.6 + Math.sin(t * 41.7 + 1.3) * 0.4;
      const sr = Math.sin(t * 31.1 + 2.1) * 0.55 + Math.sin(t * 18.3 + 0.2) * 0.45;
      q = qMultiply(q, qAxisAngle([1, 0, 0], sp * shake));
      q = qMultiply(q, qAxisAngle([0, 1, 0], Math.sin(t * 17.3 + 0.4) * shake * 0.65));
      q = qMultiply(q, qAxisAngle([0, 0, 1], sr * shake * 1.1));
    }
    const qAx = axisSet(q);
    const eyeBase = add(a.pos, add(mul(ax.up, 1.26), mul(ax.forward, 2.05)));
    const groundHere = Math.max(0, terrainAt(eyeBase[0], eyeBase[2])) + 0.55;
    const eye = [eyeBase[0], Math.max(eyeBase[1], groundHere), eyeBase[2]];
    const fov = (65.5 + clamp(a.speed / 380, 0, 1) * 2.8) * D2R;
    return { eye, forward: qAx.forward, right: qAx.right, up: qAx.up, fov };
  }

  function currentCamera() {
    return game.state === "attract" ? attractCamera(game.attractTime) : flightCamera();
  }

  // ------------------------------------------------------------ canvas / layout
  function resizePresentation() {
    cssW = Math.max(320, window.innerWidth);
    cssH = Math.max(240, window.innerHeight);
    dpr = Math.min(window.devicePixelRatio || 1, 2);
    cockpitCanvas.width = Math.floor(cssW * dpr);
    cockpitCanvas.height = Math.floor(cssH * dpr);
    cockpitCanvas.style.width = cssW + "px";
    cockpitCanvas.style.height = cssH + "px";
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    if (renderer) renderer.resize(cssW, cssH);
  }

  function layoutFor(W, H) {
    const s = clamp(Math.min(W / 1440, H / 810), 1.0, 1.35);
    const panelY = H * 0.702;
    const hudTop = H * 0.108;
    const hudW = Math.min(W * 0.46, 900 * s);
    const hudBottom = panelY - H * 0.012;
    const cx = W * 0.5;
    const hudL = cx - hudW / 2;
    const hudR = cx + hudW / 2;
    const hudCy = hudTop + (hudBottom - hudTop) * 0.445;
    const mfdW = Math.min(W * 0.145, 290 * s);
    const mfdH = mfdW * 0.69;
    const gap = W * 0.016;
    const mfdY = panelY + H * 0.047;
    const mfdLX = cx - gap / 2 - mfdW;
    const mfdRX = cx + gap / 2;
    const gaugeR = Math.min(W * 0.036, H * 0.052, 54 * s);
    const gaugeY = panelY + H * 0.145;
    return { s, panelY, hudTop, hudBottom, hudW, hudL, hudR, hudCy, cx, mfdW, mfdH, mfdLX, mfdRX, mfdY, gaugeR, gaugeY };
  }

  // --------------------------------------------------------------- map texture
  function buildMapCanvas() {
    const w = 240;
    const h = 180;
    const c = document.createElement("canvas");
    c.width = w;
    c.height = h;
    const g = c.getContext("2d");
    const img = g.createImageData(w, h);
    const b = previewMission.mapBounds;
    const data = img.data;
    let o = 0;
    for (let py = 0; py < h; py++) {
      const z = lerp(b.zMin, b.zMax, py / (h - 1));
      for (let px = 0; px < w; px++) {
        const x = lerp(b.xMin, b.xMax, px / (w - 1));
        const elev = terrainAt(x, z);
        let r = 3, gg = 13, bb = 10;
        if (elev < 12) {
          r = 2; gg = 13; bb = 18;
        } else {
          const t = clamp(elev / 2500, 0, 1);
          r = 5 + t * 29;
          gg = 25 + t * 88;
          bb = 13 + t * 36;
          const band = Math.floor(elev / 170) % 2;
          if (band === 0) { gg += 8; bb += 3; }
          if (elev > 1900) { r += 18; gg += 5; bb += 5; }
          const riv = Math.abs(x - riverX(z));
          if (elev < 480 && riv < 80) { r = 5; gg = 35; bb = 38; }
        }
        data[o++] = r;
        data[o++] = gg;
        data[o++] = bb;
        data[o++] = 255;
      }
    }
    g.putImageData(img, 0, 0);
    return c;
  }

  // ------------------------------------------------------------------ map MFD
  function mapProject(p, sx, sy, sw, sh, b) {
    return [
      sx + ((p[0] - b.xMin) / (b.xMax - b.xMin)) * sw,
      sy + ((p[2] - b.zMin) / (b.zMax - b.zMin)) * sh,
    ];
  }

  function drawMFDMap(x, y, w, h, L) {
    const s = L.s;
    const m = game.mission;
    const b = m.mapBounds;
    const inset = 7 * s;
    const sx = x + inset, sy = y + inset, sw = w - inset * 2, sh = h - inset * 2;

    ctx.save();
    ctx.fillStyle = "#030906";
    ctx.fillRect(sx, sy, sw, sh);
    ctx.beginPath();
    ctx.rect(sx, sy, sw, sh);
    ctx.clip();

    ctx.globalAlpha = 0.94;
    ctx.drawImage(mapCanvas, 0, 0, mapCanvas.width, mapCanvas.height, sx, sy, sw, sh);
    ctx.globalAlpha = 1;

    // Grid
    ctx.strokeStyle = "rgba(126,225,150,0.11)";
    ctx.lineWidth = 1;
    for (let i = 1; i < 4; i++) {
      ctx.beginPath();
      ctx.moveTo(sx + (sw * i) / 4, sy);
      ctx.lineTo(sx + (sw * i) / 4, sy + sh);
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(sx, sy + (sh * i) / 4);
      ctx.lineTo(sx + sw, sy + (sh * i) / 4);
      ctx.stroke();
    }

    // Route
    ctx.strokeStyle = "rgba(255,192,91,0.58)";
    ctx.setLineDash([4 * s, 5 * s]);
    ctx.lineWidth = Math.max(1, 1.1 * s);
    ctx.beginPath();
    for (let i = 0; i < m.gates.length; i++) {
      const pt = mapProject(m.gates[i].pos, sx, sy, sw, sh, b);
      if (i === 0) ctx.moveTo(pt[0], pt[1]);
      else ctx.lineTo(pt[0], pt[1]);
    }
    ctx.stroke();
    ctx.setLineDash([]);

    // Gates
    for (let i = 0; i < m.gates.length; i++) {
      const gate = m.gates[i];
      const pt = mapProject(gate.pos, sx, sy, sw, sh, b);
      const next = i === m.gateIndex;
      ctx.beginPath();
      ctx.arc(pt[0], pt[1], (next ? 4.2 : 2.5) * s, 0, TAU);
      ctx.fillStyle = gate.passed ? "rgba(86,206,116,0.52)" : next ? AMBER : "rgba(119,227,216,0.55)";
      ctx.fill();
      if (next) {
        ctx.strokeStyle = "rgba(255,192,91,0.8)";
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.arc(pt[0], pt[1], (7 + Math.sin(game.time * 5) * 2) * s, 0, TAU);
        ctx.stroke();
      }
    }

    // Ownship
    const ap = mapProject(game.aircraft.pos, sx, sy, sw, sh, b);
    ctx.save();
    ctx.translate(clamp(ap[0], sx + 8, sx + sw - 8), clamp(ap[1], sy + 8, sy + sh - 8));
    ctx.rotate(game.aircraft.heading * D2R);
    ctx.fillStyle = "#d8ff86";
    ctx.beginPath();
    ctx.moveTo(0, -6 * s);
    ctx.lineTo(4.4 * s, 4.5 * s);
    ctx.lineTo(0, 2.5 * s);
    ctx.lineTo(-4.4 * s, 4.5 * s);
    ctx.closePath();
    ctx.fill();
    ctx.restore();

    // Heading/N marks
    ctx.font = `${Math.max(7.2, 8.2 * s)}px Silkscreen, monospace`;
    ctx.fillStyle = "rgba(155,255,170,0.74)";
    ctx.textAlign = "left";
    ctx.textBaseline = "top";
    ctx.fillText("TAC", sx + 6 * s, sy + 5 * s);
    ctx.textAlign = "right";
    ctx.fillText("N", sx + sw - 7 * s, sy + 5 * s);
    ctx.textAlign = "left";
    ctx.fillText(`${pad(game.aircraft.heading, 3)}`, sx + 6 * s, sy + sh - 15 * s);
    ctx.fillText("KT", sx + 39 * s, sy + sh - 15 * s);
    ctx.fillStyle = "rgba(255,192,91,0.7)";
    ctx.textAlign = "right";
    ctx.fillText(`${pad(kt(game.aircraft.speed), 3)}KT`, sx + sw - 6 * s, sy + sh - 15 * s);

    // Phosphor scanlines + glass reflection
    ctx.fillStyle = "rgba(0,0,0,0.18)";
    for (let yy = sy; yy < sy + sh; yy += Math.max(2, 2.2 * s)) ctx.fillRect(sx, yy, sw, 1);
    const gl = ctx.createLinearGradient(sx, sy, sx + sw, sy + sh);
    gl.addColorStop(0, "rgba(220,255,220,0.075)");
    gl.addColorStop(0.4, "rgba(220,255,220,0)");
    gl.addColorStop(1, "rgba(220,255,220,0.025)");
    ctx.fillStyle = gl;
    ctx.fillRect(sx, sy, sw, sh);
    ctx.restore();
  }

  // ---------------------------------------------------------------- engine MFD
  function drawBar(x, y, w, h, value, color) {
    ctx.fillStyle = "rgba(141,255,167,0.10)";
    ctx.fillRect(x, y, w, h);
    ctx.fillStyle = color;
    ctx.fillRect(x, y, w * clamp(value, 0, 1), h);
    ctx.strokeStyle = "rgba(141,255,167,0.22)";
    ctx.lineWidth = 1;
    ctx.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1);
    const segs = Math.max(6, Math.floor(w / 12));
    ctx.fillStyle = "rgba(0,0,0,0.35)";
    for (let i = 1; i < segs; i++) ctx.fillRect(x + (w * i) / segs, y, 1, h);
  }

  function drawMFDEngine(x, y, w, h, L) {
    const a = game.aircraft;
    const s = L.s;
    const inset = 7 * s;
    const sx = x + inset, sy = y + inset, sw = w - inset * 2, sh = h - inset * 2;
    const fs = clamp(w * 0.048, 6.2, 12);
    ctx.save();
    ctx.beginPath();
    ctx.rect(sx, sy, sw, sh);
    ctx.clip();
    ctx.fillStyle = "#06100b";
    ctx.fillRect(sx, sy, sw, sh);
    ctx.font = `${fs}px Silkscreen, monospace`;
    ctx.textBaseline = "top";
    ctx.textAlign = "left";
    ctx.fillStyle = HUD;
    ctx.fillText("ENG SYS", sx + 9 * s, sy + 4 * s);
    ctx.textAlign = "right";
    ctx.fillStyle = HUD_DIM;
    ctx.fillText("F110-GE-129", sx + sw - 9 * s, sy + 4 * s);

    const labelX = sx + 9 * s;
    const valueX = sx + sw - 9 * s;
    const dividerY = sy + Math.max(fs * 1.9, sh * 0.18);
    ctx.fillStyle = "rgba(169,255,178,0.18)";
    ctx.fillRect(sx + 8 * s, dividerY, sw - 16 * s, 1);

    const barH = Math.max(2.5, 4 * s);
    const row = sy + sh * 0.26;
    ctx.textAlign = "left";
    ctx.fillStyle = HUD_DIM;
    ctx.fillText("THR", labelX, row);
    ctx.textAlign = "right";
    ctx.fillStyle = a.ab ? AMBER : HUD;
    ctx.fillText(`${pad(a.throttle * 100, 3)}%`, valueX, row);
    drawBar(labelX, row + fs * 1.05, sw - 18 * s, barH, a.throttle, a.ab ? AMBER : "rgba(155,255,170,0.76)");

    const row2 = sy + sh * 0.44;
    ctx.textAlign = "left";
    ctx.fillStyle = HUD_DIM;
    ctx.fillText("RPM", labelX, row2);
    ctx.textAlign = "right";
    ctx.fillStyle = HUD;
    ctx.fillText(`${pad(61 + a.throttle * 37, 3)}%`, valueX, row2);
    drawBar(labelX, row2 + fs * 1.0, sw - 18 * s, barH, 0.61 + a.throttle * 0.37, "rgba(155,255,170,0.52)");

    const row3 = sy + sh * 0.62;
    ctx.textAlign = "left";
    ctx.fillStyle = HUD_DIM;
    ctx.fillText("NOZ", labelX, row3);
    ctx.textAlign = "right";
    ctx.fillStyle = a.ab ? AMBER : HUD;
    ctx.fillText(`${pad(a.ab ? 94 : 38 + a.throttle * 22, 2)}`, valueX, row3);

    const fuelY = sy + sh * 0.76;
    ctx.textAlign = "left";
    ctx.fillStyle = HUD_DIM;
    ctx.fillText("FUEL", labelX, fuelY);
    ctx.textAlign = "right";
    ctx.fillStyle = a.fuel < 500 ? RED : HUD;
    ctx.fillText(`${pad(a.fuel * 2.20462, 4)} LB`, valueX, fuelY);

    const by = sy + sh - fs - 2;
    if (by - (fuelY + fs) > fs * 0.6) {
      ctx.fillStyle = "rgba(169,255,178,0.18)";
      ctx.fillRect(sx + 8 * s, by - fs * 0.3, sw - 16 * s, 1);
    }
    ctx.textAlign = "left";
    ctx.fillStyle = a.ab ? AMBER : "rgba(169,255,178,0.62)";
    ctx.fillText(a.ab ? "AB ARMED" : "AB SAFE", labelX, by);
    ctx.textAlign = "right";
    ctx.fillStyle = a.stall > 0.1 ? RED : "rgba(169,255,178,0.78)";
    ctx.fillText(a.stall > 0.1 ? "STALL" : "FLCS NORM", valueX, by);

    // Phosphor texture
    ctx.fillStyle = "rgba(0,0,0,0.2)";
    for (let yy = sy; yy < sy + sh; yy += Math.max(2, 2.2 * s)) ctx.fillRect(sx, yy, sw, 1);
    const gl = ctx.createLinearGradient(sx, sy, sx + sw, sy + sh);
    gl.addColorStop(0, "rgba(220,255,220,0.075)");
    gl.addColorStop(0.45, "rgba(220,255,220,0)");
    gl.addColorStop(1, "rgba(220,255,220,0.022)");
    ctx.fillStyle = gl;
    ctx.fillRect(sx, sy, sw, sh);
    ctx.restore();
  }

  function drawMFDBezel(x, y, w, h, L) {
    const s = L.s;
    const padX = 11 * s;
    ctx.fillStyle = "#070a08";
    ctx.fillRect(x - padX, y - padX, w + padX * 2, h + padX * 2);
    ctx.strokeStyle = "rgba(173,197,170,0.16)";
    ctx.lineWidth = 1;
    ctx.strokeRect(x - padX + 0.5, y - padX + 0.5, w + padX * 2 - 1, h + padX * 2 - 1);
    ctx.strokeStyle = "rgba(0,0,0,0.8)";
    ctx.strokeRect(x - 2.5, y - 2.5, w + 5, h + 5);
    ctx.fillStyle = "#27312a";
    for (const [sx, sy] of [[x - padX + 4, y - padX + 4], [x + w + padX - 7, y - padX + 4], [x - padX + 4, y + h + padX - 7], [x + w + padX - 7, y + h + padX - 7]]) {
      ctx.beginPath();
      ctx.arc(sx, sy, 2.2 * s, 0, TAU);
      ctx.fill();
    }
    ctx.fillStyle = "rgba(169,255,178,0.48)";
    ctx.font = `${Math.max(7.4, 8 * s)}px Silkscreen, monospace`;
    ctx.textAlign = "left";
    ctx.textBaseline = "bottom";
    ctx.fillText(x === L.mfdLX ? "TAC MAP" : "ENGINE", x - padX + 2, y - padX - 4);
  }

  // -------------------------------------------------------------------- gauges
  function drawAnalogGauge(cx, cy, r, value, max, label, unit, L) {
    const s = L.s;
    ctx.save();
    ctx.translate(cx, cy);
    // Bezel
    const bg = ctx.createRadialGradient(-r * 0.25, -r * 0.3, r * 0.12, 0, 0, r);
    bg.addColorStop(0, "#252d26");
    bg.addColorStop(0.72, "#0d120f");
    bg.addColorStop(1, "#050706");
    ctx.fillStyle = bg;
    ctx.beginPath();
    ctx.arc(0, 0, r, 0, TAU);
    ctx.fill();
    ctx.strokeStyle = "rgba(195,210,190,0.22)";
    ctx.lineWidth = 1.2;
    ctx.stroke();
    ctx.strokeStyle = "rgba(0,0,0,0.8)";
    ctx.beginPath();
    ctx.arc(0, 0, r - 3 * s, 0, TAU);
    ctx.stroke();

    const a0 = Math.PI * 0.77;
    const a1 = Math.PI * 2.23;
    ctx.strokeStyle = "rgba(183,206,178,0.32)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.arc(0, 0, r * 0.78, a0, a1);
    ctx.stroke();
    ctx.strokeStyle = "rgba(255,109,95,0.35)";
    ctx.beginPath();
    ctx.arc(0, 0, r * 0.78, a1 - 0.28, a1);
    ctx.stroke();

    for (let i = 0; i <= 10; i++) {
      const a = a0 + (a1 - a0) * (i / 10);
      const major = i % 2 === 0;
      ctx.strokeStyle = major ? "rgba(208,226,199,0.7)" : "rgba(180,200,175,0.33)";
      ctx.lineWidth = major ? 1.4 : 0.8;
      ctx.beginPath();
      ctx.moveTo(Math.cos(a) * r * (major ? 0.68 : 0.72), Math.sin(a) * r * (major ? 0.68 : 0.72));
      ctx.lineTo(Math.cos(a) * r * 0.84, Math.sin(a) * r * 0.84);
      ctx.stroke();
    }

    const frac = clamp(value / max, 0, 1);
    const na = a0 + (a1 - a0) * frac;
    ctx.strokeStyle = "#d8ff86";
    ctx.shadowColor = "rgba(216,255,134,0.6)";
    ctx.shadowBlur = 6;
    ctx.lineWidth = 1.7;
    ctx.beginPath();
    ctx.moveTo(-Math.cos(na) * r * 0.15, -Math.sin(na) * r * 0.15);
    ctx.lineTo(Math.cos(na) * r * 0.65, Math.sin(na) * r * 0.65);
    ctx.stroke();
    ctx.shadowBlur = 0;
    ctx.fillStyle = "#0b0e0c";
    ctx.beginPath();
    ctx.arc(0, 0, r * 0.12, 0, TAU);
    ctx.fill();
    ctx.strokeStyle = "rgba(210,230,200,0.4)";
    ctx.stroke();

    ctx.fillStyle = "rgba(196,218,187,0.64)";
    ctx.font = `${Math.max(6.4, 7.2 * s)}px Silkscreen, monospace`;
    ctx.textAlign = "center";
    ctx.textBaseline = "top";
    ctx.fillText(label, 0, r * 0.34);
    ctx.fillStyle = "rgba(169,255,178,0.56)";
    ctx.fillText(unit, 0, r * 0.34 + 8 * s);

    // Glass
    const gl = ctx.createLinearGradient(-r, -r, r * 0.4, r);
    gl.addColorStop(0, "rgba(230,255,230,0.10)");
    gl.addColorStop(0.35, "rgba(230,255,230,0.01)");
    gl.addColorStop(1, "rgba(230,255,230,0)");
    ctx.fillStyle = gl;
    ctx.beginPath();
    ctx.arc(0, 0, r - 3 * s, 0, TAU);
    ctx.fill();
    ctx.restore();
  }

  // --------------------------------------------------------------- canopy/panel
  function drawCanopy(W, H, L) {
    const side = W * 0.09;
    const pY = L.panelY;
    const lg = ctx.createLinearGradient(0, 0, side, 0);
    lg.addColorStop(0, "#040605");
    lg.addColorStop(0.55, "#101510");
    lg.addColorStop(0.86, "#20261f");
    lg.addColorStop(1, "#090c09");

    ctx.save();
    ctx.beginPath();
    ctx.moveTo(0, 0);
    ctx.lineTo(side * 0.96, 0);
    ctx.lineTo(side * 0.88, H * 0.15);
    ctx.bezierCurveTo(side * 0.79, H * 0.28, side * 0.70, H * 0.45, side * 0.70, H * 0.55);
    ctx.lineTo(side * 0.86, pY + H * 0.02);
    ctx.lineTo(0, pY + H * 0.02);
    ctx.closePath();
    ctx.fillStyle = lg;
    ctx.fill();
    ctx.strokeStyle = "rgba(187,208,179,0.12)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(side * 0.96, 0);
    ctx.lineTo(side * 0.88, H * 0.15);
    ctx.bezierCurveTo(side * 0.79, H * 0.28, side * 0.70, H * 0.45, side * 0.70, H * 0.55);
    ctx.stroke();

    ctx.beginPath();
    ctx.moveTo(W, 0);
    ctx.lineTo(W - side * 0.96, 0);
    ctx.lineTo(W - side * 0.88, H * 0.15);
    ctx.bezierCurveTo(W - side * 0.79, H * 0.28, W - side * 0.70, H * 0.45, W - side * 0.70, H * 0.55);
    ctx.lineTo(W - side * 0.86, pY + H * 0.02);
    ctx.lineTo(W, pY + H * 0.02);
    ctx.closePath();
    const rg = ctx.createLinearGradient(W, 0, W - side, 0);
    rg.addColorStop(0, "#040605");
    rg.addColorStop(0.55, "#101510");
    rg.addColorStop(0.86, "#20261f");
    rg.addColorStop(1, "#090c09");
    ctx.fillStyle = rg;
    ctx.fill();
    ctx.strokeStyle = "rgba(187,208,179,0.12)";
    ctx.beginPath();
    ctx.moveTo(W - side * 0.96, 0);
    ctx.lineTo(W - side * 0.88, H * 0.15);
    ctx.bezierCurveTo(W - side * 0.79, H * 0.28, W - side * 0.70, H * 0.45, W - side * 0.70, H * 0.55);
    ctx.stroke();

    // Side-rail bolts and ribs
    for (let i = 0; i < 5; i++) {
      const y = H * (0.10 + i * 0.105);
      const x = side * (0.83 - i * 0.018);
      ctx.fillStyle = "rgba(145,160,140,0.22)";
      ctx.beginPath();
      ctx.arc(x, y, Math.max(1.5, H * 0.0025), 0, TAU);
      ctx.fill();
      ctx.beginPath();
      ctx.arc(W - x, y, Math.max(1.5, H * 0.0025), 0, TAU);
      ctx.fill();
    }

    // Thin overhead canopy rim
    ctx.strokeStyle = "rgba(6,8,6,0.96)";
    ctx.lineWidth = Math.max(3, H * 0.012);
    ctx.beginPath();
    ctx.moveTo(0, H * 0.033);
    ctx.bezierCurveTo(W * 0.24, H * 0.017, W * 0.76, H * 0.017, W, H * 0.033);
    ctx.stroke();
    ctx.strokeStyle = "rgba(190,210,180,0.10)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, H * 0.041);
    ctx.bezierCurveTo(W * 0.24, H * 0.025, W * 0.76, H * 0.025, W, H * 0.041);
    ctx.stroke();

    // Canopy glass reflection bands
    ctx.globalCompositeOperation = "screen";
    const sheen = ctx.createLinearGradient(0, 0, W * 0.5, H * 0.7);
    sheen.addColorStop(0, "rgba(205,230,215,0.065)");
    sheen.addColorStop(0.16, "rgba(205,230,215,0.005)");
    sheen.addColorStop(0.5, "rgba(205,230,215,0)");
    ctx.fillStyle = sheen;
    ctx.beginPath();
    ctx.moveTo(0, H * 0.02);
    ctx.lineTo(W * 0.28, H * 0.02);
    ctx.lineTo(W * 0.13, H * 0.6);
    ctx.lineTo(0, H * 0.56);
    ctx.closePath();
    ctx.fill();
    ctx.globalCompositeOperation = "source-over";
    ctx.restore();
  }

  function drawPanelBase(W, H, L) {
    const s = L.s;
    const pY = L.panelY;
    const cx = L.cx;
    ctx.save();

    // Glare shield and dashboard lip
    const glare = ctx.createLinearGradient(0, pY - H * 0.05, 0, pY + H * 0.03);
    glare.addColorStop(0, "#060806");
    glare.addColorStop(0.6, "#151a14");
    glare.addColorStop(1, "#080a08");
    ctx.beginPath();
    ctx.moveTo(0, pY - H * 0.018);
    ctx.bezierCurveTo(W * 0.26, pY - H * 0.048, W * 0.74, pY - H * 0.048, W, pY - H * 0.018);
    ctx.lineTo(W, pY + H * 0.033);
    ctx.lineTo(0, pY + H * 0.033);
    ctx.closePath();
    ctx.fillStyle = glare;
    ctx.fill();
    ctx.strokeStyle = "rgba(190,205,178,0.11)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, pY - H * 0.018);
    ctx.bezierCurveTo(W * 0.26, pY - H * 0.048, W * 0.74, pY - H * 0.048, W, pY - H * 0.018);
    ctx.stroke();

    // Main panel
    const pg = ctx.createLinearGradient(0, pY, 0, H);
    pg.addColorStop(0, "#181d17");
    pg.addColorStop(0.35, "#111510");
    pg.addColorStop(1, "#070907");
    ctx.fillStyle = pg;
    ctx.fillRect(0, pY + H * 0.018, W, H - pY);

    // Side consoles
    ctx.fillStyle = "rgba(7,10,7,0.86)";
    ctx.beginPath();
    ctx.moveTo(0, pY + H * 0.08);
    ctx.lineTo(W * 0.19, pY + H * 0.08);
    ctx.lineTo(W * 0.16, H);
    ctx.lineTo(0, H);
    ctx.closePath();
    ctx.fill();
    ctx.beginPath();
    ctx.moveTo(W, pY + H * 0.08);
    ctx.lineTo(W * 0.81, pY + H * 0.08);
    ctx.lineTo(W * 0.84, H);
    ctx.lineTo(W, H);
    ctx.closePath();
    ctx.fill();

    // Panel seams and rivets
    ctx.strokeStyle = "rgba(160,180,155,0.07)";
    for (let i = 1; i < 6; i++) {
      ctx.beginPath();
      ctx.moveTo((W * i) / 6, pY + H * 0.036);
      ctx.lineTo((W * i) / 6 + (i < 3 ? -5 : 5), H);
      ctx.stroke();
    }
    ctx.strokeStyle = "rgba(0,0,0,0.5)";
    ctx.beginPath();
    ctx.moveTo(0, pY + H * 0.13);
    ctx.bezierCurveTo(W * 0.3, pY + H * 0.115, W * 0.7, pY + H * 0.115, W, pY + H * 0.13);
    ctx.stroke();

    for (let i = 0; i < 16; i++) {
      const x = W * (0.13 + (i % 8) * 0.105);
      const y = pY + H * (0.052 + Math.floor(i / 8) * 0.018);
      ctx.fillStyle = "rgba(156,171,150,0.13)";
      ctx.beginPath();
      ctx.arc(x, y, Math.max(1.2, 1.5 * s), 0, TAU);
      ctx.fill();
    }

    // Top center data strip
    ctx.font = `${Math.max(6.2, 7.2 * s)}px Silkscreen, monospace`;
    ctx.textBaseline = "middle";
    ctx.textAlign = "center";
    ctx.fillStyle = "rgba(169,255,178,0.4)";
    ctx.fillText("F-16C  //  FLCS NORM  //  MAIN BUS A", cx, pY + H * 0.034);

    // Side console switch blocks
    drawSwitchBlock(W * 0.025, pY + H * 0.16, W * 0.08, H * 0.10, ["IFF", "NAV", "TAC"], L);
    drawSwitchBlock(W * 0.895, pY + H * 0.16, W * 0.08, H * 0.10, ["ECM", "FLIR", "ARM"], L);
    ctx.restore();
  }

  function drawSwitchBlock(x, y, w, h, labels, L) {
    const s = L.s;
    ctx.save();
    ctx.font = `${Math.max(6.2, 7 * s)}px Silkscreen, monospace`;
    ctx.textBaseline = "middle";
    ctx.textAlign = "left";
    ctx.fillStyle = "rgba(169,255,178,0.34)";
    const cellH = h / labels.length;
    for (let i = 0; i < labels.length; i++) {
      ctx.fillText(labels[i], x, y + cellH * (i + 0.5));
      ctx.fillStyle = i === 0 ? "rgba(169,255,178,0.5)" : "rgba(72,88,72,0.7)";
      ctx.fillRect(x + w * 0.38, y + cellH * i + cellH * 0.28, w * 0.16, cellH * 0.40);
      ctx.strokeStyle = "rgba(0,0,0,0.55)";
      ctx.strokeRect(x + w * 0.38, y + cellH * i + cellH * 0.28, w * 0.16, cellH * 0.40);
      ctx.fillStyle = "rgba(169,255,178,0.34)";
    }
    ctx.restore();
  }

  function drawThrottle(W, H, L) {
    const a = game.aircraft;
    const s = L.s;
    const x = W * 0.052;
    const top = L.panelY + H * 0.045;
    const bottom = L.panelY + H * 0.235;
    const ty = lerp(bottom - 7 * s, top + 7 * s, a.throttle);
    ctx.save();
    ctx.strokeStyle = "rgba(150,168,146,0.2)";
    ctx.lineWidth = 3 * s;
    ctx.beginPath();
    ctx.moveTo(x, top);
    ctx.lineTo(x, bottom);
    ctx.stroke();
    ctx.strokeStyle = "rgba(0,0,0,0.75)";
    ctx.lineWidth = 1.5;
    for (let i = 0; i <= 6; i++) {
      const y = top + (bottom - top) * (i / 6);
      ctx.beginPath();
      ctx.moveTo(x - 5 * s, y);
      ctx.lineTo(x + 5 * s, y);
      ctx.stroke();
    }
    ctx.fillStyle = "#1b211b";
    ctx.strokeStyle = "rgba(190,205,180,0.25)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.roundRect(x - 10 * s, ty - 7 * s, 20 * s, 14 * s, 3 * s);
    ctx.fill();
    ctx.stroke();
    ctx.fillStyle = a.ab ? AMBER : (a.throttle > 0.8 ? "#d8ff86" : HUD);
    ctx.fillRect(x - 6 * s, ty - 2 * s, 12 * s, 2 * s);
    ctx.fillStyle = "rgba(169,255,178,0.45)";
    ctx.font = `${Math.max(6, 6.8 * s)}px Silkscreen, monospace`;
    ctx.textAlign = "center";
    ctx.fillText("THR", x, bottom + 12 * s);
    ctx.fillStyle = a.ab ? AMBER : "rgba(169,255,178,0.58)";
    ctx.fillText(`${pad(a.throttle * 100, 3)}${a.ab ? " AB" : "%"}`, x, bottom + 22 * s);
    ctx.restore();
  }

  function drawStick(W, H, L) {
    const s = L.s;
    const x = W * 0.948;
    const y = L.panelY + H * 0.155;
    ctx.save();
    ctx.strokeStyle = "rgba(0,0,0,0.85)";
    ctx.lineWidth = 8 * s;
    ctx.lineCap = "round";
    ctx.beginPath();
    ctx.moveTo(x, H + 10 * s);
    ctx.lineTo(x - 9 * s, y + 38 * s);
    ctx.stroke();
    ctx.strokeStyle = "rgba(100,115,100,0.22)";
    ctx.lineWidth = 1;
    ctx.stroke();
    ctx.translate(x - 10 * s, y);
    ctx.rotate(-0.08);
    ctx.fillStyle = "#151a15";
    ctx.strokeStyle = "rgba(185,201,175,0.22)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.roundRect(-11 * s, -17 * s, 22 * s, 38 * s, 6 * s);
    ctx.fill();
    ctx.stroke();
    ctx.fillStyle = "rgba(120,138,120,0.32)";
    ctx.fillRect(-7 * s, -10 * s, 14 * s, 2 * s);
    ctx.fillStyle = RED;
    ctx.beginPath();
    ctx.arc(0, -12 * s, 2.2 * s, 0, TAU);
    ctx.fill();
    ctx.fillStyle = "rgba(169,255,178,0.38)";
    ctx.font = `${Math.max(5.8, 6.4 * s)}px Silkscreen, monospace`;
    ctx.textAlign = "center";
    ctx.fillText("FLCS", 0, 31 * s);
    ctx.restore();
  }

  function drawCautionPanel(W, H, L) {
    const a = game.aircraft;
    const m = game.mission;
    const s = L.s;
    const y = L.panelY + H * 0.25;
    const cx = L.cx;
    const lights = [
      { label: "ENG", on: true, color: "#8fff9d" },
      { label: "FLCS", on: true, color: "#8fff9d" },
      { label: "FUEL", on: a.fuel < 500, color: AMBER },
      { label: "HYD", on: false, color: AMBER },
      { label: "STALL", on: a.stall > 0.08, color: RED },
      { label: "G", on: a.gSmooth > 9, color: RED },
      { label: "LG", on: false, color: "#8fff9d" },
    ];
    const width = Math.min(W * 0.27, 420 * s);
    const start = cx - width / 2;
    const step = width / lights.length;
    ctx.save();
    ctx.fillStyle = "rgba(4,6,4,0.75)";
    ctx.fillRect(start - 10 * s, y - 14 * s, width + 20 * s, 37 * s);
    ctx.strokeStyle = "rgba(169,255,178,0.11)";
    ctx.strokeRect(start - 10 * s + 0.5, y - 14 * s + 0.5, width + 20 * s - 1, 37 * s - 1);
    ctx.font = `${Math.max(6, 6.6 * s)}px Silkscreen, monospace`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    for (let i = 0; i < lights.length; i++) {
      const lx = start + step * (i + 0.5);
      const lit = lights[i].on;
      ctx.fillStyle = lit ? lights[i].color : "rgba(69,80,67,0.36)";
      if (lit) {
        ctx.shadowColor = lights[i].color;
        ctx.shadowBlur = 8 * s;
      }
      ctx.fillRect(lx - 7 * s, y - 7 * s, 14 * s, 4 * s);
      ctx.shadowBlur = 0;
      ctx.fillStyle = lit ? "rgba(220,235,210,0.7)" : "rgba(110,125,105,0.4)";
      ctx.fillText(lights[i].label, lx, y + 5 * s);
    }
    ctx.fillStyle = "rgba(169,255,178,0.42)";
    ctx.textAlign = "left";
    ctx.font = `${Math.max(6, 6.4 * s)}px Silkscreen, monospace`;
    ctx.fillText(`COLD RIVER ${pad(m.gateIndex, 2)}/${pad(m.gates.length, 2)}`, start - 4 * s, y + 19 * s);
    ctx.textAlign = "right";
    ctx.fillText("MASTER CAUTION  //  OFF", start + width + 4 * s, y + 19 * s);
    ctx.restore();
  }

  // ---------------------------------------------------------------- HUD frame
  function drawHUDFrame(W, H, L) {
    const s = L.s;
    const l = L.hudL, r = L.hudR, top = L.hudTop, bottom = L.hudBottom;
    const post = Math.max(5, 7 * s);
    ctx.save();

    // Glass tint
    const glass = ctx.createLinearGradient(0, top, 0, bottom);
    glass.addColorStop(0, "rgba(72,132,92,0.055)");
    glass.addColorStop(0.45, "rgba(72,132,92,0.012)");
    glass.addColorStop(1, "rgba(72,132,92,0.03)");
    ctx.fillStyle = glass;
    ctx.fillRect(l, top, r - l, bottom - top);

    // Side frame posts
    const postGradient = ctx.createLinearGradient(l - post, 0, l + post, 0);
    postGradient.addColorStop(0, "#030403");
    postGradient.addColorStop(0.5, "#252a23");
    postGradient.addColorStop(1, "#080a08");
    ctx.fillStyle = postGradient;
    ctx.fillRect(l - post, top - post * 0.5, post, bottom - top + post);
    ctx.fillRect(r, top - post * 0.5, post, bottom - top + post);
    ctx.strokeStyle = "rgba(198,210,190,0.18)";
    ctx.lineWidth = 1;
    ctx.strokeRect(l - post + 0.5, top - post * 0.5 + 0.5, post - 1, bottom - top + post - 1);
    ctx.strokeRect(r + 0.5, top - post * 0.5 + 0.5, post - 1, bottom - top + post - 1);

    // Top and bottom rails
    ctx.fillStyle = "#080a08";
    ctx.fillRect(l - post * 1.6, top - post * 1.25, r - l + post * 3.2, post * 1.25);
    ctx.strokeStyle = "rgba(198,210,190,0.16)";
    ctx.strokeRect(l - post * 1.6 + 0.5, top - post * 1.25 + 0.5, r - l + post * 3.2 - 1, post * 1.25 - 1);
    ctx.fillStyle = "#060806";
    ctx.fillRect(l - post * 1.6, bottom - post * 0.25, r - l + post * 3.2, post * 1.3);
    ctx.fillStyle = "rgba(198,210,190,0.12)";
    ctx.fillRect(l - post, bottom + post * 0.15, r - l + post * 2, 1);

    // Mounting bolts
    for (const bx of [l - post * 0.5, r + post * 0.5]) {
      for (let i = 0; i < 5; i++) {
        const by = top + (bottom - top) * (0.08 + i * 0.2);
        ctx.fillStyle = "rgba(159,175,153,0.22)";
        ctx.beginPath();
        ctx.arc(bx, by, Math.max(1.2, 1.5 * s), 0, TAU);
        ctx.fill();
      }
    }

    // Fine inner glass edge
    ctx.strokeStyle = "rgba(169,255,178,0.14)";
    ctx.strokeRect(l + 1, top + 1, r - l - 2, bottom - top - 2);
    ctx.font = `${Math.max(6, 6.6 * s)}px Silkscreen, monospace`;
    ctx.fillStyle = "rgba(169,255,178,0.38)";
    ctx.textAlign = "left";
    ctx.textBaseline = "top";
    ctx.fillText("HUD // 1", l + 5 * s, top + 4 * s);
    ctx.restore();
  }

  // --------------------------------------------------------------- HUD helpers
  function hudFont(size, weight = 400) {
    return `${weight} ${Math.max(5.8, size)}px Silkscreen, monospace`;
  }

  function hText(text, x, y, size, color = HUD, align = "left", baseline = "middle", weight = 400, glow = 0) {
    ctx.font = hudFont(size, weight);
    ctx.textAlign = align;
    ctx.textBaseline = baseline;
    ctx.fillStyle = color;
    if (glow) {
      ctx.shadowColor = color;
      ctx.shadowBlur = glow;
    }
    ctx.fillText(text, x, y);
    if (glow) ctx.shadowBlur = 0;
  }

  function hLine(x1, y1, x2, y2, color = HUD, width = 1, dash = null) {
    ctx.strokeStyle = color;
    ctx.lineWidth = width;
    if (dash) ctx.setLineDash(dash);
    ctx.beginPath();
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y2);
    ctx.stroke();
    if (dash) ctx.setLineDash([]);
  }

  function projectPoint(p) {
    const cam = game.camera;
    if (!cam) return null;
    const rel = sub(p, cam.eye);
    const z = dot(rel, cam.forward);
    if (z <= 0.25) return null;
    const aspect = cssW / cssH;
    const tan = Math.tan(cam.fov / 2);
    const nx = dot(rel, cam.right) / (z * tan * aspect);
    const ny = dot(rel, cam.up) / (z * tan);
    return {
      x: (nx * 0.5 + 0.5) * cssW,
      y: (1 - (ny * 0.5 + 0.5)) * cssH,
      nx, ny, z, dist: length(rel),
    };
  }

  function drawPitchLadder(W, H, L) {
    const a = game.aircraft;
    const s = L.s;
    const ppd = Math.max(5.4, 7.1 * s);
    const half = (L.hudBottom - L.hudTop) * 0.43;
    const ladderW = Math.min(L.hudW * 0.33, 250 * s);
    const gap = ladderW * 0.11;
    ctx.save();
    ctx.beginPath();
    ctx.rect(L.hudL + 4, L.hudTop + 4, L.hudW - 8, L.hudBottom - L.hudTop - 8);
    ctx.clip();
    ctx.translate(L.cx, L.hudCy);
    ctx.rotate(-a.bank * D2R);
    for (let deg = -70; deg <= 70; deg += 10) {
      const y = (a.pitch - deg) * ppd;
      if (Math.abs(y) > half) continue;
      const major = deg === 0;
      const w = major ? ladderW * 0.57 : ladderW * (deg % 20 === 0 ? 0.44 : 0.34);
      const alpha = major ? 0.92 : 0.58;
      const color = `rgba(169,255,178,${alpha})`;
      const dash = deg < 0 ? [6 * s, 5 * s] : null;
      hLine(-w, y, -gap, y, color, major ? 1.5 : 1, dash);
      hLine(gap, y, w, y, color, major ? 1.5 : 1, dash);
      if (!major) {
        hLine(-w, y, -w, y + 3 * s, color, 1);
        hLine(w, y, w, y + 3 * s, color, 1);
      }
      if (deg % 20 === 0 && !major) {
        hText(`${Math.abs(deg)}`, w + 7 * s, y, 7.1 * s, HUD_DIM, "left");
        hText(`${Math.abs(deg)}`, -w - 7 * s, y, 7.1 * s, HUD_DIM, "right");
      }
    }
    ctx.restore();
  }

  function drawSpeedTape(L) {
    const a = game.aircraft;
    const s = L.s;
    const x = L.hudL + L.hudW * 0.135;
    const center = L.hudCy;
    const step = 50;
    const px = 22 * s;
    const value = kt(a.speed);
    const top = center - px * 4.4;
    const bottom = center + px * 4.4;
    ctx.save();
    hLine(x, top, x, bottom, HUD_DIM, 1);
    const base = Math.floor(value / step) * step;
    for (let i = -5; i <= 5; i++) {
      const v = base + i * step;
      if (v < 0) continue;
      const y = center - ((v - value) / step) * px;
      if (y < top - 2 || y > bottom + 2) continue;
      const major = v % 100 === 0;
      hLine(x, y, x + (major ? 10 : 6) * s, y, major ? HUD_SOFT : HUD_DIM, major ? 1.2 : 1);
      if (major) hText(`${v}`, x - 7 * s, y, 7.1 * s, HUD_DIM, "right");
    }
    // bug / readout
    const boxW = 61 * s, boxH = 23 * s;
    ctx.fillStyle = "rgba(0,8,4,0.56)";
    ctx.fillRect(x - boxW / 2, center - boxH / 2, boxW, boxH);
    ctx.strokeStyle = HUD_SOFT;
    ctx.strokeRect(x - boxW / 2 + 0.5, center - boxH / 2 + 0.5, boxW - 1, boxH - 1);
    hText(pad(value, 3), x, center + 1, 15 * s, HUD, "center", "middle", 700, 5 * s);
    hText("KCAS", x, top - 12 * s, 6.9 * s, HUD_DIM, "center");
    ctx.restore();
  }

  function drawAltTape(L) {
    const a = game.aircraft;
    const s = L.s;
    const x = L.hudR - L.hudW * 0.135;
    const center = L.hudCy;
    const step = 500;
    const px = 22 * s;
    const value = ft(a.pos[1]);
    const top = center - px * 4.4;
    const bottom = center + px * 4.4;
    ctx.save();
    hLine(x, top, x, bottom, HUD_DIM, 1);
    const base = Math.floor(value / step) * step;
    for (let i = -5; i <= 5; i++) {
      const v = base + i * step;
      const y = center - ((v - value) / step) * px;
      if (y < top - 2 || y > bottom + 2) continue;
      const major = v % 1000 === 0;
      hLine(x, y, x - (major ? 10 : 6) * s, y, major ? HUD_SOFT : HUD_DIM, major ? 1.2 : 1);
      if (major) hText(`${v / 1000}`, x + 7 * s, y, 7.1 * s, HUD_DIM, "left");
    }
    const boxW = 68 * s, boxH = 23 * s;
    ctx.fillStyle = "rgba(0,8,4,0.56)";
    ctx.fillRect(x - boxW / 2, center - boxH / 2, boxW, boxH);
    ctx.strokeStyle = HUD_SOFT;
    ctx.strokeRect(x - boxW / 2 + 0.5, center - boxH / 2 + 0.5, boxW - 1, boxH - 1);
    hText(pad(value, 5), x, center + 1, 15 * s, HUD, "center", "middle", 700, 5 * s);
    hText("FT MSL", x, top - 12 * s, 6.9 * s, HUD_DIM, "center");
    hText("THOU", x + boxW / 2 + 6 * s, center - 6 * s, 6.1 * s, HUD_DIM, "left");
    hText("SANDS", x + boxW / 2 + 6 * s, center + 5 * s, 6.1 * s, HUD_DIM, "left");
    ctx.restore();
  }

  function drawCompass(L) {
    const a = game.aircraft;
    const s = L.s;
    const y = L.hudTop + 21 * s;
    const halfW = Math.min(L.hudW * 0.24, 200 * s);
    const pxPerDeg = halfW / 45;
    const target = game.mission.gates[game.mission.gateIndex];
    const targetBearing = target ? wrap360(Math.atan2(target.pos[0] - a.pos[0], -(target.pos[2] - a.pos[2])) * R2D) : null;

    ctx.save();
    hLine(L.cx - halfW, y, L.cx + halfW, y, HUD_DIM, 1);
    for (let dd = -45; dd <= 45; dd += 5) {
      const targetDeg = Math.round((a.heading + dd) / 5) * 5;
      const diff = wrap180(targetDeg - a.heading);
      if (Math.abs(diff) > 45) continue;
      const tx = L.cx + diff * pxPerDeg;
      const major = targetDeg % 30 === 0;
      hLine(tx, y, tx, y + (major ? 6 : 3) * s, major ? HUD_SOFT : HUD_DIM, 1);
      if (major) {
        const label = targetDeg % 90 === 0
          ? ["N", "E", "S", "W"][wrap360(targetDeg) / 90 % 4]
          : `${wrap360(targetDeg) / 10 | 0}`.padStart(2, "0");
        hText(label, tx, y - 9 * s, 7.1 * s, HUD_SOFT, "center");
      }
    }
    if (targetBearing != null) {
      const diff = wrap180(targetBearing - a.heading);
      if (Math.abs(diff) <= 45) {
        const tx = L.cx + diff * pxPerDeg;
        ctx.fillStyle = AMBER;
        ctx.beginPath();
        ctx.moveTo(tx, y + 8 * s);
        ctx.lineTo(tx + 4 * s, y + 13 * s);
        ctx.lineTo(tx - 4 * s, y + 13 * s);
        ctx.closePath();
        ctx.fill();
      }
    }
    hLine(L.cx - 6 * s, y, L.cx, y - 5 * s, HUD, 1.3);
    hLine(L.cx + 6 * s, y, L.cx, y - 5 * s, HUD, 1.3);
    ctx.restore();
  }

  function drawTargetCue(L) {
    const a = game.aircraft;
    const m = game.mission;
    const s = L.s;
    const gate = m.gates[m.gateIndex];
    if (!gate) return;
    const rel = sub(gate.pos, a.pos);
    const dist = length(rel);
    const bearing = Math.atan2(rel[0], -rel[2]) * R2D;
    const diff = wrap180(bearing - a.heading);
    const elevation = Math.atan2(rel[1], Math.hypot(rel[0], rel[2])) * R2D;

    const lx = L.hudL + 17 * s;
    const ly = L.hudTop + 21 * s;
    hText(`GATE ${pad(m.gateIndex + 1, 2)}/${pad(m.gates.length, 2)}`, lx, ly, 8.8 * s, AMBER, "left", "middle", 700);
    hText(`${(dist / 1000).toFixed(1)} KM`, lx, ly + 14 * s, 8.6 * s, HUD, "left", "middle");
    hText(`${diff >= 0 ? "R" : "L"} ${pad(Math.abs(diff), 2)}°  ${elevation >= 0 ? "+" : "−"}${pad(Math.abs(elevation), 2)}°`, lx, ly + 27 * s, 7.4 * s, HUD_SOFT, "left", "middle");

    const p = projectPoint(gate.pos);
    if (p && p.x > L.hudL + 8 && p.x < L.hudR - 8 && p.y > L.hudTop + 40 && p.y < L.hudBottom - 10) {
      const size = 8 * s + clamp(1 - dist / 5000, 0, 1) * 5 * s;
      ctx.save();
      ctx.strokeStyle = AMBER;
      ctx.lineWidth = 1.3;
      ctx.beginPath();
      ctx.moveTo(p.x, p.y - size);
      ctx.lineTo(p.x + size, p.y);
      ctx.lineTo(p.x, p.y + size);
      ctx.lineTo(p.x - size, p.y);
      ctx.closePath();
      ctx.stroke();
      ctx.strokeStyle = "rgba(255,192,91,0.35)";
      ctx.beginPath();
      ctx.arc(p.x, p.y, size * 1.75, 0, TAU);
      ctx.stroke();
      hText(`${(dist / 1000).toFixed(1)}`, p.x, p.y + size * 2.4, 7 * s, AMBER, "center");
      ctx.restore();
    } else {
      // Edge locator: point toward the target around the HUD rim.
      let dx = diff * D2R;
      let dy = -elevation * D2R;
      const mag = Math.hypot(dx, dy) || 1;
      dx /= mag; dy /= mag;
      const ex = L.hudL + L.hudW * (dx > 0 ? 0.91 : 0.09);
      const ey = L.hudTop + (L.hudBottom - L.hudTop) * clamp(0.5 + dy * 0.38, 0.14, 0.84);
      ctx.save();
      ctx.translate(ex, ey);
      ctx.rotate(Math.atan2(dy, dx));
      ctx.fillStyle = AMBER;
      ctx.beginPath();
      ctx.moveTo(0, 0);
      ctx.lineTo(-10 * s, -6 * s);
      ctx.lineTo(-10 * s, 6 * s);
      ctx.closePath();
      ctx.fill();
      ctx.restore();
    }
  }

  function drawWarnings(L) {
    const a = game.aircraft;
    const m = game.mission;
    const s = L.s;
    const agl = a.agl;
    const sinkRate = -a.vs;
    const timeToImpact = sinkRate > 1 ? agl / sinkRate : 99;
    const pullUp = agl < 260 && sinkRate > 7 && (timeToImpact < 6.2 || agl < 95);
    const stallWarn = a.stall > 0.06 || a.alphaDeg > 20.5 || a.speed < 92;
    const gWarn = a.gSmooth > 9.15;
    const lowFuel = a.fuel < 400;
    const timeWarn = m.timeLeft < 20;
    const warns = [];
    if (pullUp) warns.push({ text: "PULL UP", color: RED });
    if (stallWarn) warns.push({ text: "STALL", color: RED });
    if (gWarn) warns.push({ text: "G LIMIT", color: AMBER });
    if (lowFuel) warns.push({ text: "BINGO", color: AMBER });
    if (timeWarn) warns.push({ text: "TIME CRITICAL", color: AMBER });

    if (game.toastTimer > 0) {
      const alpha = clamp(game.toastTimer, 0, 1);
      hText(game.toast, L.cx, L.hudTop + 58 * s, 9.5 * s, `rgba(216,255,134,${0.62 + alpha * 0.36})`, "center", "middle", 700, 6 * s);
    }

    if (warns.length) {
      const blink = Math.floor(game.time * 4.5) % 2 === 0;
      if (blink && game.state === "flying") {
        for (let i = 0; i < Math.min(2, warns.length); i++) {
          hText(warns[i].text, L.cx, L.hudCy + (138 + i * 16) * s, 11.5 * s, warns[i].color, "center", "middle", 700, 7 * s);
        }
      }
    } else {
      hText("FLCS NORM", L.cx, L.hudCy + 142 * s, 7.1 * s, HUD_DIM, "center");
    }

    // lower data line
    const by = L.hudCy + 105 * s;
    hLine(L.cx - 175 * s, by, L.cx + 175 * s, by, HUD_FAINT, 1);
    hText(`M ${a.mach.toFixed(2)}`, L.cx - 168 * s, by + 12 * s, 7.3 * s, HUD_SOFT);
    hText(`G ${a.gSmooth.toFixed(1)}`, L.cx - 110 * s, by + 12 * s, 7.3 * s, a.gSmooth > 8.5 ? AMBER : HUD_SOFT);
    hText(`AOA ${a.alphaDeg.toFixed(1)}`, L.cx - 41 * s, by + 12 * s, 7.3 * s, a.stall > 0.05 ? RED : HUD_SOFT);
    hText(`VS ${a.vs >= 0 ? "+" : "−"}${pad(Math.abs(fpm(a.vs)), 4)}`, L.cx + 48 * s, by + 12 * s, 7.3 * s, HUD_SOFT);
    hText(`AB ${a.ab ? "ON" : "OFF"}`, L.cx + 170 * s, by + 12 * s, 7.3 * s, a.ab ? AMBER : HUD_DIM, "right");
  }

  function drawMissionInfo(L) {
    const m = game.mission;
    const s = L.s;
    const rx = L.hudR - 16 * s;
    const ry = L.hudTop + 21 * s;
    hText(`CLK ${formatTime(m.timeLeft)}`, rx, ry, 8.8 * s, m.timeLeft < 20 ? RED : HUD, "right", "middle", 700);
    hText(`SCR ${pad(m.score, 5)}`, rx, ry + 14 * s, 8 * s, HUD_SOFT, "right");
    hText(`${pad(kt(game.aircraft.speed), 3)} KT`, rx, ry + 27 * s, 7.2 * s, HUD_DIM, "right");
  }

  function drawHUD(W, H, L) {
    const a = game.aircraft;
    const m = game.mission;
    if (!a || !m) return;
    const s = L.s;
    ctx.save();
    ctx.beginPath();
    ctx.rect(L.hudL, L.hudTop, L.hudW, L.hudBottom - L.hudTop);
    ctx.clip();
    ctx.lineCap = "butt";
    ctx.lineJoin = "miter";

    drawPitchLadder(W, H, L);
    drawCompass(L);
    drawTargetCue(L);
    drawMissionInfo(L);
    drawSpeedTape(L);
    drawAltTape(L);
    drawWarnings(L);

    // Flight path marker
    const speed = length(a.vel);
    if (speed > 4) {
      const fpmPoint = add(a.pos, mul(norm(a.vel), 700));
      const pp = projectPoint(fpmPoint);
      if (pp && pp.x > L.hudL + 3 && pp.x < L.hudR - 3 && pp.y > L.hudTop + 32 && pp.y < L.hudBottom - 3) {
        const fs = 6 * s;
        ctx.strokeStyle = HUD;
        ctx.lineWidth = 1.2;
        ctx.beginPath();
        ctx.arc(pp.x, pp.y, fs, 0, TAU);
        ctx.stroke();
        hLine(pp.x - fs - 6 * s, pp.y, pp.x - fs, pp.y, HUD, 1.2);
        hLine(pp.x + fs, pp.y, pp.x + fs + 6 * s, pp.y, HUD, 1.2);
        hLine(pp.x, pp.y - fs, pp.x, pp.y - fs - 5 * s, HUD, 1.2);
      }
    }

    // Fixed reticle
    const cx = L.cx, cy = L.hudCy;
    hLine(cx - 9 * s, cy, cx - 3 * s, cy, HUD_SOFT, 1);
    hLine(cx + 3 * s, cy, cx + 9 * s, cy, HUD_SOFT, 1);
    hLine(cx, cy - 9 * s, cx, cy - 3 * s, HUD_SOFT, 1);
    hLine(cx, cy + 3 * s, cx, cy + 9 * s, HUD_SOFT, 1);

    // Bank scale at the top of the ladder. The caret follows the actual bank.
    const bankY = L.hudCy - Math.min((L.hudBottom - L.hudTop) * 0.36, 150 * s);
    ctx.save();
    ctx.translate(cx, bankY);
    ctx.rotate(-a.bank * D2R);
    ctx.strokeStyle = HUD_DIM;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.arc(0, 0, 27 * s, Math.PI * 1.18, Math.PI * 1.82);
    ctx.stroke();
    for (let i = -3; i <= 3; i++) {
      const ang = -Math.PI / 2 + i * 0.13;
      const x1 = Math.cos(ang) * 25 * s, y1 = Math.sin(ang) * 25 * s;
      const x2 = Math.cos(ang) * 29 * s, y2 = Math.sin(ang) * 29 * s;
      hLine(x1, y1, x2, y2, i === 0 ? HUD : HUD_DIM, 1);
    }
    ctx.fillStyle = HUD;
    ctx.beginPath();
    ctx.moveTo(0, -30 * s);
    ctx.lineTo(-4 * s, -36 * s);
    ctx.lineTo(4 * s, -36 * s);
    ctx.closePath();
    ctx.fill();
    ctx.restore();

    // Corner brackets
    const inset = 10 * s;
    const bl = 15 * s;
    ctx.strokeStyle = HUD_DIM;
    ctx.lineWidth = 1;
    for (const [x, y, dx, dy] of [
      [L.hudL + inset, L.hudTop + inset, 1, 1],
      [L.hudR - inset, L.hudTop + inset, -1, 1],
      [L.hudL + inset, L.hudBottom - inset, 1, -1],
      [L.hudR - inset, L.hudBottom - inset, -1, -1],
    ]) {
      ctx.beginPath();
      ctx.moveTo(x + dx * bl, y);
      ctx.lineTo(x, y);
      ctx.lineTo(x, y + dy * bl);
      ctx.stroke();
    }

    ctx.restore();
  }

  // ------------------------------------------------------------------ effects
  function drawEffects(W, H) {
    const a = game.aircraft;
    if (!a) return;
    if (game.state === "flying" || game.state === "paused" || game.state === "transition") {
      const g = clamp((a.gSmooth - 7.4) / 2.7, 0, 1);
      if (g > 0.01) {
        const grad = ctx.createRadialGradient(W * 0.5, H * 0.42, H * 0.16, W * 0.5, H * 0.42, H * 0.75);
        grad.addColorStop(0, "rgba(38,0,0,0)");
        grad.addColorStop(0.55, `rgba(64,0,0,${g * 0.12})`);
        grad.addColorStop(1, `rgba(55,0,0,${g * 0.52})`);
        ctx.fillStyle = grad;
        ctx.fillRect(0, 0, W, H);
      }
    }
    if (game.flash > 0.005) {
      ctx.fillStyle = `rgba(${game.flashColor},${game.flash * 0.34})`;
      ctx.fillRect(0, 0, W, H);
    }
  }

  // ---------------------------------------------------------------- composition
  function drawCockpit(W, H) {
    const L = layoutFor(W, H);
    ctx.clearRect(0, 0, W, H);
    if (game.state === "attract" || !game.aircraft || !game.mission) return;

    drawCanopy(W, H, L);
    drawPanelBase(W, H, L);

    drawMFDBezel(L.mfdLX, L.mfdY, L.mfdW, L.mfdH, L);
    drawMFDBezel(L.mfdRX, L.mfdY, L.mfdW, L.mfdH, L);
    drawMFDMap(L.mfdLX, L.mfdY, L.mfdW, L.mfdH, L);
    drawMFDEngine(L.mfdRX, L.mfdY, L.mfdW, L.mfdH, L);

    drawAnalogGauge(W * 0.135, L.gaugeY, L.gaugeR, kt(game.aircraft.speed), 800, "STBY", "KTS", L);
    drawAnalogGauge(W * 0.865, L.gaugeY, L.gaugeR, ft(game.aircraft.pos[1]), 50000, "STBY", "FT", L);
    drawSwitchBlock(W * 0.025, L.panelY + H * 0.16, W * 0.075, H * 0.10, ["IFF", "NAV", "TAC"], L);
    drawSwitchBlock(W * 0.9, L.panelY + H * 0.16, W * 0.075, H * 0.10, ["ECM", "FLIR", "ARM"], L);

    drawThrottle(W, H, L);
    drawStick(W, H, L);
    drawCautionPanel(W, H, L);

    drawHUDFrame(W, H, L);
    drawHUD(W, H, L);
    drawEffects(W, H);
  }

  // ------------------------------------------------------------------ warnings
  function warningInfo() {
    const a = game.aircraft;
    if (!a) return { pullUp: false, stall: false };
    const sink = -a.vs;
    const timeToImpact = sink > 1 ? a.agl / sink : 99;
    const pullUp = a.agl < 260 && sink > 7 && (timeToImpact < 6.2 || a.agl < 95);
    const stall = a.stall > 0.06 || a.alphaDeg > 20.5 || a.speed < 92;
    return { pullUp, stall };
  }

  // --------------------------------------------------------------------- input
  const gameKeys = new Set([
    "KeyW", "KeyA", "KeyS", "KeyD", "KeyQ", "KeyE", "KeyB",
    "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Space",
    "ShiftLeft", "ShiftRight", "ControlLeft", "ControlRight",
  ]);

  function toggleMute() {
    sound.init();
    game.muted = sound.toggleMute();
    if (game.state === "flying" || game.state === "paused" || game.state === "transition") {
      flashToast(game.muted ? "AUDIO MUTED" : "AUDIO ON", 1.4);
    }
    if (game.state === "attract") mountScreen(startScreen());
    else if (game.state === "paused") mountScreen(pauseScreen());
  }

  window.addEventListener("keydown", (event) => {
    if (gameKeys.has(event.code)) event.preventDefault();
    if (event.code === "KeyM" && !event.repeat) {
      toggleMute();
      return;
    }
    if (event.code === "KeyP") {
      if (game.state === "flying") pauseMission();
      else if (game.state === "paused") { sound.init(); resumeMission(); }
      return;
    }
    if (event.code === "Escape") {
      if (game.state === "flying") pauseMission();
      else if (game.state === "paused") { sound.init(); resumeMission(); }
      else if (game.state === "won" || game.state === "crashed" || game.state === "timedout") goMenu();
      return;
    }
    if (event.code === "Enter") {
      if (game.state === "attract") { sound.init(); startMission(true); }
      else if (game.state === "paused") { sound.init(); resumeMission(); }
      else if (game.state === "won" || game.state === "crashed" || game.state === "timedout") { sound.init(); startMission(true); }
      return;
    }
    if (event.code === "KeyR" && !event.repeat) {
      if (game.state === "paused" || game.state === "won" || game.state === "crashed" || game.state === "timedout") {
        sound.init();
        startMission(true);
      }
      return;
    }
    if (event.repeat) return;
    if (game.state === "flying") keys.add(event.code);
  });

  window.addEventListener("keyup", (event) => {
    keys.delete(event.code);
  });

  window.addEventListener("blur", () => {
    keys.clear();
    if (game.state === "flying") pauseMission();
  });

  document.addEventListener("visibilitychange", () => {
    if (document.hidden && game.state === "flying") pauseMission();
  });

  ui.addEventListener("click", (event) => {
    const button = event.target.closest("[data-action]");
    if (!button) return;
    action(button.dataset.action);
  });

  // ----------------------------------------------------------------- main loop
  function updateCamera() {
    game.camera = currentCamera();
  }

  function updateWorldAndCockpit(frameDt) {
    if (!renderer) return;
    updateCamera();
    renderer.render(game.camera, { time: game.time, mission: game.mission });
    drawCockpit(cssW, cssH);
    if (game.aircraft) sound.update(game.aircraft, game.state === "flying");
  }

  function tick(now) {
    requestAnimationFrame(tick);
    const frameDt = Math.min(Math.max((now - lastFrame) / 1000, 0), 0.05);
    lastFrame = now;
    game.time += frameDt;
    game.toastTimer = Math.max(0, game.toastTimer - frameDt);
    game.flash = Math.max(0, game.flash - frameDt * 2.4);

    if (game.state === "attract") {
      game.attractTime += frameDt;
      updateCamera();
      renderer.render(game.camera, { time: game.time, mission: previewMission });
      drawCockpit(cssW, cssH);
      return;
    }

    if (game.state === "flying") {
      game.accumulator = Math.min(game.accumulator + frameDt, 0.12);
      let steps = 0;
      while (game.accumulator >= 1 / 120 && steps < 8 && game.state === "flying") {
        stepSimulation(1 / 120);
        game.accumulator -= 1 / 120;
        steps++;
      }
      if (steps === 8) game.accumulator = 0;

      const warns = warningInfo();
      game.warnClock -= frameDt;
      if (game.warnClock <= 0 && (warns.pullUp || warns.stall)) {
        sound.warn(warns.pullUp ? "pullup" : "stall");
        game.warnClock = warns.pullUp ? 0.8 : 1.5;
      }
    } else if (game.state === "transition") {
      game.transitionTimer -= frameDt;
      if (game.transitionTimer <= 0) completeTransition();
    }

    updateWorldAndCockpit(frameDt);
    if (game.state === "flying") {
      if (game.toastTimer > 0) {
        // Toast is already drawn by the HUD; nothing else needed here.
      }
    }
  }

  // --------------------------------------------------------------------- boot
  function showFatal(error) {
    console.error(error);
    if (boot) boot.classList.add("ready");
    document.getElementById("app").insertAdjacentHTML(
      "beforeend",
      `<div class="error-card"><div class="inside"><h2>FLCS FAULT</h2>
        <p>The cockpit failed to initialize. A WebGL 2 capable browser is required.</p>
        <p style="color:#b78b82;word-break:break-word">${String(error && error.message ? error.message : error).replace(/[<>&]/g, "")}</p>
      </div></div>`
    );
  }

  async function init() {
    try {
      renderer = new Renderer(canvas);
      previewMission = buildMission();
      resizePresentation();
      mapCanvas = buildMapCanvas();
      try {
        await document.fonts.load("12px Silkscreen");
        await document.fonts.ready;
      } catch (err) { /* custom font may be blocked; system monospace still works */ }
      mountScreen(startScreen());
      setBody("menu");
      boot.classList.add("ready");
      if (boot) setTimeout(() => boot.remove(), 600);
      window.addEventListener("resize", resizePresentation);
      lastFrame = performance.now();
      const params = new URLSearchParams(window.location.search);
      if (params.has("autostart")) {
        startMission(false);
      }
      requestAnimationFrame(tick);
    } catch (err) {
      showFatal(err);
    }
  }

  window.__viper = {
    game,
    startMission,
    pauseMission,
    resumeMission,
    goMenu,
    renderer: () => renderer,
    mission: () => game.mission,
    aircraft: () => game.aircraft,
    setKeys: (codes) => { keys.clear(); for (const c of codes) keys.add(c); },
    clearKeys: () => keys.clear(),
    snapshot: () => ({
      state: game.state,
      mission: game.mission,
      aircraft: game.aircraft,
      camera: game.camera,
      time: game.time,
    }),
  };

  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", init);
  else init();
})();
