# VIPER RUN

A small, self-contained retro F-16 cockpit flight game. You start airborne in a
black river gorge with ten navigation gates ahead, two and a half minutes on the
clock, and a mountain range that does not move. Or you fly the same gorge armed:
a dogfight against three AI-flown aggressors, or a strike on a bridge, an SA-6
battery and a convoy.

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
| Mouse | Pitch (when selected) and yaw; captured automatically while flying |
| Hold `Alt` and move mouse | Look around the cockpit without steering |
| `A` / `D` or left / right arrows | Roll left / right |
| `S` / down arrow | Pull, pitch up (when W/S pitch is selected) |
| `W` / up arrow | Push, pitch down (when W/S pitch is selected) |
| `Q` / `E` | Rudder left / right |
| `C` or right-click | Center the mouse stick and the head view (a left click only re-captures the mouse when it is free) |
| `V` | Toggle cockpit / chase view |
| `F` or left-click (mouse captured) | Fire the gun, launch a missile, release a bomb (pickle) |
| `X` | Cycle the weapon and master mode: NAV, GUN, SRM (AIM-9), MRM (AIM-120), CCIP (Mk-82) |
| `T` | Lock the target nearest the nose (or the helmet line of sight); again for the next |
| `Z` | Countermeasures: one chaff bundle and one flare |
| `F1` / `F2` | Cycle left / right MFD through FCR, HSD and ENG |
| `K` | Open / close the controls and configuration drawer |
| `Shift` / `Ctrl` | Throttle up / down |
| `Space` | Afterburner (at full throttle) |
| `H` | Speedbrake out / in (toggle) |
| `P` | Freeze the flight; the mouse still looks around and `V`, `C`, `F1` / `F2` still work, for screenshots |
| `Esc` | Open / close the pause menu |
| `R` | Restart from the pause menu or a result screen |
| `M` | Audio on / off |
| `F11` | Fullscreen |
| `Enter`, `Tab`, arrows, mouse | Menu navigation; `Ctrl+Q` quits from a menu |

An Xbox controller (or any XInput pad) works alongside the keyboard and
mouse. It is detected when it is plugged in, and the menus and the K drawer
then list its bindings:

| Controller | Action |
| --- | --- |
| Left stick | Pitch and roll; pull back to raise the nose (push, with INVERT PITCH on) |
| Right stick | Look around the cockpit, or orbit the chase camera; springs back |
| `RT` / `LT` | Throttle up / down; `RT` pulled to the stop lights the afterburner at full throttle |
| `LB` / `RB` | Rudder left / right |
| `B` | Speedbrake out / in |
| `A` | Fire / pickle (held for the gun) |
| D-pad up / down | Cycle weapon / lock or step target |
| Left-stick click | Countermeasures: chaff and flare |
| `X` or right-stick click | Center the view |
| `Y` | Toggle cockpit / chase view |
| D-pad left / right | Cycle left / right MFD page |
| `Start` | Start, freeze / unfreeze (right stick pans and holds the view), restart from a result screen |
| `B` while frozen | Open the pause menu |
| `Back` / `View` | Open / close the controls drawer |
| D-pad or left stick, `A`, `B` | Menu navigation, select, back |

On Windows the pad is read through XInput; on Linux through the kernel
joystick device (`/dev/input/js*`, xpad driver layout). Unplugging it in
flight opens the pause menu.

Pitch comes from one input at a time, chosen in the K drawer: the mouse
(default) or W/S. The other one does nothing, so a resting mouse offset can
never fight a held key. Moving the mouse down pulls the nose up, like a
flight-sim stick; a small dead band around centre makes neutral easy to find,
and C or right-click recentres it. W/S reach full deflection after 0.15 s held
and return in under 0.1 s, so a tap is a small correction. Mouse X works the
rudder and springs back to centre when the mouse stops; A/D roll the jet. A
small box at the bottom right shows the stick position, the pitch input in
use and SPD BRK while the speedbrake is out.

The flight control system works like the real F-16 FLCS: the stick commands
load factor (+9 / -3 g) and roll rate (up to 308 degrees per second), and with
the stick centred the jet holds 1 g. Wings level that flies a straight line;
in a bank it is still 1 g, so the nose drops unless you pull. A full pull
reaches 8.5 g in about a second at the start speed; when the speed is not
there, the AoA limiter trades g for AoA and holds the jet near 25 degrees.

Hold Alt to inspect the pilot, consoles, wings and tail from the seat; release
it to return to the forward view. In chase view Alt and the mouse orbit the
external camera. The default cockpit FOV is 58 degrees (45 to 72 in the
drawer), mouse sensitivity starts at 2.8x (0.5x to 5x) and INVERT PITCH flips
every pitch input at once: mouse Y, W / S and the controller's left stick.
Opening the drawer pauses the flight. Settings and the best
run are saved to `%APPDATA%\viper-run\settings.txt` on Windows and
`~/.config/viper-run/settings.txt` on Linux.

Hold a pull rather than tapping it. The F-16 is fly-by-wire, but it still carries
energy and alpha. Roll into the turn first, then pull through the valley.

## Missions

Pick the mission on the start screen. DOGFIGHT and STRIKE then open a loadout
screen: the stores on the outboard (stations 3 and 7) and inboard (4 and 6)
pylon pairs, the internal fuel, and the gross weight, stores drag and
thrust-to-weight that result. The wingtip AIM-9s, the 511 rounds of the gun and
30 chaff and 30 flares always come along.

| Mission | Objective | Default stores | Clock |
| --- | --- | --- | --- |
| GATE RUN | The 10-gate time trial below, unchanged | 2 AIM-9M (unused) | 02:30 |
| DOGFIGHT | Shoot down three aggressor F-16s; two come head-on, a third joins after the first kill | 2 AIM-9M, 2 AIM-120C | 05:00 |
| STRIKE | Destroy the Cold River bridge and the SA-6 launcher and radar | 2 AIM-9M, 4 Mk-82 | 07:00 |

Kills score 1,500 points; ground targets 200 (truck) to 1,500 (bridge); a
completed sortie adds 10 points per second left and 5 per point of airframe
health. Being shot down ends the run (the pilot ejects), as does hitting the
ground. The result screen shows the kills or targets, the time and a rating.

### Weapons and sensors

- Master modes follow the weapon (`X`): NAV with the master arm safe, GUN,
  SRM, MRM and CCIP. The HUD shows the selected weapon and count, the lock's
  range and closure, a target designator box, or a locator line when the
  target is outside the HUD.
- GUN: an EEGS-style funnel drawn from virtual rounds flown with the real
  ballistics, and an LCOS pipper on that stream at the locked range (2,000 ft
  without a lock) with a range ring. One round in five is a tracer.
- SRM: the AIM-9 seeker circle searches its 2.5-degree field along the
  boresight, or where the helmet looks, and slaves to a radar lock within its
  40-degree gimbal; a high tone means it tracks. MRM: the AIM-120 steering
  circle and dot. Both show IN RNG and SHOOT from a launch-zone fly-out
  against the target's present track.
- CCIP: the bomb's continuously computed impact point and the bomb fall line
  from the flight-path marker. The pipper is a real bomb flown ahead of time,
  so a bomb released on it lands on it. Fuzes arm 2 s after release; a bomb
  released too low is a dud, and its fragments reach a jet below 160 m.
- Turning the head (Alt and the mouse, or the right stick) brings up a
  helmet-mounted display: an aiming cross, air data, the target box and the
  seeker, which the helmet cues off the boresight. `T` locks the target
  nearest the helmet line of sight.
- The FCR page shows air contacts on a B-scope with aspect, or ground targets
  in CCIP; the RWR shows bandit radars (16), AAA (A) and the SA-6 radar (6),
  with a diamond on those locked on and a flashing M for missiles in flight.
  It pings for a new emitter, chirps on a lock and warbles while a missile
  guides; outside the cockpit a small scope repeats it.
- Flares fool only infrared seekers (the AIM-9), chaff only radar ones (the
  AIM-120 and the SA-6), best with the missile on the beam. The bandits use
  them too.

### Bandits and ground targets

Bandits fly the same six-degree-of-freedom F-16 and FLCS as the player, in
desert splinter paint. Their AI pilot only moves the stick and throttle: lead
pursuit to track with the gun, pure pursuit to close, lag pursuit to avoid an
overshoot, a max-g break when the player is at its six, beaming and then
breaking into missiles, unloading to regain speed, and an Auto-GCAS-style
recovery that predicts the pull-up and overrides everything near the terrain.
They shoot the gun inside 900 m on a lead solution and fire AIM-9s from
inside the launch zone. Hit points take 6 per 20 mm round; a warhead's damage
falls off with the miss distance. A damaged jet trails smoke, and a kill
leaves a burning wreck tumbling to the ground.

The STRIKE targets stand on the valley floor along the river: a convoy of
four trucks with a ZSU-23-4, the SA-6 launcher and radar with a resupply truck
and a second ZSU, the road bridge with a third, and a depot of warehouses and
fuel tanks. Armour decides what the gun can kill; the bridge only falls to a
bomb. The ZSU-23-4s fire tracer bursts at a jet within 2.5 km in their line of
sight; the SA-6 fires when its radar sees the jet more than 300 m above the
ground, so the gorge is the way in.

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

- Six-degree-of-freedom flight model after the JSBSim F-16 that FlightGear
  flies, integrated at 480 Hz inside a fixed 120 Hz step: forces and moments
  from the NASA TP-1538 wind-tunnel tables (AoA -10 to 45 degrees, sideslip,
  stabilator, flaperon and rudder, rate damping), FlightGear's Mach wave drag,
  transonic aerodynamic-centre shift and ground effect, body-axis moment
  equations with the inertia tensor, the roll-yaw product of inertia and the
  engine's gyroscopic moment. Mass and inertia follow the FlightGear Block 50
  model plus the stores on the stations and the fuel, which burns off as you
  fly; bombs and underwing missiles add mass and drag, and releasing them
  takes it off again.
- F110-GE-129 thrust from FlightGear's tables (17,000 / 29,000 lbf), lapse with
  altitude and Mach, normal-shock inlet recovery, slow core spool and quick
  afterburner light-off, fuel flow from TSFC. Full-afterburner level speed is
  Mach 1.24 at sea level and Mach 2.02 at 40,000 ft.
- FLCS with the F-16 command paths: stick-force g command, AoA limiter,
  roll-rate command reduced at high AoA, aileron-rudder interconnect, yaw
  damper and sideslip feedback, driving rate- and travel-limited stabilator,
  flaperon and rudder actuators. The start is trimmed for 1 g.
- F-16C Block 50 limits: +9 / -3 g, about 25 degrees AoA at full aft stick.
- Stall: each wing panel separates on its own local AoA against a critical
  angle of about 27 degrees in slow, smooth air. Mach (shock-induced
  separation), altitude, ground effect and sideslip lower it; a quick pull
  delays it for a moment. Rolling raises the AoA of the down-going wing,
  rudder that of the windward one, and low-level gusts either. Weight, load
  factor, bank and airspeed act through the AoA the jet needs. A separated
  panel loses lift and adds drag, and the flow only reattaches a few degrees
  below the critical angle. An uneven stall drops that wing and yaws the nose
  towards it (DEPARTURE). The limiter keeps a straight full pull just short
  of it in smooth air, but high and fast, slow in the gusts near the ground,
  or with rudder at full aft stick, the wing stalls.
- Pilot G tolerance from eye-level blood pressure: a hydrostatic column of
  about 22 mmHg per g, the anti-G suit, COMBAT EDGE pressure breathing, an
  AGSM that tires within a minute at 9 G, and the baroreflex, which after a
  push leaves the pilot less tolerant of the next pull. The retina and the
  brain hold a few seconds of oxygen. Relaxed and unprotected the view closes
  near 4.9 G, blacks out near 5.5 G and the pilot goes out near 6.9 G; a
  trained, protected pilot holds a 9 G break for 15 s with a narrowed view.
  The view greys, closes into a tunnel and blacks out over the HUD too, and
  negative g reddens it. Nothing happens if the g comes off before the
  brain's few seconds of oxygen run out; past that the pilot goes out, for
  about 3 s if the g comes off at once, and every second more under g adds
  more (2 to 38 s are measured in centrifuges), up to 20 s; the controls then
  come back over 2 to 12 s. Meanwhile Auto-GCAS rolls level and pulls 5 G if
  the terrain is close.
- Aeroelastic wing: the 40-degree swept wing washes out under load, losing
  lift outboard and flaperon power as the dynamic pressure climbs (the FLCS
  compensates). Its 8 Hz antisymmetric bending-torsion mode loses damping
  towards the flutter speed (920 KEAS, 15 % past the 800 KCAS placard, with a
  transonic dip and lower with heavy underwing stores) and past it grows into
  a limit cycle that breaks the wing; torsional divergence and 1.5 times the
  +9 / -3 g limits break it outright. The wings bend and flutter on the
  model, the cockpit shakes, the HUD warns FLUTTER.
- Condensation where the expanding flow cools the air past saturation (60 to
  70 % humidity under the cumulus, drier above): the LERX vortices stream
  over the wings at high AoA and burst past 25 degrees, the wingtip vortices
  leave trails in hard pulls, and the vapor cone fills the supersonic pocket
  from Mach 0.86 and ends at the terminating shock. Above about 7 km, below
  -40 C, the exhaust leaves a contrail.
- Mach effects: wave drag and the transonic aerodynamic-centre shift (the
  tables), transonic buffet from a lift-coefficient boundary that sags
  towards Mach 0.95, the altimeter's Mach jump as the bow shock passes the
  static ports, faint bow and tail shock cones past Mach 1, and the double
  crack of the sonic boom when the Mach cone sweeps over an outside camera.
- Exterior proportioned from a scale three-view and measured by the test
  suite: 15.06 m overall length, 9.96 m span over the wingtip AIM-9s, 4.88 m
  height, canopy, intake and wing
  stations along the body and the forebody depth. Long radome, D-shaped chin
  intake flush with the lower fuselage, blended leading-edge root extensions,
  40-degree cropped-delta wing with a NACA 4 % section, swept fin with
  drag-chute housing, all-moving stabilators with 10 degrees of anhedral,
  canted ventral fins, petalled nozzle and tinted bubble canopy. Stabilators,
  rudder and the clamshell speedbrakes show their true deflections.
- Weapons of the F-16C Block 50, simplified but with the published figures:
  the M61A1 gun (511 rounds, 6,000 rounds per minute, 1,036 m/s, rounds with
  gravity and velocity-squared drag), the AIM-9M Sidewinder (85.5 kg, about
  5 s of motor, 35 g, infrared seeker gimballed to 40 degrees), the AIM-120C
  AMRAAM (161.5 kg, boost-sustain motor to about Mach 4, 40 g, active radar)
  and the Mk-82 500 lb bomb (241 kg). Missiles fly proportional navigation
  with their thrust, wave and induced drag, angle-of-attack and g limits,
  arming delay and proximity fuze; see the reference document for the data.
- Collision against ten airframe points: nose, canopy, tail, both wingtips,
  intake, belly, fin and both stabilator tips.
- Depth-buffered 3D cockpit at realistic distances from the design eye: the
  F-16V-style panel with two portrait MFDs and a large centre display, the UFC
  (DED over the ICP keypad) under a tall HUD combiner, RWR scope, standby ADI,
  airspeed and altimeter, RPM / FTIT / nozzle / oil gauges in round bezels,
  caution panel, AOA indexer, master caution, side consoles after the F-16C
  Block 50 layout (every Dzus lightplate in its real place and size, with
  sharp legends and chunky toggles, guards, knobs and trim wheels, the tops
  tilted 10 degrees toward the pilot), the throttle in its quadrant and the
  pressure sidestick on its mount, both where they sit in the jet beside the
  pilot (the stick head shows at the bottom of the default view, which looks
  24 degrees down), a reclined ACES II seat and the pilot's arms, hands
  and legs following the live controls.
- Low-poly pixel art: the jet is a few hundred Gouraud-
  shaded polygons wearing a small hand-painted texture (FS 36270 / 36375
  greys, panel lines, low-vis star-and-bars, AV tail code, green fin band),
  sampled with nearest filtering; the cockpit is low-poly with chunky gauges.
- The canopy is a single convex glass layer with a faint tint, Fresnel sheen
  and sun glints, drawn once per pixel, so it never washes out the view.
- Green NAV HUD clipped to the physical combiner glass: inertial pitch ladder,
  flight-path marker, boxed KCAS and barometric altitude, radar altitude,
  heading tape, Mach, alpha, current and maximum G, steerpoint cue and warnings,
  with the weapon symbology of the selected master mode laid over it.
- Retro presentation: the world renders at a low internal resolution (320 to
  480 lines) shown as crisp blocks, quantised to 15-bit colour with a 4x4
  ordered dither; the terrain wears pixel tiles (grass, scrub, rock strata,
  snow, sand, patchwork farmland with hedgerows and the Cold River) with
  Gouraud lighting and dithered material borders, under a clear blue sky
  with screen-door clouds. The HUD and menus are drawn crisp at native
  resolution in a 5x7 pixel font.
- Procedural audio synthesised on its own thread: engine, wind, rumble and
  turbine whine follow throttle, speed and afterburner, plus warning tones,
  the gun's buzz, the Sidewinder's growl and lock tone, launches, blasts and
  the RWR.

The aerodynamic data is the reduced TP-1538 set (no leading-edge flap
schedule or post-stall tables beyond 45 degrees), and the avionics are a game
subset, not a full F-16 database or avionics suite. Sources,
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

## Recording

The Windows executable supports OBS Game Capture. Select the `VIPER RUN` window
in the source. The game includes a per-executable `RTSSHooksCompatibility` data
export: RivaTuner Statistics Server skips its overlay hooks for this game, so
they cannot collide with OBS while patching OpenGL's `SwapBuffers`. This avoids
the native access violation and black capture caused by that hook conflict.
It does not change OBS, RTSS profiles or other applications. The RTSS overlay
and its frame limiter are consequently unavailable inside this executable.

The export is retained in release builds with LTO and symbol stripping. OBS
capture was checked by reading its shared GPU texture, verifying visible,
changing game frames. Check the export in a rebuilt release with
`python3 tools/check_windows_capture.py dist/viper-run-windows-x86_64.exe`;
the check uses only Python's standard library.

## Build

The only dependency is [miniquad](https://crates.io/crates/miniquad), a tiny
windowing and OpenGL layer (no engine). On Linux it loads `libGL` and `libX11`
at run time, so building needs no system development packages.

```bash
rustup target add x86_64-pc-windows-gnu
./build.sh            # both binaries into dist/
cargo run --release   # run on the current machine
cargo test --release  # physics, weapons, AI, terrain, mission and airframe checks
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
viper-run --shot out.png --screen pause    # pause | freeze | win | crash | timeout
viper-run --bench 15 --size 1920x1080      # real-time frame timing, vsync off
viper-run --shot out.png --mach 0.98       # start at a Mach number (vapor cone, shocks)
viper-run --shot out.png --vision 0.5,0,0  # pilot state: retinal oxygen, redout, G-LOC s
viper-run --shot out.png --mission dogfight --sim 20 --input lock   # gates | dogfight | strike
viper-run --shot out.png --mission strike --input auto --sim 20     # AI flies to the steerpoint
viper-run --shot out.png --weapon gun --input fire --sim 1          # nav | gun | aim9 | aim120 | mk82
viper-run --shot out.png --mission strike --screen loadout          # also shotdown
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
    ├── sim.rs          # 6-DOF rigid body, FLCS, engine, collision, mission
    ├── physio.rs       # pilot G tolerance, G-LOC, redout, Auto-GCAS
    ├── structure.rs    # wing aeroelasticity, flutter, divergence, load limits
    ├── vapor.rs        # condensation, vapor cone, shock cones, sonic boom
    ├── weapons.rs      # gun, missiles, bombs: ballistics, guidance, sights, decoys
    ├── combat.rs       # loadouts, master modes, lock, bandits, targets, damage, missions
    ├── ai.rs           # AI pilot: BFM, weapons employment, terrain recovery
    ├── targets.rs      # STRIKE ground targets and their models
    ├── props.rs        # combat models, smoke trails, tracers, effects and sounds
    ├── symbology.rs    # HUD weapon symbols, helmet display, FCR, RWR, overlays
    ├── aero.rs         # F-16 aerodynamic tables and F110 thrust tables
    ├── terrain.rs      # analytic terrain, height grid, LOD chunk meshes
    ├── world.rs        # sky, terrain, ocean, clouds, gates, aircraft, pixel pass
    ├── texture.rs      # procedural pixel-art terrain and cloud textures
    ├── airframe.rs     # F-16 exterior, moving surfaces and painted skin
    ├── cockpit.rs      # 3D cockpit, instrument pages and NAV HUD
    ├── cockpit/consoles.rs  # side consoles: lightplates, legends, switches
    ├── cockpit/pilot.rs     # pilot, seat, stick and throttle
    ├── ui.rs           # menus, controls drawer, flight overlays
    ├── canvas.rs       # GPU 2D batcher with convex clipping
    ├── mesh.rs         # mesh builder (lofts, lathes, capsules, bezels)
    ├── audio.rs        # synth and waveOut / PulseAudio / ALSA output
    ├── gamepad.rs      # Xbox controller via XInput / Linux joystick device
    ├── font.rs         # 5x7 pixel font
    ├── gfx.rs          # shader and render-target helpers
    ├── settings.rs     # settings and best-run persistence
    ├── math.rs         # vectors, quaternions, matrices
    └── png.rs          # screenshot writer
```
