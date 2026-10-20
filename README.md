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
| `A` / `D` or `←` / `→` | Roll left / right |
| `S` / `↓` | Backup pull (pitch up) |
| `W` / `↑` | Backup push (pitch down) |
| `Q` / `E` | Rudder left / right |
| `C` or right-click | Center the mouse stick |
| `V` | Toggle cockpit / chase view |
| `Shift` / `Ctrl` | Throttle up / down |
| `Space` | Afterburner (at full throttle) |
| `B` | Airbrake |
| `P` / `Esc` | Pause / resume |
| `R` | Restart from pause or a result screen |
| `M` | Audio on / off |

Moving the mouse down pulls the nose up, matching a conventional flight-sim
stick. Mouse X commands yaw, while A/D rolls the jet. The pointer is captured by
clicking the canvas; release it with `Esc`. If pointer lock is unavailable, the
keyboard controls still work.

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
- Canvas 2D cockpit with HUD, pitch ladder, speed/altitude tapes, heading tape,
  tactical map MFD, engine MFD, data-entry display (DED), integrated control
  panel (ICP), standby ADI, standby gauges, caution/advisory panel, throttle,
  and stick. The chase view draws a procedural F-16 airframe and afterburner
  plume.
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

## Project layout

```
viper-run/
├── build.mjs               # zero-dependency single-file build
├── dist/
│   ├── viper-run.html      # playable build
│   └── index.html          # same file for hosting
└── src/
    ├── game.js             # physics, terrain, mission, WebGL renderer, audio
    ├── app.js              # game states, HUD/cockpit, UI screens, input
    ├── style.css           # menu and CRT presentation
    ├── index.html          # source shell
    └── fonts/              # embedded Silkscreen TTFs
```
