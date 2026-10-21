# VIPER RUN

A small, self-contained retro F-16 cockpit flight game. You start airborne in a
black river gorge with ten navigation gates ahead, two and a half minutes on the
clock, and a mountain range that does not move.

The game is a single native executable of about 1 MB, written in Rust:

| Platform | File | Needs at run time |
| --- | --- | --- |
| Windows 10/11 x64 | `dist/viper-run-windows-x86_64.exe` | OpenGL 3.1 driver (any GPU from the last decade) |
| Linux x86_64 | `dist/viper-run-linux-x86_64` | X11 or XWayland, `libGL`, glibc 2.17+; PulseAudio, PipeWire or ALSA for sound |

No installer, runtime, game engine or asset files. Everything, including the
terrain, models, instrument pages, font and sounds, is generated procedurally
at start-up in about half a second.

## Fly

| Input | Action |
| --- | --- |
| Mouse | Pitch / yaw (captured automatically while flying) |
| Hold `Alt` and move mouse | Look around the cockpit without steering |
| `A` / `D` or left / right arrows | Roll left / right |
| `S` / down arrow | Backup pull (pitch up) |
| `W` / up arrow | Backup push (pitch down) |
| `Q` / `E` | Rudder left / right |
| `C` or right-click | Center the mouse stick and the head view |
| `V` | Toggle cockpit / chase view |
| `F1` / `F2` | Cycle left / right MFD through FCR, HSD and ENG |
| `K` | Open / close the controls and configuration drawer |
| `Shift` / `Ctrl` | Throttle up / down |
| `Space` | Afterburner (at full throttle) |
| `H` | Airbrake, hold |
| `P` / `Esc` | Pause / resume |
| `R` | Restart from pause or a result screen |
| `M` | Audio on / off |
| `F11` | Fullscreen |
| `Enter`, `Tab`, arrows, mouse | Menu navigation; `Ctrl+Q` quits from a menu |

Moving the mouse down pulls the nose up, matching a conventional flight-sim
stick. Mouse X commands yaw, while A/D rolls the jet. Keyboard pitch overrides
an opposing mouse command while a key is held.

Hold Alt to inspect the pilot, consoles, wings and tail from the seat; release
it to return to the forward view. In chase view Alt and the mouse orbit the
external camera. The default cockpit FOV is 58 degrees (45 to 72 in the
drawer), mouse sensitivity starts at 2.8x (0.5x to 5x) and the vertical axis
can be inverted. Opening the drawer pauses the flight. Settings and the best
run are saved to `%APPDATA%\viper-run\settings.txt` on Windows and
`~/.config/viper-run/settings.txt` on Linux.

Hold a pull rather than tapping it. The F-16 is fly-by-wire, but it still carries
energy and alpha. Roll into the turn first, then pull through the valley.

## Objective

Fly through all **10 gates** in order before the clock reaches zero.

- Start clock: **02:30**
- Each gate: **+8 seconds**
- Gate score: **1,000 points each**
- Time bonus: **25 points per second left**
- Final rating: `ACE` under 1:35, `VIPER` under 2:05, `QUALIFIED` under 2:35,
  otherwise `CADET`

Hitting the ground, a canyon wall, a wingtip, a stabilator or the water ends the
run. The clock is a hard limit; stall and over-G warnings mean you are running
out of margin.

## The jet

- Fixed-step 120 Hz simulation, independent of the frame rate, ported line by
  line from the original model: quaternion attitude, ISA density and speed of
  sound, compressible calibrated airspeed (including the supersonic pitot
  relation), lift-dependent drag, Mach wave drag, ground effect, stall, FBW
  alpha/G limiting, engine spool and afterburner lag, altitude thrust lapse, ram
  effect, fuel mass and airbrake. Full-afterburner level-flight equilibrium is
  Mach 1.653 / 1.740 / 1.869 / 2.019 at 10,000 / 20,000 / 30,000 / 40,000 ft,
  identical to the previous version.
- F-16C Block 50 / F110-GE-129 scale and limits: about 9 g, 26 degrees AoA.
- Exterior measured from the mesh by the test suite: 15.06 m overall length,
  9.96 m span over the wingtip AIM-9s and 5.09 m height. Lofted fuselage with
  an ogive radome, D-shaped chin intake with its diverter, blended leading-edge
  root extensions, 40-degree cropped-delta wing with a NACA 4 % section, swept
  fin with drag-chute housing, all-moving stabilators with 10 degrees of
  anhedral, canted ventral fins, petalled nozzle and gold-tinted bubble canopy.
  Stabilators, rudder and the clamshell speedbrakes move with the controls.
- Collision against ten airframe points: nose, canopy, tail, both wingtips,
  intake, belly, fin and both stabilator tips.
- Depth-buffered 3D cockpit at realistic distances from the design eye: the
  F-16V-style panel with two portrait MFDs and a large centre display, the UFC
  (DED over the ICP keypad) under a tall HUD combiner, RWR scope, standby ADI,
  airspeed and altimeter, RPM / FTIT / nozzle / oil gauges in round bezels,
  caution panel, AOA indexer, master caution, labelled side consoles, throttle
  and pressure sidestick, a reclined ACES II seat and the pilot's arms, hands
  and legs following the live controls.
- The canopy is a single convex glass layer with a faint tint, Fresnel sheen
  and sun glints, drawn once per pixel, so it never washes out the view.
- Green NAV HUD clipped to the physical combiner glass: inertial pitch ladder,
  flight-path marker, boxed KCAS and barometric altitude, radar altitude,
  heading tape, Mach, alpha, current and maximum G, steerpoint cue and warnings.
- Retro presentation: the world renders at a reduced resolution and goes
  through a CRT pass (barrel, scanlines, chroma, grain, vignette, G grey-out);
  the HUD and menus are drawn crisp at native resolution in a 5x7 pixel font.
- Procedural audio synthesised on its own thread: engine, wind, rumble and
  turbine whine follow throttle, speed and afterburner, plus warning tones.

The aerodynamic coefficients, engine lapse and instrument conversions are game
approximations, not a full F-16 aerodynamic database or avionics suite. Sources,
calibration and omissions are in [F16-REFERENCES.md](docs/F16-REFERENCES.md).

## Performance

The browser version recomputed the whole terrain noise function (45 noise
evaluations per vertex) every frame in the vertex shader and rebuilt the
cockpit as thousands of Canvas2D polygons. Now the terrain height field is
generated once at start-up on all CPU cores into a 32 m grid and uploaded as
static chunks with four levels of detail, frustum culling and skirts; the
cockpit is static geometry with a small dynamic mesh for the hands and controls,
and the instrument pages are redrawn into one texture at 30 Hz. On the test
machine the game holds the 165 Hz display refresh at 1280x720 and 1920x1080
(about 6 ms per frame, p99 7 ms).

## Build

The only dependency is [miniquad](https://crates.io/crates/miniquad), a tiny
windowing and OpenGL layer (no engine). On Linux it loads `libGL` and `libX11`
at run time, so building needs no system development packages.

```bash
rustup target add x86_64-pc-windows-gnu
./build.sh            # both binaries into dist/
cargo run --release   # run on the current machine
cargo test --release  # physics, terrain, mission and airframe checks
```

`build.sh` uses [Zig](https://ziglang.org) with
[cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) as the linker
when available, which cross-compiles both targets from Linux with one small
tool. Without it, it falls back to plain `cargo` with gcc and mingw-w64.

Test switches, useful for checking a change without flying:

```bash
viper-run --autostart                      # skip the menu
viper-run --shot out.png --sim 3 --chase   # simulate 3 s, save a screenshot, exit
viper-run --shot out.png --look 120,-20    # cockpit free look (yaw, pitch degrees)
viper-run --shot out.png --screen pause    # pause | win | crash | timeout
viper-run --bench 15 --size 1920x1080      # real-time frame timing, vsync off
```

## Project layout

```
viper-run/
├── Cargo.toml          # single dependency: miniquad
├── build.sh            # release binaries for Linux and Windows
├── dist/               # prebuilt executables
├── docs/F16-REFERENCES.md
└── src/
    ├── main.rs         # game states, input, cameras, frame composition
    ├── sim.rs          # flight model, atmosphere, collision, mission
    ├── terrain.rs      # analytic terrain, height grid, LOD chunk meshes
    ├── world.rs        # sky, terrain, ocean, clouds, gates, aircraft, CRT pass
    ├── airframe.rs     # dimensioned F-16 exterior and moving surfaces
    ├── cockpit.rs      # 3D cockpit, instrument pages and NAV HUD
    ├── ui.rs           # menus, controls drawer, flight overlays
    ├── canvas.rs       # GPU 2D batcher with convex clipping
    ├── mesh.rs         # mesh builder (lofts, lathes, capsules, bezels)
    ├── audio.rs        # synth and waveOut / PulseAudio / ALSA output
    ├── font.rs         # 5x7 pixel font
    ├── gfx.rs          # shader and render-target helpers
    ├── settings.rs     # settings and best-run persistence
    ├── math.rs         # vectors, quaternions, matrices
    └── png.rs          # screenshot writer
```
