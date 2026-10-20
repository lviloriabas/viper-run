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
  const formatClock = (sec) => {
    const s = Math.max(0, Math.floor(sec));
    return `${pad(Math.floor(s / 3600) % 24, 2)}:${pad(Math.floor((s % 3600) / 60), 2)}:${pad(s % 60, 2)}`;
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
  const SETTINGS_KEY = "viperRun.settings.v1";
  const DEFAULT_SETTINGS = { sensitivity: 2.8, invertY: false, fov: 58 };
  const settings = loadSettings();

  const sound = new Sound();
  const keys = new Set();
  const ui = document.getElementById("ui");
  const boot = document.getElementById("boot");
  const canvas = document.getElementById("world");
  const cockpitCanvas = document.getElementById("cockpit");
  const ctx = cockpitCanvas.getContext("2d");

  let renderer = null;
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
    view: "cockpit",
    mouse: { x: 0, y: 0, locked: false, sensitivity: 0.0022 * settings.sensitivity },
    look: { yaw: 0, pitch: -12 * D2R },
    chaseLook: { yaw: 0, pitch: 0 },
    controlsOpen: false,
    controlsWasFlying: false,
    mfdPages: ["FCR", "ENG"],
    hands: { roll: 0, pitch: 0, throttle: 0.82, rudder: 0, brake: 0, ab: 0, grip: 0 },
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

  function loadSettings() {
    try {
      const s = JSON.parse(localStorage.getItem(SETTINGS_KEY) || "{}");
      return {
        sensitivity: Number.isFinite(s.sensitivity) ? clamp(s.sensitivity, .5, 5) : DEFAULT_SETTINGS.sensitivity,
        invertY: s.invertY === true,
        fov: Number.isFinite(s.fov) ? clamp(s.fov, 45, 72) : DEFAULT_SETTINGS.fov,
      };
    } catch (_) { return { ...DEFAULT_SETTINGS }; }
  }

  function applySettings(values) {
    if (Number.isFinite(values.sensitivity)) settings.sensitivity = clamp(values.sensitivity, .5, 5);
    if (typeof values.invertY === "boolean") settings.invertY = values.invertY;
    if (Number.isFinite(values.fov)) settings.fov = clamp(values.fov, 45, 72);
    game.mouse.sensitivity = .0022 * settings.sensitivity;
    game.mouse.x = 0; game.mouse.y = 0;
    try { localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings)); } catch (_) {}
  }

  function toggleControls(force) {
    const open = typeof force === "boolean" ? force : !game.controlsOpen;
    if (open === game.controlsOpen) return;
    const drawer = document.getElementById("flight-controls");
    game.controlsOpen = open;
    if (open) {
      game.controlsWasFlying = game.state === "flying";
      if (game.controlsWasFlying) {
        game.state = "paused"; keys.clear(); game.mouse.x = 0; game.mouse.y = 0;
        releaseMouseControl(); setBody("paused");
        if (sound.ctx) sound.ctx.suspend().catch(() => {});
      }
      drawer.innerHTML = `
        <div class="drawer-heading"><span>CONTROLS / CONFIG</span><button type="button" data-close-controls aria-label="Close controls">CLOSE <kbd>K</kbd></button></div>
        <p class="drawer-intro">${game.controlsWasFlying ? "FLIGHT PAUSED" : "FLIGHT CONTROLS"}</p>
        <dl class="control-list">
          <div><dt><kbd>S</kbd></dt><dd>Pitch up, pull</dd></div>
          <div><dt><kbd>W</kbd></dt><dd>Pitch down, push</dd></div>
          <div><dt><kbd>MOUSE</kbd></dt><dd>Pitch / yaw</dd></div>
          <div><dt><kbd>A</kbd> <kbd>D</kbd></dt><dd>Roll left / right</dd></div>
          <div><dt><kbd>Q</kbd> <kbd>E</kbd></dt><dd>Rudder left / right</dd></div>
          <div><dt><kbd>SHIFT</kbd> <kbd>CTRL</kbd></dt><dd>Throttle up / down</dd></div>
          <div><dt><kbd>SPACE</kbd></dt><dd>Afterburner</dd></div>
          <div><dt><kbd>H</kbd></dt><dd>Airbrake, hold</dd></div>
          <div><dt><kbd>ALT + MOUSE</kbd></dt><dd>Hold Alt to look around</dd></div>
          <div><dt><kbd>C</kbd></dt><dd>Center view / mouse</dd></div>
          <div><dt><kbd>V</kbd></dt><dd>Cockpit / chase</dd></div>
          <div><dt><kbd>F1</kbd> <kbd>F2</kbd></dt><dd>MFD pages</dd></div>
          <div><dt><kbd>P</kbd> <kbd>ESC</kbd></dt><dd>Pause / resume</dd></div>
          <div><dt><kbd>M</kbd></dt><dd>Audio on / off</dd></div>
        </dl>
        <div class="drawer-settings">
          <label for="mouse-sensitivity">Mouse sensitivity <output id="sensitivity-value">${settings.sensitivity.toFixed(1)}x</output></label>
          <input id="mouse-sensitivity" type="range" min="0.5" max="5" step="0.1" value="${settings.sensitivity}">
          <label class="invert-option"><input id="invert-y" type="checkbox" ${settings.invertY ? "checked" : ""}> Invert mouse vertical axis</label>
          <p>W and S always keep their labeled directions.</p>
          <label for="cockpit-fov">Cockpit field of view <output id="fov-value">${settings.fov}°</output></label>
          <input id="cockpit-fov" type="range" min="45" max="72" step="1" value="${settings.fov}">
          <button type="button" id="reset-settings">RESTORE DEFAULTS</button>
        </div><p class="drawer-footnote">Press K to close. Settings are saved on this browser.</p>`;
      drawer.hidden = false;
      drawer.querySelector("[data-close-controls]").onclick = () => toggleControls(false);
      drawer.querySelector("#mouse-sensitivity").oninput = event => {
        applySettings({ sensitivity: Number(event.target.value) });
        drawer.querySelector("#sensitivity-value").textContent = settings.sensitivity.toFixed(1) + "x";
      };
      drawer.querySelector("#invert-y").onchange = event => applySettings({ invertY: event.target.checked });
      drawer.querySelector("#cockpit-fov").oninput = event => {
        applySettings({ fov: Number(event.target.value) });
        drawer.querySelector("#fov-value").textContent = settings.fov + "°";
      };
      drawer.querySelector("#reset-settings").onclick = () => {
        applySettings(DEFAULT_SETTINGS); toggleControls(false); toggleControls(true);
      };
      drawer.querySelector("[data-close-controls]").focus({ preventScroll: true });
    } else {
      drawer.hidden = true;
      if (game.controlsWasFlying) {
        game.state = "flying"; game.accumulator = 0; lastFrame = performance.now(); setBody("playing");
        if (sound.ctx) sound.ctx.resume().catch(() => {});
      }
      game.controlsWasFlying = false;
      canvas.focus({ preventScroll: true });
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
            <span>// TACTICAL AIR COMMAND, 0715 ZULU</span>
            <div class="lights"><i class="lamp"></i><i class="lamp dim"></i><i class="lamp red"></i><span>FLCS ONLINE</span></div>
          </div>
          <div class="menu-grid">
            <div>
              <div class="kicker">F-16 VIPER, MODERN COCKPIT</div>
              <h1 class="title">VIPER<span><span class="slash">/</span>RUN</span></h1>
              <div class="title-rule">COLD RIVER GORGE, SECTOR 07</div>
              <p class="mission-copy">
                You are already airborne, twenty clicks from the border, with
                ten navigation gates strung down a black river gorge. Thread
                the valley. Stay under the ridgeline. Make the coast before the
                clock runs out. <strong>The jet is loaded. The weather is perfect. The mountain is not.</strong>
              </p>
              <div class="cta-row">
                <button class="btn" data-action="start" data-autofocus>ENGAGE <small>ENTER</small></button>
                <button class="btn ghost" data-action="mute">${sound.muted ? "AUDIO OFF" : "AUDIO ON"}</button>
                <button class="btn ghost" data-action="controls">CONTROLS / CONFIG <small>K</small></button>
              </div>
            </div>
            <aside class="briefing">
              <div class="briefing-label">Mission data // COLD RIVER</div>
              <div class="brief-row"><span class="k">AIRFRAME</span><span class="v"><strong>F-16C FIGHTING FALCON</strong><br>F110-GE-129 / FLCS NORM</span></div>
              <div class="brief-row"><span class="k">ROUTE</span><span class="v"><strong>10 GATES</strong>, RIVER GORGE, COASTAL EGRESS</span></div>
              <div class="brief-row"><span class="k">CLOCK</span><span class="v"><em>02:30</em> + 8 SEC PER GATE</span></div>
              <div class="brief-row"><span class="k">HAZARD</span><span class="v"><em>TERRAIN</em>, STALL, G-LIMIT</span></div>
              <div class="controls-block">
                <div class="controls-title"><span>Flight controls</span><span>Hybrid</span></div>
                <div class="keys">
                  <div><kbd>MOUSE</kbd> PITCH / YAW</div><div><kbd>ALT + MOUSE</kbd> HOLD TO LOOK AROUND</div><div><kbd>F1</kbd><kbd>F2</kbd> MFD PAGES</div>
                  <div><kbd>A</kbd><kbd>D</kbd> ROLL</div>
                  <div><kbd>W</kbd><kbd>S</kbd> BACKUP PITCH</div>
                  <div><kbd>Q</kbd><kbd>E</kbd> RUDDER</div>
                  <div><kbd>SHIFT</kbd><kbd>CTRL</kbd> THROTTLE</div>
                  <div><kbd>SPACE</kbd> AFTERBURNER</div>
                  <div><kbd>V</kbd> CHASE / COCKPIT</div>
                  <div><kbd>C</kbd> CENTER MOUSE</div>
                  <div><kbd>H</kbd> AIRBRAKE</div>
                  <div><kbd>P</kbd> PAUSE, <kbd>M</kbd> AUDIO</div>
                </div>
              </div>
            </aside>
          </div>
          <div class="menu-footer">
            <span class="best">${best}</span>
            <span>CLICK TO CAPTURE MOUSE, <kbd>V</kbd> CHASE VIEW, <kbd>P</kbd> PAUSE</span>
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
          <div class="status-kicker">// flight suspended, flcs hold</div>
          <h2 class="status-title">PAUSED</h2>
          <p class="status-sub">Your flight is frozen on the clock. Take a breath, check the route, and re-engage when ready.</p>
          <div class="stats">
            <div class="stat"><div class="k">CLEARED</div><div class="v green">${pad(m.passed, 2)} / ${pad(m.gates.length, 2)}</div></div>
            <div class="stat"><div class="k">TIME LEFT</div><div class="v amber">${formatTime(m.timeLeft)}</div></div>
            <div class="stat"><div class="k">AIRSPEED</div><div class="v">${pad(a.airspeedKt, 3)} KT</div></div>
            <div class="stat"><div class="k">ALTITUDE</div><div class="v">${pad(ft(a.pos[1]), 5)} FT</div></div>
          </div>
          <div class="result-actions">
            <button class="btn" data-action="resume" data-autofocus>RESUME <small>P</small></button>
            <button class="btn ghost" data-action="restart">RESTART RUN <small>R</small></button>
            <button class="btn ghost" data-action="menu">ABORT TO MENU</button>
            <button class="btn ghost" data-action="controls">CONTROLS / CONFIG <small>K</small></button>
          </div>
          <div class="small-print">P / ESC RESUME, R RESTART, M AUDIO ${sound.muted ? "OFF" : "ON"}</div>
        </div>
      </section>`;
  }

  function winScreen() {
    const r = game.lastResult;
    const rank = r.time < 95 ? "ACE" : r.time < 125 ? "VIPER" : r.time < 155 ? "QUALIFIED" : "CADET";
    return `
      <section class="screen">
        <div class="menu-shell panel-narrow">
          <div class="status-kicker">// route complete, cold river gorge</div>
          <h2 class="status-title win">COURSE<br> COMPLETE</h2>
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
          <div class="small-print">${bestLine()}, R RESTART, ESC MENU</div>
        </div>
      </section>`;
  }

  function crashScreen() {
    const r = game.lastResult;
    return `
      <section class="screen">
        <div class="menu-shell panel-narrow">
          <div class="status-kicker" style="color:var(--red)">// flight terminated, airframe lost</div>
          <h2 class="status-title fail">TERRAIN<br> WINS</h2>
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
          <div class="status-kicker" style="color:var(--amber)">// clock expired, run invalid</div>
          <h2 class="status-title fail" style="color:var(--amber)">RUN<br> EXPIRED</h2>
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
    if (game.controlsOpen && name !== "controls") return;
    if (name === "controls") {
      toggleControls();
    } else if (name === "start") {
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

  function setView(view, announce = true) {
    game.view = view === "chase" ? "chase" : "cockpit";
    if (announce) flashToast(game.view === "chase" ? "CHASE VIEW // V COCKPIT" : "COCKPIT VIEW // V CHASE", 1.8);
  }

  function toggleView() {
    if (game.state !== "flying" && game.state !== "transition") return;
    setView(game.view === "chase" ? "cockpit" : "chase");
  }

  function centerMouseStick(announce = true) {
    game.mouse.x = 0;
    game.mouse.y = 0;
    game.look.yaw = 0;
    game.look.pitch = -12 * D2R;
    game.chaseLook.yaw = 0; game.chaseLook.pitch = 0;
    if (announce) flashToast("MOUSE STICK CENTERED", 1.1);
  }

  function requestMouseControl() {
    if (game.state !== "flying" || document.pointerLockElement === canvas) return;
    try {
      const result = canvas.requestPointerLock({ unadjustedMovement: true });
      if (result && typeof result.catch === "function") {
        result.catch(() => {
          try {
            Promise.resolve(canvas.requestPointerLock()).catch(() => flashToast("MOUSE CAPTURE UNAVAILABLE // KEYBOARD", 2.2));
          } catch (_) { flashToast("MOUSE CAPTURE UNAVAILABLE // KEYBOARD", 2.2); }
        });
      }
    } catch (err) {
      try {
        canvas.requestPointerLock();
      } catch (err2) {
        flashToast("MOUSE CAPTURE UNAVAILABLE // KEYBOARD", 2.2);
      }
    }
  }

  function releaseMouseControl() {
    if (document.pointerLockElement === canvas) document.exitPointerLock();
  }

  function startMission(fromGesture = true) {
    if (game.controlsOpen) toggleControls(false);
    const mission = buildMission();
    game.mission = mission;
    game.aircraft = makeAircraft(mission);
    game.state = "flying";
    game.accumulator = 0;
    game.transitionTimer = 0;
    game.flash = 0;
    game.toastTimer = 0;
    game.hands.roll = 0;
    game.hands.pitch = 0;
    game.hands.throttle = mission.start.throttle;
    keys.clear();
    centerMouseStick(false);
    hideUI();
    setBody("playing");
    if (fromGesture && sound.ctx && sound.ctx.state === "suspended") sound.ctx.resume().catch(() => {});
    if (fromGesture) sound.init();
    flashToast("COLD RIVER // 10 GATES // CLICK TO CAPTURE MOUSE", 3.4);
    lastFrame = performance.now();
  }

  function pauseMission() {
    if (game.state !== "flying") return;
    game.state = "paused";
    keys.clear();
    centerMouseStick(false);
    releaseMouseControl();
    if (sound.ctx) sound.ctx.suspend().catch(() => {});
    mountScreen(pauseScreen());
  }

  function resumeMission() {
    if (game.controlsOpen) return;
    if (game.state !== "paused") return;
    game.state = "flying";
    keys.clear();
    centerMouseStick(false);
    hideUI();
    setBody("playing");
    if (sound.ctx) sound.ctx.resume().catch(() => {});
    flashToast("CLICK TO CAPTURE MOUSE // V CHASE", 1.8);
    lastFrame = performance.now();
    game.accumulator = 0;
  }

  function goMenu() {
    game.state = "attract";
    game.attractTime = 0;
    keys.clear();
    centerMouseStick(false);
    releaseMouseControl();
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
    centerMouseStick(false);
    releaseMouseControl();
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
    const mouse = game.mouse.locked ? game.mouse : { x: 0, y: 0 };
    const keyboardPitch = keys.has("KeyS") || keys.has("KeyW") || keys.has("ArrowDown") || keys.has("ArrowUp");
    return {
      pitchUp: keys.has("KeyS") || keys.has("ArrowDown"),
      pitchDown: keys.has("KeyW") || keys.has("ArrowUp"),
      rollLeft: keys.has("KeyA") || keys.has("ArrowLeft"),
      rollRight: keys.has("KeyD") || keys.has("ArrowRight"),
      yawLeft: keys.has("KeyQ"),
      yawRight: keys.has("KeyE"),
      // Mouse Y is pitch (down = pull) and mouse X is yaw, matching the
      // requested hybrid mouse/hands-on-stick layout. A/D remains roll.
      pitchAxis: keyboardPitch ? 0 : mouse.y * (settings.invertY ? -1 : 1),
      yawAxis: mouse.x,
      rollAxis: 0,
      throttleUp: keys.has("ShiftLeft") || keys.has("ShiftRight"),
      throttleDown: keys.has("ControlLeft") || keys.has("ControlRight"),
      afterburner: keys.has("Space"),
      airbrake: keys.has("KeyH"),
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
    q = qMultiply(q, qAxisAngle([0, 1, 0], -game.look.yaw));
    q = qMultiply(q, qAxisAngle([1, 0, 0], game.look.pitch));
    const qAx = axisSet(q);
    const eyeBase = add(a.pos, qRotate(a.q, window.__VIPER_COCKPIT.EYE));
    const groundHere = Math.max(0, terrainAt(eyeBase[0], eyeBase[2])) + 0.55;
    const eye = [eyeBase[0], Math.max(eyeBase[1], groundHere), eyeBase[2]];
    // Wide cockpit field of view. The HUD symbology is projected with this
    // same cone, so a wider view keeps the ladder and flight-path marker tied
    // to the real horizon instead of drifting as a decorative overlay.
    const fov = settings.fov * D2R;
    return { eye, forward: qAx.forward, right: qAx.right, up: qAx.up, fov };
  }

  function chaseCamera() {
    const a = game.aircraft;
    const ax = axisSet(a.q);
    const t = game.time;
    const speedFactor = clamp(a.speed / 380, 0, 1);
    const velocityDir = a.speed > 5 ? norm(a.vel) : ax.forward;
    const viewDir = norm(add(mul(ax.forward, 0.55), mul(velocityDir, 0.45)));
    let offset = add(mul(ax.back, 14.5 + speedFactor * 3), mul(ax.up, 4.2));
    offset = qRotate(qAxisAngle(ax.up, game.chaseLook.yaw), offset);
    offset = qRotate(qAxisAngle(ax.right, game.chaseLook.pitch), offset);
    const eye = add(a.pos, offset);
    const orbiting = Math.abs(game.chaseLook.yaw) + Math.abs(game.chaseLook.pitch) > .001;
    const look = orbiting ? add(a.pos, mul(ax.up, .35)) : add(a.pos, add(mul(viewDir, 10), mul(ax.up, 0.7)));

    // A partially stabilized up vector keeps the horizon readable during a
    // fast roll without making the chase view feel rigidly welded to the jet.
    const stableUp = norm(add(mul(ax.up, 0.70), [0, 0.30, 0]));
    let forward = norm(sub(look, eye));
    let right = norm(cross(forward, stableUp));
    if (length(right) < 1e-4) right = ax.right;
    let up = norm(cross(right, forward));

    const active = game.state === "flying" || game.state === "transition";
    if (active) {
      let shake = 0.00055 + a.turbulence * 0.0012 + a.buffet * 0.004 + a.afterburnerVisual * 0.0007;
      if (a.mach > 0.92) shake += 0.0007;
      if (game.state === "transition" && game.transitionKind === "crash") {
        shake += Math.max(0, game.transitionTimer) * 0.02;
      }
      const sx = Math.sin(t * 31.1) * 0.6 + Math.sin(t * 18.3 + 0.4) * 0.4;
      const sy = Math.sin(t * 26.3 + 1.7) * 0.6 + Math.sin(t * 41.7 + 0.3) * 0.4;
      const shakenEye = add(eye, add(mul(right, sx * shake * 210), mul(up, sy * shake * 210)));
      eye[0] = shakenEye[0];
      eye[1] = shakenEye[1];
      eye[2] = shakenEye[2];
    }

    const floor = Math.max(0, terrainAt(eye[0], eye[2])) + 2.4;
    if (eye[1] < floor) eye[1] = floor;
    forward = norm(sub(look, eye));
    right = norm(cross(forward, stableUp));
    if (length(right) < 1e-4) right = ax.right;
    up = norm(cross(right, forward));
    return {
      eye,
      forward,
      right,
      up,
      fov: (48 + speedFactor * 3) * D2R,
    };
  }

  function currentCamera() {
    if (game.state === "attract") return attractCamera(game.attractTime);
    return game.view === "chase" ? chaseCamera() : flightCamera();
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
    return { s: clamp(Math.min(W / 1280, H / 720), 0.75, 1.35) };
  }

  // --------------------------------------------------------------- HUD helpers
  function hudFont(size, weight = 400) {
    // The real combiner uses a thin stroke font. A clean monospace reads much
    // closer to that than the blocky terminal face used by the rest of the HUD
    // furniture, and stays legible at small sizes.
    return `${weight} ${Math.max(6, size)}px "DejaVu Sans Mono", "Consolas", "Liberation Mono", monospace`;
  }

  function hText(text, x, y, size, color = HUD, align = "left", baseline = "middle", weight = 400, glow = 0) {
    ctx.font = hudFont(size, weight);
    ctx.textAlign = align;
    ctx.textBaseline = baseline;
    ctx.fillStyle = color;
    ctx.fillText(text, x, y);
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

  function drawMissionOverlay(W, H, L) {
    const m = game.mission;
    const a = game.aircraft;
    if (!m || !a) return;
    const s = L.s;
    const gate = m.gates[m.gateIndex];
    const rel = gate ? sub(gate.pos, a.pos) : null;
    const dist = rel ? length(rel) : 0;
    const bearing = rel ? Math.atan2(rel[0], -rel[2]) * R2D : 0;
    const diff = rel ? wrap180(bearing - a.heading) : 0;
    const elevation = rel ? Math.atan2(rel[1], Math.hypot(rel[0], rel[2])) * R2D : 0;

    // Mission data lives at the edges of the canopy, not in the HUD combiner.
    ctx.save();
    ctx.textBaseline = "middle";
    ctx.textAlign = "left";
    ctx.fillStyle = "rgba(169,255,178,0.72)";
    ctx.font = `${Math.max(7.5, 9 * s)}px Silkscreen, monospace`;
    ctx.fillText(`GATE ${pad(Math.min(m.gateIndex + 1, m.gates.length), 2)}/${pad(m.gates.length, 2)}`, W * 0.026, H * 0.058);
    ctx.font = `${Math.max(6.5, 7.6 * s)}px Silkscreen, monospace`;
    ctx.fillStyle = "rgba(169,255,178,0.54)";
    ctx.fillText(`${(dist / 1000).toFixed(1)} KM`, W * 0.026, H * 0.084);
    ctx.fillStyle = "rgba(169,255,178,0.40)";
    ctx.fillText(`${diff >= 0 ? "R" : "L"} ${pad(Math.abs(diff), 2)}°  ${elevation >= 0 ? "+" : "−"}${pad(Math.abs(elevation), 2)}°`, W * 0.026, H * 0.108);

    ctx.textAlign = "right";
    ctx.fillStyle = m.timeLeft < 20 ? RED : "rgba(169,255,178,0.78)";
    ctx.font = `${Math.max(8.5, 10.5 * s)}px Silkscreen, monospace`;
    ctx.fillText(`CLK ${formatTime(m.timeLeft)}`, W * 0.974, H * 0.058);
    ctx.fillStyle = "rgba(169,255,178,0.54)";
    ctx.font = `${Math.max(6.5, 7.6 * s)}px Silkscreen, monospace`;
    ctx.fillText(`SCR ${pad(m.score, 5)}`, W * 0.974, H * 0.084);
    ctx.fillStyle = "rgba(169,255,178,0.40)";
    ctx.fillText(`${pad(a.airspeedKt, 3)} KT`, W * 0.974, H * 0.108);
    ctx.restore();
  }

  // -------------------------------------------------------------- chase HUD
  function chaseWarnings() {
    const a = game.aircraft;
    const m = game.mission;
    const clearance = Math.min(a.agl, a.clearance > 0 ? a.clearance : a.agl);
    const sinkRate = -a.vs;
    const timeToImpact = sinkRate > 1 ? clearance / sinkRate : 99;
    const pullUp = clearance < 260 && sinkRate > 7 && (timeToImpact < 6.2 || clearance < 95);
    const warns = [];
    if (pullUp) warns.push({ text: "PULL UP", color: RED });
    if (a.stall > 0.06 || a.alphaDeg > 20.5 || a.speed < 92) warns.push({ text: "STALL", color: RED });
    if (a.gSmooth > 9.15) warns.push({ text: "G LIMIT", color: AMBER });
    if (a.fuel < 400) warns.push({ text: "BINGO", color: AMBER });
    if (m.timeLeft < 20) warns.push({ text: "TIME CRITICAL", color: AMBER });
    return warns;
  }

  function drawChaseGateCue(W, H, s) {
    const m = game.mission;
    const gate = m.gates[m.gateIndex];
    if (!gate) return;
    const rel = sub(gate.pos, game.camera.eye);
    const dist = length(rel);
    const p = projectPoint(gate.pos);
    const marginX = 44 * s;
    const marginTop = 96 * s;
    const marginBottom = 88 * s;
    if (p && p.x > marginX && p.x < W - marginX && p.y > marginTop && p.y < H - marginBottom) {
      const size = 8 * s + clamp(1 - dist / 5000, 0, 1) * 6 * s;
      ctx.save();
      ctx.strokeStyle = AMBER;
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(p.x, p.y - size);
      ctx.lineTo(p.x + size, p.y);
      ctx.lineTo(p.x, p.y + size);
      ctx.lineTo(p.x - size, p.y);
      ctx.closePath();
      ctx.stroke();
      ctx.strokeStyle = "rgba(255,192,91,0.30)";
      ctx.beginPath();
      ctx.arc(p.x, p.y, size * 1.85, 0, TAU);
      ctx.stroke();
      hText(`GATE ${pad(m.gateIndex + 1, 2)}  ${(dist / 1000).toFixed(1)} KM`, p.x, p.y + size * 2.6, 7.2 * s, AMBER, "center");
      ctx.restore();
      return;
    }

    // Off-screen locator. Use the camera basis directly so the arrow still
    // points at the gate when the target is behind the chase camera.
    const cam = game.camera;
    const tan = Math.tan(cam.fov / 2);
    const aspect = W / H;
    let nx = dot(rel, cam.right) / Math.max(Math.abs(dot(rel, cam.forward)) * tan * aspect, 0.05);
    let ny = dot(rel, cam.up) / Math.max(Math.abs(dot(rel, cam.forward)) * tan, 0.05);
    if (dot(rel, cam.forward) <= 0) {
      nx = -nx;
      ny = -ny;
    }
    const mag = Math.hypot(nx, ny) || 1;
    nx /= mag;
    ny /= mag;
    const cx = W * 0.5;
    const cy = H * 0.48;
    const rx = W * 0.39;
    const ry = H * 0.31;
    const x = cx + clamp(nx, -1, 1) * rx;
    const y = cy - clamp(ny, -1, 1) * ry;
    ctx.save();
    ctx.translate(x, y);
    ctx.rotate(Math.atan2(-ny, nx));
    ctx.fillStyle = AMBER;
    ctx.beginPath();
    ctx.moveTo(0, 0);
    ctx.lineTo(-11 * s, -6 * s);
    ctx.lineTo(-11 * s, 6 * s);
    ctx.closePath();
    ctx.fill();
    ctx.restore();
    hText(`${pad(m.gateIndex + 1, 2)}  ${(dist / 1000).toFixed(1)} KM`, x, y + 16 * s, 7.2 * s, AMBER, "center");
  }

  function drawChaseHUD(W, H) {
    const a = game.aircraft;
    const m = game.mission;
    if (!a || !m) return;
    const s = clamp(Math.min(W / 1280, H / 720), 1.0, 1.35);
    const edge = 20 * s;
    const topH = 76 * s;
    const bottomH = 70 * s;

    ctx.save();
    // Restrained canopy-frame HUD slabs.
    const topGrad = ctx.createLinearGradient(0, 0, 0, topH);
    topGrad.addColorStop(0, "rgba(4,10,6,0.82)");
    topGrad.addColorStop(0.72, "rgba(4,10,6,0.42)");
    topGrad.addColorStop(1, "rgba(4,10,6,0)");
    ctx.fillStyle = topGrad;
    ctx.fillRect(0, 0, W, topH);
    const botGrad = ctx.createLinearGradient(0, H - bottomH, 0, H);
    botGrad.addColorStop(0, "rgba(4,10,6,0)");
    botGrad.addColorStop(0.28, "rgba(4,10,6,0.44)");
    botGrad.addColorStop(1, "rgba(4,10,6,0.78)");
    ctx.fillStyle = botGrad;
    ctx.fillRect(0, H - bottomH, W, bottomH);
    hLine(edge, topH - 5 * s, W - edge, topH - 5 * s, "rgba(169,255,178,0.16)", 1);
    hLine(edge, H - bottomH + 5 * s, W - edge, H - bottomH + 5 * s, "rgba(169,255,178,0.12)", 1);

    // Top strip: airframe, route, time, energy.
    hText("F-16C  //  CHASE VIEW", edge, 17 * s, 8.5 * s, HUD, "left", "top", 700);
    hText("COLD RIVER  /  SECTOR 07", edge, 31 * s, 6.8 * s, HUD_DIM, "left", "top");
    hText(`GATE ${pad(m.gateIndex + 1, 2)} / ${pad(m.gates.length, 2)}`, W * 0.5, 14 * s, 8.8 * s, AMBER, "center", "top", 700);
    hText(formatTime(m.timeLeft), W * 0.5, 27 * s, 19 * s, m.timeLeft < 20 ? RED : HUD, "center", "top", 700, 5 * s);
    hText(`${pad(a.airspeedKt, 3)} KT`, W - edge, 14 * s, 12 * s, HUD, "right", "top", 700);
    hText(`ALT ${pad(ft(a.pos[1]), 5)} FT`, W - edge, 30 * s, 7.4 * s, HUD_SOFT, "right", "top");
    hText(`M ${a.mach.toFixed(2)}  G ${a.gSmooth.toFixed(1)}  AOA ${a.alphaDeg.toFixed(0)}  GAVL ${a.gAvail.toFixed(1)}`, W - edge, 43 * s, 6.8 * s, HUD_DIM, "right", "top");
    hText(`SCR ${pad(m.score, 5)}`, W - edge, 55 * s, 6.6 * s, HUD_DIM, "right", "top");

    // Gate locator and target data.
    drawChaseGateCue(W, H, s);
    const gate = m.gates[m.gateIndex];
    if (gate) {
      const rel = sub(gate.pos, a.pos);
      const bearing = Math.atan2(rel[0], -rel[2]) * R2D;
      const diff = wrap180(bearing - a.heading);
      const elevation = Math.atan2(rel[1], Math.hypot(rel[0], rel[2])) * R2D;
      hText(`TGT ${diff >= 0 ? "R" : "L"} ${pad(Math.abs(diff), 2)}°   EL ${elevation >= 0 ? "+" : "−"}${pad(Math.abs(elevation), 2)}°`, edge, H - 62 * s, 7.2 * s, AMBER, "left", "top");
    }

    // Warnings and transient messages.
    if (game.toastTimer > 0) {
      const alpha = clamp(game.toastTimer, 0, 1);
      hText(game.toast, W * 0.5, topH + 21 * s, 8.8 * s, `rgba(216,255,134,${0.62 + alpha * 0.36})`, "center", "middle", 700, 5 * s);
    }
    const warns = chaseWarnings();
    if (warns.length && game.state === "flying" && Math.floor(game.time * 4.5) % 2 === 0) {
      const y = H * 0.60;
      hText(warns[0].text, W * 0.5, y, 13 * s, warns[0].color, "center", "middle", 700, 7 * s);
      if (warns[1]) hText(warns[1].text, W * 0.5, y + 19 * s, 10 * s, warns[1].color, "center", "middle", 700, 5 * s);
    }

    // Bottom strip: mouse state, view hint, fuel.
    hText(game.mouse.locked ? "MOUSE CAPTURED  //  C CENTER" : "CLICK TO CAPTURE MOUSE  //  C CENTER", edge, H - 46 * s, 7.2 * s, game.mouse.locked ? HUD_SOFT : AMBER, "left", "top");
    hText(`FUEL ${pad(a.fuel * 2.20462, 4)} LB`, edge, H - 32 * s, 7 * s, a.fuel < 500 ? RED : HUD_DIM, "left", "top");
    hText("V  COCKPIT VIEW", W - edge, H - 46 * s, 7.6 * s, HUD_SOFT, "right", "top");
    hText("A / D  ROLL  ,   MOUSE PITCH / YAW", W - edge, H - 32 * s, 6.6 * s, HUD_DIM, "right", "top");
    hText("CHASE", W * 0.5, H - 18 * s, 6.2 * s, HUD_FAINT, "center", "top");
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
    ctx.clearRect(0, 0, W, H);
    if (game.state === "attract" || !game.aircraft || !game.mission) return;
    if (game.view === "chase") drawChaseHUD(W, H);
    else {
      window.__VIPER_COCKPIT.render(ctx, W, H, game);
      drawMissionOverlay(W, H, layoutFor(W, H));
    }
    drawEffects(W, H);
  }

  // ------------------------------------------------------------------ warnings
  function warningInfo() {
    const a = game.aircraft;
    if (!a) return { pullUp: false, stall: false };
    const clearance = Math.min(a.agl, a.clearance > 0 ? a.clearance : a.agl);
    const sink = -a.vs;
    const timeToImpact = sink > 1 ? clearance / sink : 99;
    const pullUp = clearance < 260 && sink > 7 && (timeToImpact < 6.2 || clearance < 95);
    const stall = a.stall > 0.06 || a.alphaDeg > 20.5 || a.speed < 92;
    return { pullUp, stall };
  }

  // --------------------------------------------------------------------- input
  const gameKeys = new Set([
    "KeyW", "KeyA", "KeyS", "KeyD", "KeyQ", "KeyE", "KeyH", "KeyK", "KeyV", "KeyC",
    "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Space",
    "ShiftLeft", "ShiftRight", "ControlLeft", "ControlRight",
    "AltLeft", "AltRight", "F1", "F2",
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
    if (event.code === "KeyK" && !event.repeat) {
      event.preventDefault(); toggleControls(); return;
    }
    if (game.controlsOpen) {
      if (event.code === "Escape") { event.preventDefault(); toggleControls(false); }
      return;
    }
    if (gameKeys.has(event.code)) event.preventDefault();
    if ((event.code === "AltLeft" || event.code === "AltRight") && game.state === "flying") {
      // Changing head direction must not add a new flight-control demand.
      game.mouse.x = 0; game.mouse.y = 0;
      keys.add(event.code);
      return;
    }
    if ((event.code === "F1" || event.code === "F2") && !event.repeat && game.state === "flying") {
      const i = event.code === "F1" ? 0 : 1;
      const pages = ["FCR", "HSD", "ENG"];
      game.mfdPages[i] = pages[(pages.indexOf(game.mfdPages[i]) + 1) % pages.length];
      return;
    }
    if (event.code === "KeyM" && !event.repeat) {
      toggleMute();
      return;
    }
    if (event.code === "KeyV" && !event.repeat) {
      toggleView();
      return;
    }
    if (event.code === "KeyC" && !event.repeat) {
      if (game.state === "flying") centerMouseStick();
      return;
    }
    if (event.code === "KeyP") {
      if (event.repeat) return;
      if (game.state === "flying") pauseMission();
      else if (game.state === "paused") { sound.init(); resumeMission(); }
      return;
    }
    if (event.code === "Escape") {
      if (event.repeat) return;
      if (game.state === "flying") pauseMission();
      else if (game.state === "paused") { sound.init(); resumeMission(); }
      else if (game.state === "won" || game.state === "crashed" || game.state === "timedout") goMenu();
      return;
    }
    if (event.code === "Enter") {
      if (event.repeat) return;
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
    if (gameKeys.has(event.code)) event.preventDefault();
    keys.delete(event.code);
    if ((event.code === "AltLeft" || event.code === "AltRight") &&
        !keys.has("AltLeft") && !keys.has("AltRight")) {
      centerMouseStick(false);
    }
  });

  canvas.addEventListener("mousedown", (event) => {
    if (game.state !== "flying") return;
    if (event.button === 2 || event.button === 1) {
      event.preventDefault();
      centerMouseStick();
      return;
    }
    if (event.button === 0) requestMouseControl();
  });

  canvas.addEventListener("contextmenu", (event) => {
    if (game.state === "flying") {
      event.preventDefault();
      centerMouseStick();
    }
  });

  window.addEventListener("mousemove", (event) => {
    if (game.state !== "flying") return;
    if (event.altKey || keys.has("AltLeft") || keys.has("AltRight")) {
      if (game.view === "cockpit") {
        game.look.yaw = clamp(game.look.yaw + event.movementX * 0.003, -155 * D2R, 155 * D2R);
        game.look.pitch = clamp(game.look.pitch - event.movementY * 0.003, -85 * D2R, 85 * D2R);
      } else {
        game.chaseLook.yaw += event.movementX * .003;
        game.chaseLook.pitch = clamp(game.chaseLook.pitch + event.movementY * .003, -65 * D2R, 65 * D2R);
      }
      return;
    }
    if (!game.mouse.locked) return;
    game.mouse.x = clamp(game.mouse.x + event.movementX * game.mouse.sensitivity, -1, 1);
    game.mouse.y = clamp(game.mouse.y + event.movementY * game.mouse.sensitivity, -1, 1);
  });

  document.addEventListener("pointerlockchange", () => {
    game.mouse.locked = document.pointerLockElement === canvas;
    if (!game.mouse.locked) {
      centerMouseStick(false);
      if (game.state === "flying") pauseMission();
    }
  });

  document.addEventListener("pointerlockerror", () => {
    game.mouse.locked = false;
    flashToast("MOUSE CAPTURE UNAVAILABLE // KEYBOARD", 2.2);
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

  function updatePilotVisuals(dt) {
    const a = game.aircraft;
    const active = game.state === "flying" && a && !a.crashed;
    let roll = 0;
    let pitch = 0;
    let rudder = 0;
    let brake = 0;
    let ab = 0;
    if (active) {
      const input = readInput();
      roll = clamp((input.rollAxis || 0) + (input.rollRight ? 1 : 0) - (input.rollLeft ? 1 : 0), -1, 1);
      pitch = clamp((input.pitchAxis || 0) + (input.pitchUp ? 1 : 0) - (input.pitchDown ? 1 : 0), -1, 1);
      rudder = clamp((input.yawRight ? 1 : 0) - (input.yawLeft ? 1 : 0), -1, 1);
      brake = input.airbrake ? 1 : 0;
      ab = a.ab ? 1 : 0;
    }
    const response = 1 - Math.exp(-Math.max(0, dt) * 11);
    const throttleResponse = 1 - Math.exp(-Math.max(0, dt) * 7);
    const rudderResponse = 1 - Math.exp(-Math.max(0, dt) * 8);
    game.hands.roll += (roll - game.hands.roll) * response;
    game.hands.pitch += (pitch - game.hands.pitch) * response;
    game.hands.rudder += (rudder - game.hands.rudder) * rudderResponse;
    game.hands.brake += (brake - game.hands.brake) * response;
    game.hands.ab += (ab - game.hands.ab) * response;
    game.hands.throttle += ((active ? a.throttle : 0.82) - game.hands.throttle) * throttleResponse;
    game.hands.grip += ((active ? clamp(Math.hypot(pitch, roll) * 0.35 + Math.abs(rudder) * 0.15, 0, 0.5) : 0) - game.hands.grip) * response;
  }

  function updateWorldAndCockpit(frameDt) {
    if (!renderer) return;
    updatePilotVisuals(frameDt);
    updateCamera();
    renderer.render(game.camera, { time: game.time, mission: game.mission, view: game.view, aircraft: game.aircraft });
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
      renderer.render(game.camera, { time: game.time, mission: previewMission, view: "attract", aircraft: null });
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
      if (params.get("view") === "chase" || params.has("chase")) game.view = "chase";
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
    setView: (view) => setView(view, false),
    toggleView,
    setMouse: (x, y) => {
      game.mouse.x = clamp(Number(x) || 0, -1, 1);
      game.mouse.y = clamp(Number(y) || 0, -1, 1);
    },
    setMouseLocked: (locked) => { game.mouse.locked = Boolean(locked); },
    centerMouseStick: () => centerMouseStick(false),
    setLook: (yaw, pitch) => {
      game.look.yaw = clamp(yaw * D2R, -155 * D2R, 155 * D2R);
      game.look.pitch = clamp(pitch * D2R, -85 * D2R, 85 * D2R);
    },
    setChaseLook: (yaw, pitch) => { game.chaseLook.yaw = yaw * D2R; game.chaseLook.pitch = pitch * D2R; },
    readInput,
    stepSimulation,
    settings,
    applySettings,
    toggleControls,
    snapshot: () => ({
      state: game.state,
      view: game.view,
      mission: game.mission,
      aircraft: game.aircraft,
      camera: game.camera,
      mouse: { ...game.mouse },
      time: game.time,
    }),
  };

  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", init);
  else init();
})();
