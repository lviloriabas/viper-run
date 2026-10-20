# VIPER RUN

A small, self-contained retro F-16 cockpit flight game. You start airborne in a
black river gorge with ten navigation gates ahead, two and a half minutes on the
clock, and a mountain range that does not move.

Everything is in one file: **`dist/viper-run.html`**. Open it in any modern
desktop browser with WebGL 2. No server, installs, network connection, or build
step is required to play.

## Fly

| Input | Action |
| --- | --- |
| Mouse | Pitch / yaw (click the canvas to capture the pointer) |
| Hold `Alt` and move mouse | Look around the cockpit without steering |
| `A` / `D` or left / right arrows | Roll left / right |
| `S` / down arrow | Backup pull (pitch up) |
| `W` / up arrow | Backup push (pitch down) |
| `Q` / `E` | Rudder left / right |
| `C` or right-click | Center the mouse stick and the head view |
| `V` | Toggle cockpit / chase view |
| `F1` / `F2` | Cycle left / right MFD through FCR, HSD and ENG |
| `K` | Open / close left controls and configuration drawer |
| `Shift` / `Ctrl` | Throttle up / down |
| `Space` | Afterburner (at full throttle) |
| `H` | Airbrake, hold |
| `P` / `Esc` | Pause / resume |
| `R` | Restart from pause or a result screen |
| `M` | Audio on / off |

Moving the mouse down pulls the nose up, matching a conventional flight-sim
stick. Mouse X commands yaw, while A/D rolls the jet. The pointer is captured by
clicking the canvas; release it with `Esc`. If pointer lock is unavailable, the
keyboard controls still work.

Hold Alt to inspect the pilot, consoles, wings and tail from the seat. Releasing
Alt restores the forward view and centers the mouse stick. Free look also
works without pointer lock while the mouse is inside the game window. The
physical cockpit and HUD combiner move in perspective with the head. The HUD
is visible only through its glass, rather than following the head around.
Hold Alt and move the mouse to orbit the external camera in chase view;
releasing Alt restores the normal chase camera. C also centers either view.

The default cockpit FOV is 58 degrees, adjustable from 45 to 72 in the left
drawer. Mouse sensitivity starts at 2.8 times the original response and can
be adjusted from 0.5x to 5x. The invert setting reverses mouse pitch; S always
pulls up and W always pushes down. Keyboard pitch overrides an opposing mouse
command while a key is held. Opening the drawer during flight pauses the jet
and mission clock; K resumes. Settings persist in localStorage when available.

Hold a pull rather than tapping it. The F-16 is fly-by-wire, but it still carries
energy and alpha. Roll into the turn first, then pull through the valley. Watch
the airspeed and the ridge lines. Press `V` when you want a chase-camera view of
the jet and the gorge.

## Objective

Fly through all **10 gates** in order before the clock reaches zero.

- Start clock: **02:30**
- Each gate: **+8 seconds**
- Gate score: **1,000 points each**
- Time bonus: **25 points per second left**
- Final rating: `ACE` under 1:35, `VIPER` under 2:05, `QUALIFIED` under 2:35,
  otherwise `CADET`

Hitting the ground, a canyon wall, a wingtip on either side, or the water ends
the run. The clock is a hard limit; stall and over-G warnings mean you are
running out of margin. Best score and time are kept in `localStorage`.

## The jet

- Fixed-step 120 Hz simulation with rendering-independent input.
- Quaternion flight model with ISA density and speed-of-sound variation,
  lift-dependent drag, Mach wave drag, ground effect, stall, alpha/G limiting,
  engine spool and afterburner lag, altitude thrust lapse, ram effect, fuel mass,
  airbrake, and multi-point airframe collision.
- F-16C Block 50 / F110-GE-129 scale and limits: approximately 9 g, 26° AoA,
  and a 6,000 m turn/roll test that produces less authority than at low level.
- Low-resolution WebGL 2 world (terrain, river, ocean, sky, volumetric-ish
  clouds, gates) composited through a CRT-style post pass.
- Depth-buffered WebGL cockpit with a reclined pilot, visible legs and boots,
  lap belts, kneeboard, ejection handle, articulated sleeves and gloved fingers
  around the left throttle and right pressure sidestick. Throttle, hands and
  rudder pedals follow the live controls. The sidestick has deliberately tiny
  travel, consistent with its pressure-sensing role.
- Modernized F-16V-inspired panel following the supplied reference: combined
  central DED and ICP below a smaller HUD, two portrait side displays and a
  large center pedestal HSD. Rounded housings, a sloped coaming and knee
  cutouts replace the old solid panel. RWR, RPM / FTIT / nozzle / oil gauges,
  standby ADI, altimeter, airspeed, HSD heading, vertical speed and caution indications.
  FCR and RWR show no contacts because the navigation mission has no enemies.
  HSD plots the actual route in heading-up orientation; ENG reads simulation
  telemetry. Decorative ICP keys and console switches are not clickable.
- Smooth exterior with measured 15.06 m overall length and 9.96 m span,
  continuous fuselage profiles, airfoil wings, blended leading-edge root
  extensions, rounded intake, separate transparent canopy and pilot, swept
  fin, stabilators, ventral fins, pylons, wingtip rails and petalled nozzle.
  Panel seams and restrained shading keep the retro finish without using
  box-shaped geometry for the main silhouette.
- Green NAV HUD with inertial pitch ladder, velocity-vector flight-path marker,
  KCAS, barometric MSL altitude, radar-style terrain AGL, heading, Mach, alpha,
  current / maximum G, steerpoint cue and warnings. AGL assumes flat terrain
  immediately below the jet; it is not a radar beam simulation. Mission time
  and score remain outside the combiner. Exterior wings and tail are rendered
  from the same aircraft position and attitude as the cockpit camera.
- Compressible calibrated airspeed, including the supersonic pitot relation;
  the HUD no longer confuses true and calibrated airspeed. Empty fuel cuts
  thrust and fuel flow. Engine indications follow spool and afterburner lag.
- The aerodynamic coefficients, engine lapse and instrument conversions are
  game approximations, not a full F-16 aerodynamic database or avionics suite.
  Sources, calibration and omissions are in [F16-REFERENCES.md](docs/F16-REFERENCES.md).
- Procedural audio only. No samples, images, or other external assets.
- Embedded Silkscreen font (OFL 1.1, by Jason Kottke).

## Develop

Source lives in `src/`. The build is a tiny zero-dependency Node script that
inlines the CSS, JavaScript, and fonts:

```bash
node build.mjs
```

Output:

- `dist/viper-run.html` — self-contained game, open directly in a browser
- `dist/index.html` — identical copy, convenient for static hosting

Test URLs:

- `dist/viper-run.html` — normal start menu
- `dist/viper-run.html?autostart=1` — skip the menu and fly immediately
- `dist/viper-run.html?autostart=1&view=chase` — start directly in chase view

Requirements: a WebGL 2 browser, a mouse, and a keyboard. Chrome, Edge, and
Firefox on desktop are the target; Safari may work but is not the primary target.

Checks, with no third-party test dependencies:

```bash
node tests/physics.mjs
node tests/performance.mjs
node tests/airframe.mjs
```

Serve the repository root with any local static HTTP server and open
`tests/browser.html`. It verifies boot, Alt/mouse isolation, MFD pages, pilot
animation, pause/resume, chase mode, gate crossings, win, timeout, crash and
restart, plus W/S override, H airbrake, K drawer, sensitivity, inversion, FOV
and settings persistence. The view buttons freeze the mission for visual inspection. Keep
the harness on HTTP; the shipped single-file game also works from a local file.

## Project layout

```
viper-run/
├── build.mjs               # zero-dependency single-file build
├── dist/
│   ├── viper-run.html      # playable build
│   └── index.html          # same file for hosting
└── src/
    ├── game.js             # physics, terrain, mission, WebGL renderer, audio
    ├── airframe.js         # smooth dimensioned exterior geometry
    ├── app.js              # game states, camera, UI screens, input
    ├── cockpit.js          # 3D cockpit, pilot, instruments and NAV HUD
    ├── style.css           # menu and CRT presentation
    ├── index.html          # source shell
    └── fonts/              # embedded Silkscreen TTFs
```
