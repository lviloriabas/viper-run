# VIPER RUN

A self-contained retro cockpit flight game with four selectable aircraft:
F-16C Block 50, MiG-29S, F/A-18C Hornet and Su-27SK. Fly the Cold River gorge,
train in flight school, fight AI, strike ground targets or fly a runway circuit.
Online rooms add PVP dogfights and team combat, plus cooperative combat,
strike, escort and air defense for up to eight pilots on LAN or the Internet.

The game is a single native executable, written in Rust:

| Platform | File | Needs at run time |
| --- | --- | --- |
| Windows 10/11 x64 | `dist/viper-run-windows-x86_64.exe` | OpenGL 3.1 driver (any GPU from the last decade) |
| Linux x86_64 | `dist/viper-run-linux-x86_64` | X11 or XWayland, `libGL`, glibc 2.17+; PulseAudio, PipeWire or ALSA for sound |

No installer, separate runtime, game engine or external asset files are needed.
Terrain, F-16/MiG-29 models, instrument pages, font and engine sounds are generated
procedurally. Adapted FlightGear Hornet/Su-27 meshes, native material images and
original synthesised warning clips are embedded in the executable. Model sources,
credits and licences are included in [assets/models](assets/models/README.md).

## Flight school, aircraft and online rooms

FLIGHT SCHOOL opens five objective-driven lessons: controls, basic flight,
weapons, takeoff and landing. The instructor checks actual inputs and flight
conditions, follows remapped keys and controller layouts, and saves completion.
Training has no mission clock or return fire. Configuration includes restart
when flying a local lesson.

The F/A-18C and Su-27SK have distinct exterior and cockpit models, flight
polars, engine ratings, gear, flap schedules and weapon stations.
[New aircraft references and limitations](docs/NEW-AIRFRAMES.md) describe the
variant data, calibration and shared game approximations.

ONLINE provides private invitations, public directory browsing and quick
matching by mode. Choose a callsign, airframe, mode and capacity; join or host,
mark ready, then the host launches. The microphone starts off. Push to talk,
voice detection, mute, deafen and participant volume are in the room panel.
The default PTT key is U, remappable in configuration; controller View/Back is
PTT online and Start opens configuration.

The optional viper-server executable supplies directory and relay service on
UDP 42861. Internet relay requires a reachable server running that binary;
direct LAN rooms need no directory. See [multiplayer setup and verification](docs/MULTIPLAYER.md),
[mission rules](docs/MULTIPLAYER-MISSIONS.md) and [voice chat](docs/VOICE-CHAT.md).

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
| `X` | Cycle the weapon and master mode: GUN, SRM (AIM-9 or R-73), MRM (AIM-120 or R-27R), CCIP (bombs); weapons only, never NAV or the chaff and flares |
| `T` | Lock the target nearest the nose (or the helmet line of sight); again for the next |
| `I` | MiG-29: switch the radar and the IRST (OEPS-29) for locking |
| Hold `Backspace` for 0.5 s | Eject; both hands pull the seat handle |
| `Z` | Countermeasures: one chaff bundle and one flare |
| `F1` / `F2` | Cycle left / right MFD through FCR, HSD and ENG |
| `K` | Open / close the controls and configuration drawer |
| `Shift` / `Ctrl` | Throttle up / down |
| `Space` | Afterburner (at full throttle) |
| `H` | Airbrake deployed / retracted (toggle) |
| `G` | Landing gear handle up / down (it will not come up with weight on the wheels) |
| `L` | ALT FLAPS: trailing-edge flaps down with the gear up (they droop on their own with the gear down) |
| `B` (held) | Wheel brakes |
| `P` | Freeze the flight; the mouse still looks around and `V`, `C`, `F1` / `F2` still work, for screenshots |
| `Esc` | Open configuration and pause the flight; again closes it and resumes |
| `R` | Restart while paused or from a result screen |
| `M` | Audio on / off |
| `F11` / `Alt+Enter` | Borderless fullscreen |
| `F12` | Save a PNG screenshot in Pictures/viper-run, including in fullscreen |
| Right-click (held) | Zoom the view in to 40 % of its field of view, for as long as it is held |
| `Enter`, `Tab`, arrows, mouse | Menu navigation: arrows move the highlight, `Enter` acts on the highlighted button; choosing a game mode opens its setup immediately; `Space` engages from the Main Menu and launches from the setup screen; `Ctrl+Q` quits from a menu |

Every flight key above, from the stick to the MFD pages, can be rebound: the
K drawer opens a KEY BINDINGS panel with a primary and an alternate key per
action. Click a key and press the new one (Backspace clears it, Esc keeps the
old one); a key taken from another action leaves that action. `Esc`, `Enter`,
`P`, `K`, `M`, `F11`, `F12` and `Alt` stay fixed. The bindings are saved with the
other settings.

An Xbox controller (or any XInput pad) works alongside the keyboard and
mouse. It is detected when it is plugged in, and the menus and the K drawer
then list its bindings. The K drawer's PAD LAYOUT button switches between two
layouts. COMBAT, the default, keeps everything the fight needs on the
triggers and bumpers, so the left thumb never leaves the flight stick and
the right thumb never leaves the helmet: look at a bandit, lock it with `RB`,
fire with `RT`.

| Controller (COMBAT) | Action |
| --- | --- |
| Left stick | Pitch and roll; pull back to raise the nose (push, with INVERT PITCH on). On the ground its X axis also steers the nose wheel |
| Right stick | Look around the cockpit (the helmet line of sight), or orbit the chase camera; springs back |
| `RT` | Fire / pickle (held for the gun) |
| `LB` + `RB` held for 1.5 s | Eject; flight and weapon inputs are suspended during the hold |
| `RB` | Lock the target nearest the helmet line of sight or the nose; again for the next; held half a second drops the lock |
| `LB` / `LT` | Throttle up / down; `LB` held to the stop lights the afterburner |
| `LT` on the ground | Wheel brakes |
| `X` (held) | Rudder: the left stick's X axis works the pedals instead of the roll |
| Left-stick click | Countermeasures: chaff and flare |
| `A` | Cycle the weapon and master mode |
| `B` | Airbrake deployed / retracted |
| `Y` | Toggle cockpit / chase view |
| D-pad left / right | Cycle left / right MFD page |
| D-pad down / up | Landing gear / ALT FLAPS |
| D-pad left in the MiG-29 | Radar / IRST switch (the MiG has no MFDs) |
| `Start` | Start, freeze / unfreeze (right stick pans and holds the view), restart from a result screen |
| `B` while frozen | Pause and release the mouse |
| `Back` / `View` | Open / close the controls drawer |
| Right-stick click (held) | Zoom, for as long as it is held |
| D-pad or left stick, `A`, `B` | Menu navigation: `A` acts on the highlighted button, `B` goes back; `Start` engages and launches |

Menus show their prompts the same way everywhere: a button carries a key or
pad button only when that one works from anywhere on the screen (`Start` or
`Space` to launch, `B` or `Esc` back, `Back` or `K` for the controls), and the
legend along the foot of the screen names the rest: move, select (`A` or
`Enter`, always on the highlighted button) and the screen's shortcuts.

Menu movement follows the visible rows and columns. Holding the stick or
d-pad repeats after a short delay, with hysteresis around its activation point.
Button widths and settings positions stay the same when the active input changes;
a resting controller does not keep overriding keyboard prompts.

CLASSIC is the earlier layout: `RT` / `LT` throttle (`RT` to the stop for
afterburner), `LB` / `RB` rudder, `A` fire, d-pad up / down cycle weapon and
lock, `X` landing gear; the rest is as above.

On Windows the pad is read through XInput; on Linux through the kernel
joystick device (`/dev/input/js*`, xpad driver layout). Unplugging it in
flight pauses the flight and releases the mouse.

Pitch comes from one input at a time, chosen in the K drawer: the mouse
(default) or W/S. The other one does nothing, so a resting mouse offset can
never fight a held key. Moving the mouse down pulls the nose up, like a
flight-sim stick; a small dead band around centre makes neutral easy to find,
and C or right-click recentres it. W/S reach full deflection after 0.11 s held
and return in under 0.08 s, so a tap is a small correction. Mouse X works the
rudder and springs back to centre when the mouse stops; A/D roll the jet. A
small box at the bottom right shows the stick position, the pitch input in
use and SPD BRK while the speedbrake is out.

The flight control system follows the F-16 command paths: the stick commands
load factor (+9 / -3 g) and roll rate (up to 308 degrees per second), and with
the stick centred the jet holds 1 g. Wings level that flies a straight line;
in a bank it is still 1 g, so the nose drops unless you pull. A full pull
reaches 8.5 g in about a second at the start speed; when the speed is not
there, the AoA limiter trades g for AoA, up to a nominal 25.5 degrees.
The limit adapts to each wing's margin from separation and reduces roll and
pilot rudder authority near it; unloading restores those controls.

Hold Alt to inspect the pilot, consoles, wings and tail from the seat; release
it to return to the forward view. In chase view Alt and the mouse orbit the
external camera. The default cockpit FOV is 58 degrees (45 to 72 in the
drawer), mouse sensitivity starts at 2.8x (0.5x to 5x) and INVERT PITCH flips
every pitch input at once: mouse Y, W / S and the controller's left stick.
Opening the drawer pauses the flight; closing it restores the previous flying or frozen state. RESTART and MAIN MENU are available in the drawer during a sortie. Settings and the best
run are saved to `%APPDATA%\viper-run\settings.txt` on Windows and
`~/.config/viper-run/settings.txt` on Linux.

Hold a pull rather than tapping it. The F-16 is fly-by-wire, but it still carries
energy and alpha. Roll into the turn first, then pull through the valley.

## Missions

Pick the mission on the start screen; ENGAGE then opens its setup screen.
Every mission sets the airframe (F-16C, MiG-29S, F/A-18C or Su-27SK), the internal
fuel and, except free flight, the mission clock there. DOGFIGHT, STRIKE and
FREE FLIGHT add the loadout and the opposition: the stores on the outboard (stations 3 and 7) and inboard (4 and 6)
pylon pairs, the internal fuel, and the gross weight, stores drag and
thrust-to-weight that result. The wingtip AIM-9s, the 511 rounds of the gun and
30 chaff and 30 flares always come along. The fuel steps down from full
through 75, 50 and 25 % to the minimum a jet may start with: 1,000 lb in
the F-16C, the Block 40+ minimum fuel, and an estimated 550 kg in the MiG,
whose setup reads in kilograms like its gauges.

Every station pair can be changed, the wingtips included. The F-16C Block
50 has nine stations: air-to-air missiles go on 1 to 3 and 7 to 9 (AIM-9Ms,
AIM-9Xs, AIM-120Cs or AIM-120Ds on the wingtip rails 1 and 9 and on 2 and
8, those or AIM-7M Sparrows on 3 and 7), and bombs on 3, 4, 6 and 7, one
Mk-82 each or three on a TER-9/A triple ejector rack, for up to six
missiles and twelve Mk-82s; 4 and 6 are not air-to-air stations, and the
centreline (5) takes a tank, which the game leaves off. The MiG-29S 9.13S
has three pylons under each wing and one under the belly for a tank:
R-73s (AA-11 Archer) or R-60Ms (AA-8 Aphid) on the outer pair; those,
R-77s (AA-12 Adder, active radar), FAB-250M-62s or FAB-500M-62s on the
middle pair; R-27Rs (AA-10 Alamo-A, semi-active), R-27Ts (Alamo-B,
infrared), R-77s, FAB-500s, FAB-250s or two FAB-250s on an MBD3-U2T tandem
rack on the inner pair, for up to six missiles or 2,000 kg of bombs. A
rack's own weight and drag stay on the pylon after its bombs have gone.
Its GSh-30-1 fires 150 rounds of 30 mm, and two BVP-30-26M dispensers hold
30 chaff and 30 flares. The DOGFIGHT defaults match each other: four heat
seekers and two active radar missiles each (AIM-9Ms on 1, 2, 8 and 9 and
AIM-120Cs on 3 and 7, or four R-73s and two R-77s); STRIKE is four Mk-82s
or two FAB-250s and two FAB-500s, FREE FLIGHT two AIM-120Cs and two Mk-82s
or four R-73s and two FAB-500s. Changing jets swaps each pair's stores for
the other airframe's equivalent in the same family.

The missiles come in families by what guides them (`weapons.rs`,
`combat::arsenal`):

| Family | F-16C Block 50 | MiG-29S 9.13S |
| --- | --- | --- |
| Infrared, short range (Fox 2) | AIM-9M, AIM-9X | R-60M, R-73 |
| Infrared, medium range (Fox 2) | none | R-27T |
| Semi-active radar (Fox 1) | AIM-7M | R-27R |
| Active radar (Fox 3) | AIM-120C, AIM-120D | R-77 |
| Bombs | Mk-82 | FAB-250M-62, FAB-500M-62 |

| Mission | Objective | Default stores | Clock |
| --- | --- | --- | --- |
| GATE RUN | The 10-gate time trial below, unchanged | 2 AIM-9M (unused) | 02:30 |
| DOGFIGHT | Shoot down three aggressors (MiG-29s, or F-16s); two come head-on from 20 km, spread 3 km apart, a third joins after the first kill | 4 AIM-9M, 2 AIM-120C | 06:00 |
| STRIKE | Destroy the Cold River bridge and the SA-6 launcher and radar | 2 AIM-9M, 4 Mk-82 | 07:00 |
| AIRFIELD | Take off from runway A of the valley airbase, fly out to a steerpoint 9 km past the far end, come back and land to a full stop on the runway | 2 AIM-9M, 50 % fuel | 10:00 |
| FREE FLIGHT | Practice with no end: the gate rings on a loop, the strike targets and a pair of bandits flying circuits round the gorge (the lead clockwise 14 km out, the wing the other way 9 km out), none of which chase or shoot at you; a new pair comes 30 s after the last one falls, the targets are rebuilt 30 s after the last one burns, and stopping on the ground rearms, refuels and repairs the jet; with AUTO REFILL on in its setup, each kind of weapon reloads as soon as the last one is fired, the gun when its drum is empty and the chaff and flares when both run out. Only a crash ends it | 2 AIM-9M, 2 AIM-120C, 2 Mk-82 | none |

MISSION CLOCK on the setup screen (saved with the settings) switches the
clock off for every mission: nothing times out, the HUD counts the time flown
instead, no seconds-left bonus is scored, and a gate run without a clock does
not set a best score.

Kills score 1,500 points; ground targets 200 (truck) to 1,500 (bridge); a
completed sortie adds 10 points per second left and 5 per point of airframe
health. Being shot down ends the run (the pilot ejects), as does hitting the
ground. The result screen shows the kills or targets, the time and a rating.

### Weapons and sensors

- Master modes follow the weapon (`X`): GUN, SRM, MRM and CCIP, as the
  stores on board allow. The jet starts in NAV with the master arm safe, but
  `X` cycles only the weapons and never stops on NAV or the countermeasures,
  which have their own button (`Z`). The HUD shows the master mode and arm state (for example
  SRM ARM), the selected weapon with what is left of it (2 AIM-9M, 511 20MM,
  or EMPTY), the lock's
  range and closure, a target designator box, or a locator line when the
  target is outside the HUD.
- GUN: an EEGS-style funnel drawn from virtual rounds flown with the real
  ballistics, and an LCOS pipper on that stream at the locked range (2,000 ft
  without a lock) with a range ring. One round in five is a tracer.
- SRM: the AIM-9 seeker circle searches its 2.5-degree field along the
  boresight, or where the helmet looks, and slaves to a radar lock within its
  gimbal (40 degrees on the AIM-9M, 90 on the AIM-9X, 20 on the R-60M, 45 on the R-73); a high tone means it tracks. MRM: the AIM-120 steering
  circle and dot. Both show IN RNG and SHOOT from a launch-zone fly-out
  against the target's present track.
- CCIP: the bomb's continuously computed impact point and the bomb fall line
  from the flight-path marker. The pipper is a real bomb flown ahead of time,
  so a bomb released on it lands on it. Fuzes arm 2 s after release; a bomb
  released too low is a dud, and its fragments reach a jet below 160 m.
- Turning the head (Alt and the mouse, or the right stick) brings up a
  helmet-mounted display that works like JHMCS: it blanks while the line of
  sight passes through the HUD; fixed in the visor are the aiming cross,
  airspeed and altitude boxes, heading, Mach, G, radar altitude down low, the
  off-boresight angle and the master mode, arm state and weapon count;
  stabilised on the world are the aircraft boresight cross, the target box
  (or a locator line with the angle to go) and the seeker, which the helmet
  cues off the boresight. `T` locks the target nearest the helmet line of
  sight.
- The FCR page shows air contacts on a B-scope with aspect, or ground targets
  in CCIP; the RWR shows bandit radars (16), AAA (A) and the SA-6 radar (6),
  with a diamond on those locked on and a flashing M for missiles in flight.
  It pings for a new emitter, chirps on a lock and warbles while a missile
  guides; outside the cockpit a small scope repeats it.
- Launch rules follow the seeker (docs/WEAPONS.md). Heat seekers lock on
  before launch: with no tone the AIM-9M, R-60M, R-73 and R-27T leave
  unguided, while the AIM-9X Block II takes the radar lock as its cue and
  locks on after launch (or searches ahead with nothing to cue it). The
  semi-active AIM-7M and R-27R are not fired without a radar lock and go
  blind the moment it drops. The AIM-120C/D and R-77 ride the datalink
  while the radar holds the lock and turn their own radar on near the
  target; fired without a lock they go active off the rail, and if the lock
  drops with the target still beyond 15 km they fly on blind and search
  when they get there. The R-27T flies in MRM but shows the seeker circle.
- The MiG-29S's OEPS-29 sight puts an IRST and a laser rangefinder in the
  ball ahead of the windscreen. `I` (d-pad left on a pad) switches the lock
  between the N019M radar and the IRST. The IRST searches 30 degrees either
  side, 15 down and 30 up, and sees a fighter some 15 km from behind but only
  8 km head on, half as far again if it is in afterburner; it is passive, so
  nothing warns the target. Its track cues the R-73, R-60M and R-27T seekers,
  but the R-27R and R-77 need the radar. The laser gives a range inside
  6.5 km; beyond that the HUD reads LASER --- and the IT-23 shows the IRST's
  azimuth and elevation picture (TP) with the range as D. Neither jet has a
  missile approach warner: bandits know a radar missile by its guidance, but
  see a heat seeker only by its smoke within 6 km or close in.
- Flares fool only infrared seekers (the AIM-9s, R-60M, R-73 and R-27T),
  chaff only radar ones (the AMRAAMs, Sparrow, R-27R, R-77 and SA-6), best with the missile on the beam. The bandits use
  them too.

### Bandits and ground targets

Bandits fly the same six-degree-of-freedom model and FLCS as the player:
MiG-29S Fulcrums by default, or F-16s in the 64th Aggressor Squadron's splinter schemes, taken in turn: Flanker blues, Desert, Ghost and Arctic, with Nellis's WA tail code (TYPE in the
threat setup); the RWR calls them 29 or 16. Their pilots coordinate an attacker
and supporting flankers, relieve slow or threatened wingmen, preserve energy,
avoid friendly firing lanes and stagger missile launches. Each jet carries six
missiles: short-range IR plus active radar on leads, with semi-active radar on
cover jets. They select a weapon by range, geometry, radar support and a real
fly-out, then remove its station's mass and drag on release. Guns fire bounded
bursts on a continuously checked ballistic and angular tracking solution.

Missile defence combines beam, drag and crossing manoeuvres, finite chaff/flare
programs, a cooler tail against IR and safe terrain masking. Radar illumination
and datalink require real tracks; losing the whole flight's view leaves only a
last-known contact. Seekers cannot see through ridges, radar seekers can lose a
sustained look-down notch, and missiles can run out of energy or miss a hard
crossing target. Close high-energy shots remain dangerous. An Auto-GCAS-style
recovery overrides tactics near terrain, and free-flight patrols hold fire and
countermeasures. See [Enemy AI and missile defence](docs/ENEMY-AI.md) for the
behaviour and simulation checks. A damaged jet trails smoke, and a kill leaves
a burning wreck tumbling to the ground.

Damage goes where the hits land (`damage.rs`), for the player and the
bandits alike, in a simplified version of module damage. Each jet is a set
of systems in its own geometry, each with its own hit points: the pilot,
the engine (two on the MiG-29), the fuselage and wing fuel tanks, the
wings, the stabilators, the fin (twin fins on the MiG), the hydraulics and
the airframe structure round them. A round flies its real path through
the jet: it holes the skin and spends itself in what it crosses, the first
system most, and its high explosive (8 points for the 20 mm PGU-28, 24 for
the 30 mm OFZ, 5 for the ZSU's 23 mm) reaches what lies within half a
metre or so; a round can pass clean between the wing and the tail. A
missile warhead throws a ring of fragments round its axis that hit
whatever they meet out to its outer radius, weaker the further they fly,
and its blast wrecks everything within its lethal radius but the pilot,
whom only fragments reach. Rounds and fragments can set a tank or an
engine on fire. A fire burns the system and the structure round it, faster
the longer it burns, until it goes out; the MiG-29's extinguisher puts out
its first engine fire, the F-16C has none, and a burning tank that burns
through explodes. A fire shows as flames licking back from the burning
system and a trail of black smoke; a holed tank streams a white mist of
fuel, and a damaged engine trails grey smoke. An engine below half its hit
points loses its oil and seizes in a minute or so; a seized engine burns
no fuel, spools down to windmilling and gives no thrust, so the F-16C
glides, while the MiG-29 flies on its other engine and yaws toward the
dead one.

There are no hit points for the whole jet: what ends a flight is losing
the pilot (a single 20 mm round through the cockpit can do it), a wing,
the stabilators or the structure, or a tank exploding. Short of that the
damage reaches the flight model: a damaged engine gives less thrust and a
wrecked or burning one none (the MiG keeps its other engine), holed tanks
leak up to 3 kg/s each, a damaged wing lifts less, drags more and rolls
the jet toward its side, damaged hydraulics, stabilators or fin cut the
surface travel, and a damaged structure takes less g. In the tests an
AIM-9M bursting 4 m from an F-16 brings it down nine times in ten and
almost never from 10 m, the 39 kg R-27R reaches further, and spread over
the whole jet it takes about 10 hits of 20 mm or 3 of 30 mm to bring a
fighter down outright, fewer on one spot, while an engine, a tank or the
hydraulics can be lost to the first few. The HUD and
the overlays name the damage (ENGINE FIRE, ENGINE DAMAGE, HYDRAULICS, WING
DAMAGE, FUEL LEAK, STRUCT DAMAGE) and every hit is announced with the
systems it struck; AIRFRAME % shows what is left of the systems overall;
the F-16's caution panel lights FLCS, FUEL LOW, ENGINE and ENG FIRE, and
the MiG's POZHAR lamp a fire. The result screen says what brought the jet
down; a pilot killed in the cockpit does not eject. Stopping on the
ground in free flight repairs it all.

The STRIKE targets stand on the valley floor along the river: a convoy of
four trucks with a ZSU-23-4, the SA-6 launcher and radar with a resupply truck
and a second ZSU, the road bridge with a third, and a depot of warehouses and
fuel tanks. Armour decides what the gun can kill; the bridge only falls to a
bomb. The ZSU-23-4s fire tracer bursts at a jet within 2.5 km in their line of
sight; the SA-6 fires when its radar sees the jet more than 300 m above the
ground, so the gorge is the way in.

### Scenery

The valley is lived in. The airbase has a control tower, three arched
maintenance hangars, four hardened aircraft shelters, a fuel farm, the
approach radar's dome and a windsock. Seven farming villages stand on the
valley floor by the river, each a lane of gabled houses with a church, red
barns and silos and trees round it; groves of conifers and broadleaf trees
dot the floor and the lower slopes about a kilometre apart, and a power line
runs down the west side of the valley. All of it is placed once from a fixed
seed on the terrain grid, clear of the runway and the strike targets, and
drawn within 14 km.

### MiG-29

AIRFRAME on the setup screen swaps the player's F-16C for a MiG-29S
(9.13S, Fulcrum-C), and TYPE in the threat setup picks the aggressors. The
9.13S is the 9.13 "fatback", whose humped spine holds a larger No. 1 fuel
tank and an L-203 jammer, with the N019M radar that adds the R-77. The
Fulcrum is a low-poly model after the published three-view (17.3 m long, 11.4 m span, 4.7 m tall): radome and
bubble canopy, leading-edge root extensions, the flat centre body between
two widely spaced nacelles with wedge intakes, 42 degree wings, twin fins
canted out 6 degrees, moving stabilators, two afterburner flames, in the
two-tone grey of the type with red stars. It flies its own lift, drag,
pitch and engine data (src/aero_mig29.rs). No wind-tunnel tables of the
Fulcrum are public, so the model is fitted to the published figures: Cx0 =
0.022, Cy = 1.43 at 24 degrees AoA and an Oswald factor of 0.87 from the
MiG-29A aerodynamics manual; 6.6 g sustained at Mach 0.9 and 5,000 m, and a
6 g turn at 1,000 m and 900 km/h in afterburner still gaining 20 km/h a
second, from its turn charts; and 1,500 km/h (Mach 1.26) at sea level and
2,450 km/h (Mach 2.3) high. The model gives 6.8 g, 20 km/h a second, Mach
1.28 and Mach 2.32. Lateral data and rate damping are still the F-16's, on
the MiG's 11.36 m span and 3.73 m chord. The rest comes from public
figures: 38.06 m² of wing, 11,200 kg
empty, 3,820 kg of internal fuel (the 9.12's 3,630 and the spine tank's
extra 240 litres), two RD-33s with 49.4 kN (11,111 lbf) dry
and 81.8 kN (18,387 lbf) in afterburner each (0.77 and 1.85 lb/lbf/h),
behind variable-ramp inlets, and about 35 % more inertia.
The limits follow what the 9.12 and 9.13 flight manuals are cited as giving: the AoA
held near 25 degrees (24 in the Russian manual, 26 in the German one, against
26 to 28 critical), 9 g only below 14,200 kg, 8 g above, and 7 g past
Mach 0.85 (German manual). The cockpit
is the Fulcrum's own (docs/MIG29-REFERENCES.md).

### Ejection views

While the ejection key or both controller bumpers are held, steering, throttle,
weapons and other flight bindings are suspended. Both hands reach forward clear
of the vest, close around the raised seat handle
and pull upward; the pilot briefly checks the grip, then looks forward again.
The F-16 uses one ring, the MiG-29 two linked grips. The hands hold until
seat separation; the K-36DM adds arm protectors, leg lift and delayed rocket
ignition after its catapult stroke. The cockpit remains visible
until the seat fires, then V switches between the pilot's eyes and the external
seat/parachute view. The close view omits the helmet and neck cap. On touchdown
the parachute falls to the side and settles on the ground, retaining its fabric.
Thin suspension cords meet wider fabric risers connected to the shoulder harness.
Research, timing approximations and visual checks are in
[COCKPIT-REFINEMENT.md](docs/COCKPIT-REFINEMENT.md).

The MiG-29 mirrors refresh on every rendered cockpit frame, independently of the
30 Hz instruments, including at a 165 Hz display refresh. Actual frame rate
still follows the renderer and display settings.

### Airbase, take-off and landing

An airbase sits on the west bank of the valley floor, north of the gate run's
start: a 9,200 ft runway 45 m wide (threshold A in the south, B in the north)
with edge and centre lines, piano keys, aim-point and touchdown-zone marks
and tyre scuffs, overruns with chevrons, a parallel taxiway with three links
and an apron. The ground under it is levelled into the terrain, so the wheels
roll on what you see.

- Landing gear: three oleo legs (nose 3.45 m ahead of the centre of gravity,
  mains 0.55 m behind it on a 2.36 m track) with springs, dampers and a 30 cm
  stroke. The jet stands on them with about 14 % of its weight on the nose
  wheel. Tyres roll with a little resistance, grip sideways up to their
  limit, the mains brake and the nose wheel steers with the rudder input (32
  degrees at a taxi, a few at speed). A sink rate that bottoms a strut breaks
  the leg; a touchdown with the gear up is a belly landing and ends the run.
- The gear takes 5 s to cycle, the nose leg leading, and adds drag; above 300
  KCAS the HUD shows GEAR SPEED. Continued air loads first jam the mechanism
  and then break individual legs; retracting the handle cannot repair them. GEAR UP flashes when the jet sinks toward the
  ground below 220 KCAS with the gear still up.
- Trailing-edge flaps droop 20 degrees with the gear handle down, or with ALT
  FLAPS, adding lift (lower stall and approach speeds), drag and a nose-down
  moment the FLCS trims out; they blow back up above 370 KCAS.
- On the wheels the FLCS rests its g integrator, so the stick commands pitch
  rate for the rotation. In full afterburner the jet rotates at about 150 KCAS
  and lifts off after roughly 12 s.

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
  fly; bombs and underwing missiles add mass, drag and roll inertia (they
  hang about 2.9 m out on the pylons), and releasing them takes it off again.
  The weight sets the AoA every manoeuvre needs and the drag eats the
  thrust, so a loaded jet accelerates, climbs, sustains g and rolls worse.
  The data box at the top left shows the gross weight, the drag the stores,
  gear and flaps add (in counts) and the thrust-to-weight ratio of the
  moment.
- F110-GE-129 thrust from FlightGear's tables (17,000 / 29,000 lbf), lapse with
  altitude and Mach, normal-shock inlet recovery, slow core spool and quick
  afterburner light-off, fuel flow from TSFC. Full-afterburner level speed is
  Mach 1.24 at sea level and Mach 2.02 at 40,000 ft.
- FLCS with the F-16 command paths: stick-force g command, AoA limiter,
  roll-rate command reduced at high AoA, aileron-rudder interconnect, yaw
  damper and sideslip feedback, driving rate- and travel-limited stabilator,
  flaperon and rudder actuators. The start is trimmed for 1 g.
- F-16C Block 50 limits: +9 / -3 g and a nominal 25.5 degree AoA cap.
  Adaptive protection lowers the cap for Mach, altitude, ground proximity,
  sideslip, roll and gusts. It also limits roll/yaw coupling to the moment
  the stabilator can counter. See [flight protection](docs/FLIGHT-PROTECTION.md).
- Stall: each wing panel separates on its own local AoA against a critical
  angle of about 27 degrees in slow, smooth air. Mach (shock-induced
  separation), altitude, ground effect and sideslip lower it; a quick pull
  delays it for a moment. Rolling raises the AoA of the down-going wing,
  rudder that of the windward one, and low-level gusts either. Weight, load
  factor, bank and airspeed act through the AoA the jet needs. A separated
  panel loses lift and adds drag, and the flow only reattaches a few degrees
  below the critical angle. An uneven stall drops that wing and yaws the nose
  towards it (DEPARTURE). With protection active, pitch, roll and pilot
  pedals share a reserve below the separation boundary. The override still
  allows stalls and departures; protection cannot create lift when the jet
  has run out of speed or restore lost control authority after damage.
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
  +9 / -3 g limits break it outright. The wings of both jets bend on the
  model, some 36 cm at the tips at 9 g through a 3.5 Hz bending mode that
  lags and swings past a sudden pull, and the stores ride with them; the
  airframe buzzes and the cockpit shakes as the airspeed nears the placard.
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
  F-16C/D reference layout with two square MFDs and an analogue centre stack,
  an ICP below the HUD with the DED to its right, RWR scope, standby ADI,
  airspeed and altimeter, RPM / FTIT / nozzle / oil gauges in round bezels,
  caution panel, AOA indexer, master caution, VVI, fuel quantity, fuel flow
  and threat warning prime lights, the outboard thirds of the panel folded 30
  degrees toward the pilot and, beyond them, the auxiliary consoles folded
  further from knee height (landing gear handle and lights, ALT GEAR, hook,
  flaps, brakes and jettison on the left; hydraulic, EPU, oxygen and cabin
  dials, the clock and the warning lights on the right), side consoles after the F-16C
  Block 50 layout (every Dzus lightplate in its real place and size, with
  sharp legends and chunky toggles, guards, knobs and trim wheels, the tops
  tilted 10 degrees toward the pilot), the throttle in its quadrant and the
  pressure sidestick on its mount, beside the pilot and a little forward of
  the reference positions so they show (the default view looks 14 degrees
  down: HUD, panel and sky in
  view, the stick and throttle heads at the bottom corners), a reclined
  ACES II seat, the canopy jettison handle and map case on the walls, and the
  pilot's arms, hands and legs following the live controls.
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
  The F-16 adds CAS and altitude tapes, heading and steerpoint cues, a 5/10-degree pitch ladder, a drift-corrected FPM marked X when limited, and the landing AoA bracket; altitude readouts omit leading zeroes.
- Retro presentation: the world renders at a low internal resolution (360 to
  576 lines) shown as crisp blocks, quantised to 15-bit colour with a 4x4
  ordered dither; the terrain wears pixel tiles (grass, scrub, rock strata,
  snow, sand, patchwork farmland with hedgerows and the Cold River) with
  Gouraud lighting and dithered material borders, under a clear blue sky
  with screen-door clouds. The HUD and menus are drawn crisp at native
  resolution in a 5x7 pixel font.
- Procedural audio synthesised on its own thread: engine, wind, rumble and
  turbine whine follow throttle, speed and afterburner, plus warning tones,
  the gun's buzz, the Sidewinder's growl and lock tone, launches, blasts and
  the RWR. Engine pitch follows spool RPM; independently filtered noise and
  drifting turbine partials avoid a repeating engine loop. The MiG uses two
  independent sources. Airframe-specific voice warnings have priorities,
  repeat delays and engine ducking; F-16 low-speed and gear warnings use tones.
  Voice assets are original speech synthesis, not recordings from an aircraft
  or another game. See [assets/voices/README.md](assets/voices/README.md).

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

## Recording and screenshots

Fullscreen uses a borderless desktop window. Windows capture, recording and
vendor-overlay shortcuts are left to the OS; opening a system overlay or
switching to another app pauses the flight, frees the cursor and clears held
controls. Returning to the game leaves it paused until P or Esc is pressed.
The view stays visible and the camera orientation is preserved.

F12 also saves the composed frame directly from OpenGL to
`Pictures/viper-run/viper-run-<timestamp>.png`, without requiring an overlay.
P freezes the scene while keeping camera controls available; Esc pauses and
releases the mouse. K opens the configuration drawer.

The executable no longer exports `RTSSHooksCompatibility = 0`, which previously
disabled RivaTuner's overlay and frame limiter. RTSS and NVIDIA capture hooks
are allowed. The built-in overlay works independently of these tools.

For OBS, use Window Capture with **Windows 10 (1903 and up)** (Windows Graphics
Capture), or Display Capture. BitBlt can return a black image with OpenGL.
These capture methods also avoid competing with RTSS for the OpenGL
`SwapBuffers` hook. Game Capture can still be used, but OBS and third-party
OpenGL overlay hooks can conflict on some driver/tool combinations; see
[OBS's overlay conflict guidance](https://obsproject.com/kb/known-conflicts).
The game does not change external application profiles or Windows settings.

Check a rebuilt executable with
`python3 tools/check_windows_capture.py dist/viper-run-windows-x86_64.exe`.
This verifies that the final PE does not disable external overlays.

The PERFORMANCE panel sits below the left-side air-data block in both camera
views. It shows the rendered frame count, FPS averaged over the last 60 frames,
the last frame's elapsed time in milliseconds, and 180-frame FPS and frametime
histories. These use wall time, so they continue updating while flight is
paused and include presentation pacing rather than fixed simulation steps.
CPU, GPU and physical RAM are system-wide usage, sampled every 750 ms on a
separate thread. Windows uses GetSystemTimes, PDH GPU Engine counters (the
busiest physical engine, aggregated across processes) and GlobalMemoryStatusEx.
Linux uses /proc and the DRM gpu_busy_percent counter where exposed by the
driver. Unavailable metrics show N/A. Disable PERFORMANCE OVERLAY in K config
to stop both drawing and hardware sampling; the choice is saved. For automated
captures, `--performance` and `--no-performance` override it for that run.

## Build

Windowing uses [miniquad](https://crates.io/crates/miniquad), a tiny
OpenGL layer (no engine). Multiplayer adds Serde/Postcard serialization,
RustCrypto authenticated encryption and system randomness. On Linux miniquad loads `libGL` and `libX11`
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
viper-run --ui-check --screen menu --shot menu-check.png --frames 200 --jet mig29 # input/menu integration check
viper-run --audio-demo engine.wav --jet mig29 # render the runtime mixer without a window
viper-run --bench 15 --size 1920x1080      # real-time frame timing, vsync off
viper-run --shot out.png --mach 0.98       # start at a Mach number (vapor cone, shocks)
viper-run --shot out.png --vision 0.5,0,0  # pilot state: retinal oxygen, redout, G-LOC s
viper-run --shot out.png --mission dogfight --sim 20 --input lock   # gates | dogfight | strike | airfield | free
viper-run --shot out.png --mission strike --input auto --sim 20     # AI flies to the steerpoint
viper-run --shot out.png --weapon gun --input fire --sim 1          # nav | gun | aim9 | aim120 | mk82
viper-run --shot out.png --mission strike --screen loadout          # also shotdown
```

## Project layout

```
viper-run/
├── Cargo.toml          # windowing, network codec and authenticated encryption
├── build.sh            # release binaries for Linux and Windows
├── dist/               # prebuilt executables
├── docs/F16-REFERENCES.md
└── src/
    ├── main.rs         # game states, input, cameras, frame composition
    ├── sim.rs          # 6-DOF rigid body, FLCS, engine, landing gear, collision, mission
    ├── physio.rs       # pilot G tolerance, G-LOC, redout, Auto-GCAS
    ├── structure.rs    # wing aeroelasticity, flutter, divergence, load limits
    ├── vapor.rs        # condensation, vapor cone, shock cones, sonic boom
    ├── weapons.rs      # gun, missiles, bombs: ballistics, guidance, sights, decoys
    ├── combat.rs       # loadouts, master modes, lock, bandits, targets, damage, missions
    ├── ai.rs           # AI pilot: BFM, weapons employment, terrain recovery
    ├── targets.rs      # STRIKE ground targets and their models
    ├── scenery.rs      # airbase buildings, villages, trees, power line
    ├── props.rs        # combat models, smoke trails, tracers, effects and sounds
    ├── symbology.rs    # HUD weapon symbols, helmet display, FCR, RWR, overlays
    ├── aero.rs         # F-16 aerodynamic tables and F110 thrust tables
    ├── terrain.rs      # analytic terrain, airbase, height grid, LOD chunk meshes
    ├── world.rs        # sky, terrain, ocean, clouds, gates, aircraft, pixel pass
    ├── texture.rs      # procedural pixel-art terrain and cloud textures
    ├── airframe.rs     # F-16 exterior, moving surfaces, landing gear and painted skin
    ├── mig29.rs        # MiG-29 exterior
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
    ├── keymap.rs       # remappable flight keys
    ├── math.rs         # vectors, quaternions, matrices
    └── png.rs          # screenshot writer
```
