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
| `S` / `↓` | Pull (pitch up) |
| `W` / `↑` | Push (pitch down) |
| `A` / `D` or `←` / `→` | Roll left / right |
| `Q` / `E` | Rudder left / right |
| `Shift` / `Ctrl` | Throttle up / down |
| `Space` | Afterburner (at full throttle) |
| `B` | Airbrake |
| `P` / `Esc` | Pause / resume |
| `R` | Restart from pause or a result screen |
| `M` | Audio on / off |

Hold a pull rather than tapping it — the F-16 is fly-by-wire, but it still
carries energy and alpha. Roll into the turn first, then pull through the
valley. Watch the airspeed and the ridge lines.

## Objective

Fly through all **10 gates** in order before the clock reaches zero.

- Start clock: **02:30**
- Each gate: **+8 seconds**
- Gate score: **1,000 points each**
- Time bonus: **25 points per second left**
- Final rating: `ACE` under 1:35, `VIPER` under 2:05, `QUALIFIED` under 2:35,
  otherwise `CADET`

Terrain impact ends the run. So do stall departure, over-G, and the clock.
Best score and time are kept in `localStorage`.

## The jet

- Fixed-step 120 Hz simulation with sub-stepped rendering-independent input.
- Quaternion flight model with lift, drag, stall, alpha/G limiting, afterburner,
  fuel, airbrake, and terrain collision.
- Low-resolution WebGL 2 world (terrain, river, ocean, sky, volumetric-ish
  clouds, gates) composited through a CRT-style post pass.
- Canvas 2D cockpit with HUD, pitch ladder, speed/altitude tapes, heading tape,
  tactical map MFD, engine MFD, standby gauges, caution panel, throttle, and
  stick.
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

Requirements: a WebGL 2 browser. A keyboard is recommended. Chrome, Edge, and
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
