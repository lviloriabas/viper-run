# F-16 cockpit and flight model

Revised 23 October 2026, native Rust version. The flight base is still the
F-16C Block 50 with the F110-GE-129 engine, ported line by line from the web
version. The cockpit follows the user's modernised image, with two vertical
side displays and a centre CPD, as in the F-16V upgrade, but it is now real
3D geometry at realistic distances from the design eye. The modelling is
procedural, with smooth surfaces and a retro finish through resolution,
colour and textures. The photos are used as visual reference and are neither
included as textures nor redistributed.

## Visual references

- The four images the user supplied: the layout of the modernised cockpit
  and top, front and side views of the jet. They are not turned into
  textures or shipped as game files.
- [Lockheed Martin's description of the F-16V](https://lockheedmartin.com/en-us/news/features/2016/meet-the-f-16v--the-most-technologically-advanced-4th-generation.html).
  It confirms the 6 by 8 inch centre CPD of the upgrade.
- [f-16.net cockpit and HUD gallery](https://www.f-16.net/g3/f-16-photos/album03/album72).
  General layout, proportions, consoles and the placement of the controls.
- [Front view of an F-16C/D Block 40, USAF photo](https://www.f-16.net/g3/f-16-photos/album03/album72/aaa).
  Useful to study how the instruments, legs and hands relate. It is a Block
  40; its HUD is not taken as an exact reference for the Block 50.
- [Throttle, Martin Ingold](https://www.f-16.net/g3/f-16-photos/album03/album72/aak).
  The left control, its grip and the console.
- [Throttle grip diagram, Wikimedia Commons](https://commons.wikimedia.org/wiki/File:F-16_Throttle.jpg)
  and [1:1 Thrustmaster Viper TQS replica](https://shop.thrustmaster.com/en_gb/viper-tqs.html).
  They set the shape of the game's throttle: a lever at the outboard edge of
  the left console carries a grip that runs inboard, rising about 15 degrees,
  from a thick bulb by the lever, through a waist, to the switch head at the
  inboard end. On top of the head sit the MAN RNG/UNCAGE knob, DOG FIGHT and
  the communications switch; at the end, the ANT ELEV wheel; on the aft face,
  the radar cursor and the speedbrake. The cutoff lever stays outboard. The
  left hand rests flat on top, in line with the forearm: a single glove piece
  falling over the front, the thumb along the aft face toward the cursor.
- [Side stick, Martin Ingold](https://www.f-16.net/g3/f-16-photos/album03/album72/aal).
  The right grip and the arm rest.
- [F-16C HUD, USAF photo](https://www.f-16.net/g3/f-16-photos/album03/album72/aad).
  Colour, transparency and how the symbols sit on the landscape. The photo
  shows the wide-angle HUD of the Block 40/42; the game uses a compact
  adaptation, not a reproduction of that unit in a Block 50.
- [Block 50/52 description on f-16.net](https://www.f-16.net/f-16_versions_article9.html).
  Engine variant and reference flight figures. The present panel layout is
  inspired by the later upgrade, not by that Block's basic panel.

### Side consoles

Sources:

- [T.O. 1F-16CM/AM-1, Falcon BMS 4.37 edition](https://cdn.falcon-bms.com/docs/4.37/TO%201F-16CMAM-1%20BMS.pdf),
  section 1.5 (the figures of the left console, auxiliary consoles and right
  console). It sets the order of the panels, their columns and which are
  turned.
- The FlightGear F-16 cockpit model
  ([NikolaiVChr/f16](https://github.com/NikolaiVChr/f16), `Models/Cockpit/Main/`
  and `cockpit.xml`), with the pilot's eye at view 0. The boxes of each
  panel, the stick and the throttle were measured against the eye. It gives
  the heights, the tilt and the widths.
- [Standard Dzus panel](https://shaddack.mauriceward.com/notes/panels.webt):
  5.75 inches wide and heights in steps of 3/8 inch.
- [Henkie's ViperPit](https://pdp-11.nl/viperpit/), real panels fitted in a
  left console: the look, colour and proportions of EXT LIGHTING, ECM, EPU
  and ENG & JET START.

Measurements against the design eye (FlightGear): the top face of both
consoles is 0.74 to 0.77 m below the eye at the inboard edge; it is tilted
10 degrees toward the pilot (outboard edge higher) and its forward section
rises about 4.6 degrees forward. The inboard edge is 0.26 m from the
centreline, beside the seat, and the outboard one about 0.58 m. The console
is 0.86 m long; the panels fill the first 0.74 m. The side stick grip is
0.32 m ahead of the eye, 0.29 m to the right, with its top 0.50 m below; the
throttle, at mid travel, 0.19 m ahead, 0.41 m to the left and 0.59 m below,
its lever in a slot by the outboard edge of the left console.

Left console, front to back (height x width in inches): an inboard column of
UHF (4.875), AUDIO 1 (3), AUDIO 2 (1.5), ECM (2.625) and AVTR (2.25), all
5.75 wide; an outboard column 3 wide with MANUAL PITCH (2.625), ENG & JET
START (7.875) and ELEC (3.75); EPU, 2.125 wide, beside ELEC; outboard of
them, the throttle quadrant. Behind, panels turned 90 degrees (read from
outboard, with the 5.75 running fore and aft): EXT LIGHTING (3.375), IFF
(3.75) and FUEL (2.25); then MANUAL TRIM (4.125) and FLT CONTROL (4.875);
last ANTI-G and TEST (3).

Right console: the stick on its mount at the inboard front corner; behind
it, in the inboard strip, ZEROIZE and VOICE MESSAGE; a main column with SNSR
PWR (1.5), HUD (2.625), LIGHTING (5.625) and AIR COND (2.25); NUCLEAR
CONSENT in the outboard strip. Behind, turned: OXYGEN REGULATOR (3.375) and
SECURE VOICE (3.75); AVIONIC POWER (4.125) with ENGINE ANTI ICE and ANT SEL
(1.5); and the DTU (4.5). The gaps carry plain blanking plates.

Each panel carries its real legends (switches, rotary knobs, push buttons,
lights and displays) drawn in the right half of the instrument atlas (2048
x 2048, at 36 pixels per inch; the 5x7 font comes out about 0.19 inch, half
again the real size so it reads) and its controls in geometry: toggle
switches with a hex nut, red or black guards, knobs with an index line, push
buttons, trim wheels and the three levers of the oxygen regulator.

The consoles, the stick and the throttle sit where they really are against
the eye: the stick 5 cm behind the front edge of the right console, on a
mount of about 9 cm, and the throttle 27 to 7 cm behind the edge of the left
one, its quadrant housing only 2 to 4 cm above the console. So the hands sit
50 to 62 degrees below the horizon, largely under the pilot's shoulders and
arms. The default view looks 14 degrees down: the HUD, the panel and a good
deal of sky stay in view, and the heads of the stick and throttle show in
the lower corners. The stick, the throttle and the consoles are 12 cm
further forward than in the FlightGear model, so they look as they do from
the real seat. The default tilt is a single constant, `LOOK_PITCH` in
`src/main.rs`. A test pins the reference positions of the stick and the
throttle. The outer thirds of the main panel fold 30 degrees toward the
pilot on vertical hinges beside the MFDs. Under them, outboard of the knees,
the auxiliary consoles carry each side console forward on a 45 degree ramp,
with its same 10 degree tilt and turned 7.3 degrees toward the pilot, as in
the FlightGear model (`panels_mlu_left-aux-console` and `right-aux-console`
in `cockpit.xml`) and the cockpit photos on Wikimedia Commons; a shelf joins
them to the panel below the line of sight to the MFDs. The left one carries
the gear (a handle with a red light in transit, green NOSE LEFT RIGHT
lights, ALT GEAR, hook, ALT FLAPS, brakes, landing lights, STORES CONFIG and
emergency jettison), and the right one the A and B hydraulic pressure, EPU
fuel, oxygen, cabin altitude, the clock and the warning lights (ENG FIRE,
ENGINE, HYD/OIL PRESS, FLCS, DBU ON, TO/LDG CONFIG, CANOPY, OXY LOW). Under
the UFC sit in a row the standby airspeed indicator, ADI, altimeter and
accelerometer, as in the F-16C. Under the MFDs are the VVI and the fuel
gauge, over the columns the TWP and the fuel flow, and on the walls the
canopy jettison handle, the map light, the canopy switch and the map case.

The [USAF fact sheet on the F-16C](https://www.jba.af.mil/About/Fact-Sheets/Display/Article/4282145/f-16c-fighting-falcon/)
confirms the seat reclined 30 degrees, the force-sensing side stick, the
canopy's visibility and the 9 g capability. The design eye is at
(0, 1.25, -3.24) m in body axes, behind the hips because of the seat's
recline and inside the canopy. The panel is about 0.9 m from the eye, the
HUD combiner glass about 0.75 m, and the UFC (the DED over the ICP keypad)
just under the HUD. The two vertical MFDs flank the centre CPD; on the left
are the RWR, the standby airspeed indicator and altimeter, and on the right
the standby ADI and the round RPM, FTIT, nozzle and oil gauges, each in its
round bezel. The canopy is a single convex layer of slightly tinted glass
with Fresnel sheen and sun glints, so each pixel crosses it only once. The
coordinates are approximate; they do not come from a scan or a dimensioned
drawing.

The default FOV drops from 78 to 58 degrees, adjustable between 45 and 72.
The default mouse sensitivity is 2.8 times the original. K opens the left
drawer and pauses the flight while settings change; H puts the speedbrake
out or in. Pitch comes from a single input, chosen in that drawer: the
mouse (default) or W/S; the other one is inactive. Inversion applies to the
mouse's vertical axis; S always pulls and W pushes. The mouse's X axis
works the rudder and springs back to centre.

The FLCS pitch law commands load factor: on top of the pitch rate that load
factor produces, it adds an angle-of-attack term that builds the lift in
about 0.3 s, rather than waiting for the jet's own response of about 1 s. A
full pull at the start speed reaches about 6 g in 0.4 s and 9 g in under a
second; on release it is back at 1 g in half a second. With the stick
centred the steady state is the same flight-path hold as before, and the
command never takes the jet past about 16 degrees of AOA on its own.

The exterior is a fuselage of superelliptic sections with an ogive radome,
a ventral D intake with its boundary-layer splitter, LERX blended into the
wing, a 40 degree swept wing with a 4 % NACA section, a fin of about 47
degrees with the drag chute housing, all-moving stabilators with 10 degrees
of anhedral, canted ventral fins, a petal nozzle and a gold-tinted bubble
canopy. Stabilators, rudder and speedbrakes move with the controls. The tests
measure the real mesh: 15.06 metres long, 9.96 metres span over the
wingtip AIM-9s and 5.09 metres tall from the gear line. The ten collision
contacts (nose, canopy, tail, wingtips, intake, belly, fin and stabilator
tips) follow that mesh. The cockpit remains a visual adaptation for a game.

## How the instruments map to the simulation

| Instrument | Data or behaviour |
| --- | --- |
| HUD and airspeed indicator | Calibrated airspeed in knots, from impact pressure and the ISA atmosphere |
| Mach | True airspeed over the local speed of sound |
| Altimeter and B on the HUD | Altitude above mean sea level, in feet |
| R on the HUD | Height above the terrain under the jet's centre, in feet |
| ADI and pitch ladder | Attitude from the quaternion and an inertial horizon |
| FPM | The real direction of the velocity vector, projected from the camera |
| HSI and heading | The model's true heading; magnetic variation is not modelled |
| G and maximum G | Normal specific force and the maximum shown at the end |
| AOA | Angle between the flow and the longitudinal axis |
| HSD | The real route, the jet's heading and the next gate as the steerpoint |
| DED | Fixed radios for atmosphere, steerpoint, heading, distance, time and fuel |
| RPM, FTIT, nozzle and oil | Estimates from the engine's power and the afterburner |
| Fuel and fuel flow | Mass burnt by the model, converted to pounds and pounds per hour |
| Warnings | Low fuel, engine out of fuel, AOA, G, Mach overspeed, terrain proximity and battle damage |
| Hands and pedals | Smoothed control inputs; the left hand follows the throttle travel |
| FCR | Contacts within ±60° and the scale (10 to 80 NM), with aspect; the locked target with its speed and altitude |
| RWR | Bandit radars (16 or 29), AAA (A) and the SA-6 radar (6), a diamond when they lock or fire, M for a missile in flight |
| MASTER ARM switch | SAFE in NAV, ARM with a weapon selected |

The HUD shows NAV mode and, with a weapon selected, the GUN, SRM, MRM and
CCIP master modes described in the weapons section. The FCR shows the air
contacts (or the ground targets in CCIP) and the RWR the mission's threats;
the gate run has no contacts or emitters. ILS, INS navigation with errors,
TACAN and communications are not simulated. The ICP buttons and console
switches are decorative; the MFD pages change with F1 and F2. The AGL
readout leaves out a real radar altimeter's beam, attitude limits and
failures.

## Parameters and calibration

### Flight model

The model follows the FlightGear F-16 (NikolaiVChr/f16 repository, JSBSim):

- Aerodynamics: the NASA TP-1538 tables (Nguyen et al., 1979) in the reduced
  form of Stevens and Lewis, *Aircraft Control and Simulation*: CX, CZ and
  Cm against AOA and stabilator; Cl and Cn against AOA and sideslip;
  flaperon and rudder effects; damping derivatives. It is the same data set
  JSBSim uses in full. The tables were checked against the public
  AeroBenchVV transcription.
- From FlightGear (`Systems/jsb-aerodynamics-block15-subtle.xml`): wave drag
  against Mach, the transonic shift of the aerodynamic centre (Nikolai V.
  Chr.'s estimate) and ground effect.
- Block 50 mass and inertias (`Systems/jsb-mass-50.xml`): 19,200 lb empty,
  Ixx 9,007, Iyy 49,956, Izz 56,770 and Ixz 198 slug·ft²; the pilot, the
  wingtip missiles and the fuel (7,000 lb internal), which burns off, are
  added.
- F110-GE-129 engine (`Engines/F110-GE-129.xml`): 17,000 lbf military and
  29,000 lbf in afterburner, thrust tables against Mach and altitude, TSFC
  0.661 and 1.9. The power dynamics are Stevens and Lewis's. The pressure
  recovery of a fixed normal-shock intake is added, which is what holds the
  F-16 near Mach 2.
- FLCS: the same command paths as `Systems/jsb-controls.xml` (stick to G,
  +9/-3 G limits, the AOA limiter from 22.5° with a stop near 25°, a roll
  rate command of up to 308 °/s reduced with AOA, the aileron-rudder
  interconnect, the yaw damper) with actuators limited in travel and rate.
  The inner pitch and roll loops invert the aerodynamic tables themselves
  instead of reproducing the analogue gain network, so the response is
  stable from 120 knots to Mach 2. The stick does not use the real force
  gradients, nearly dead over the first fifth of the travel and with only
  50 °/s of roll at half stick: power curves with the same end points
  (8.5 g and -4.2 g about 1 g, 308 °/s) and a 3 % dead zone give authority
  from the keyboard, the mouse or a controller.

`cargo test --release level_speed -- --nocapture` solves the level-flight
balance in full afterburner with half fuel:

| Altitude | Maximum level Mach |
| --- | --- |
| Sea level | 1.24 |
| 10,000 ft | 1.47 |
| 20,000 ft | 1.68 |
| 30,000 ft | 1.90 |
| 40,000 ft | 2.02 |

Limitations: there is no leading-edge flap schedule nor stall tables past
45° AOA (a flat-plate nose-down moment is added), the visual model's centre
of gravity is somewhat forward of the real one, and thermal limits and wind
are not modelled. External stores change the mass and drag but not the
inertias or the stability. Battle damage lands on the systems a round or a
fragment actually strikes (`src/damage.rs`; see the README).

### Exterior proportions

The stations were measured on the three-view on Wikimedia Commons
(*General Dynamics F-16 Fighting Falcon 3-view line drawing*), scaled to
15.06 m: the windscreen at 17.8 % of the fuselage length, the end of the
canopy at 41 %, the intake lip at 29 %, the wing trailing edge at 75 %, and
a canopy-to-intake-floor depth of 0.15 times the length. The engine axis,
the stabilators and the fin sit above the nose line. The span and height use
the published figures (9.96 m with AIM-9s and 4.88 m), because the drawing
is not exactly to scale in those dimensions.

### Pilot G tolerance

`src/physio.rs` works out the blood pressure at eye level with a
hydrostatic column: with the seat reclined 30 degrees the eye sits about
28 cm above the heart, and each G takes about 22 mmHg (blood at
1,060 kg/m³) off a mean pressure of 100 mmHg at the heart. Protection adds
up from the CSU-13B/P anti-G suit (about 30 mmHg, 1.4 G, inflating from
2 G), COMBAT EDGE pressure breathing (25 mmHg, 1.1 G, from 4 G), the AGSM
straining manoeuvre (65 mmHg, 3 G, which the pilot exhausts in about 50 s of
maximum effort and recovers in a minute of rest) and the baroreflex
(+20 mmHg in about 6 s; -30 mmHg in 2 s after negative G, the push-pull
effect). The retina stops being perfused between 30 and 0 mmHg and the
brain between -5 and -25 mmHg; each keeps oxygen for 3 and 5 s. With Burton
and Whinnery's centrifuge data as the reference, the relaxed, unprotected
pilot loses peripheral vision at about 4.9 G with a gradual onset
(0.1 G/s), blacks out at 5.5 G and loses consciousness at 6.9 G; with a
rapid onset to a plateau, the view closes 0.7 G earlier. The trained pilot
holds 9 G for 15 s with the view somewhat narrowed, and passes out if it
lasts more than a minute. The length of a G-LOC depends on the time under
G: if the load comes off before the brain's seconds of oxygen run out
nothing happens; if they run out, a short peak costs about 3 s of absolute
and 2 s of relative incapacitation, nobody wakes while the G stays on, and
each further second of oxygen debt stretches the episode up to 20 s and
12 s (centrifuge runs have measured 2 to 38 s, averaging 12 s and 12 s).
During the relative incapacitation the controls come back little by little.
While the pilot cannot fly, Auto-GCAS (in F-16 service since 2014) predicts
a recovery (half a second to react, a roll at 180 degrees a second to wings
level and a 5 g pull) and flies it if its path would pass within 60 m of
the terrain. Negative G reddens the view from -1.5 G and fully at -3 G. The
game draws the grey, the tunnel, the blackout and the red veil over the
scene and the HUD, dithered on the scene's pixels; the sound cuts out
during a G-LOC and the AGSM breathing is heard every 3 s.

### Aeroelasticity and structural limits

`src/structure.rs` uses the clean F-16C placard, 800 KCAS (equal to KEAS at
sea level) or Mach 2.05, and the MIL-A-8870 margins. The real figures are
not public; these are consistent estimates:

| Phenomenon | Equivalent airspeed |
| --- | --- |
| Placard limit | 800 KEAS |
| Flutter (15 % over the limit) | 920 KEAS, with a 14 % transonic dip between Mach 0.9 and 1.1 and 28 % less with 1,500 kg under the wings |
| Flaperon reversal | 1,100 KEAS |
| Wing torsional divergence | 1,280 KEAS, less with compressibility (the Prandtl-Glauert factor on the Mach normal to the sweep) |

The 40 degree swept wing bends up under load and the sweep turns that
bending into nose-down twist (wash-out): at the limit's dynamic pressure it
loses 8 % of its lift, mostly on the outer panels, and the centre of lift
moves forward. The flaperon loses effectiveness until it reverses at
1,100 KEAS, but half the roll comes from the differential stabilator, so
the control does not reverse. The FLCS, which inverts the same tables,
makes up for these effects as its gain scheduling on dynamic pressure
would. The wing's antisymmetric bending and torsion mode with its tip
missile (8 Hz) loses damping toward the flutter speed and grows beyond it,
to a limit cycle that damages the structure and breaks the wing in seconds;
turbulence, buffet and the rough air of high dynamic pressure excite it, and
the cockpit shakes with it. The wingtip bends about 36 cm at 9 G, an estimate
on the flexible side, through a 3.5 Hz symmetric bending mode that lags a
sudden pull and swings past it; the bend and the flutter show on the exterior
model of both jets, stores included. Past 1.5 times the +9 and -3 G limits
the wing breaks; above the limit damage builds up. The HUD warns OVERSPEED,
and the outside view also STRUCT DAMAGE.

### Condensation and Mach effects

`src/vapor.rs` cools the air adiabatically where the flow speeds up, with a
pressure coefficient for each zone: the LERX vortex core (about
-12 sin²(alpha), Polhamus-style vortex suction), the wingtip vortex core (a
0.6 m Rankine core with the circulation L / (rho V b') of an elliptic wing)
and the supersonic pocket over the jet near Mach 1 (about -0.45). The
relative humidity is 60 to 70 % under the cumulus base, near 1,500 m, and
falls with altitude; the saturation pressure follows the Magnus formula,
and the visible density of the fog follows the water that condenses (grams
per kilogram of air), so high up, in cold dry air, it hardly shows. The
LERX vortex is a tube of fog over each wing root that bursts and swells
ahead of the trailing edge from about 25 degrees of AOA. The tip vortices
leave trails in hard pulls. The vapour cone fills the supersonic pocket
between Mach 0.86 and 1.04 and ends at the shock, which moves aft toward
Mach 1. Below about -40 °C (the Schmidt-Appleman criterion) the exhaust
leaves a contrail. Past Mach 1 the bow and tail shocks are drawn as faint
cones at the Mach angle, and the outside camera hears the double boom when
the Mach cone sweeps over it; the pilot, who is ahead of it, does not. The
transonic buffet follows a lift-coefficient boundary that sags toward
Mach 0.95, plus the shocks shuffling over the wing through Mach 1, and the
altimeter shows the Mach jump as the bow shock passes the static ports.

## Weapons, sensors and combat missions

Simplified, but with the published figures. The start screen picks the
mission, and every mission opens a setup screen: the airframe, the fuel and
the clock, and for the fighting missions the three station pairs (the
wingtips, stations 3 and 7, and 4 and 6), the opposition, and the gross
weight, added drag and thrust-to-weight that result. The gate run carries
the usual two wingtip AIM-9s, so its flight does not change. The missile
families, the other stores and each missile's launch rules are in
docs/WEAPONS.md.

| Weapon | Data used | Source |
| --- | --- | --- |
| M61A1 gun | 511 rounds of 20 mm, 6,000 rounds a minute, about 1,036 m/s at the muzzle, a PGU-28/B projectile of about 100 g with Cd 0.28 (about 600 m/s left at 1,000 m); the muzzle in the left wing root at (-0.72, 0.52, -3.40) m; the line of fire through the HUD's gun cross; the cases go back into the drum | [USAF F-16 fact sheet](https://www.af.mil/About-Us/Fact-Sheets/Display/Article/104505/f-16-fighting-falcon/), [M61 Vulcan](https://en.wikipedia.org/wiki/M61_Vulcan), [PGU-28/B](https://www.globalsecurity.org/military/systems/munitions/pgu-28.htm) |
| AIM-9M Sidewinder | 85.5 kg, 2.87 m, 127 mm; Mk 36 Mod 11 reduced-smoke motor, about 5 s and 10.1 kN; Mach 2.5 to 2.7; 35 G; all-aspect infrared seeker with a 40° gimbal; WDU-17/B 9.4 kg warhead with a DSU-15 proximity fuze | [US Navy fact file](https://www.navy.mil/Resources/Fact-Files/Display-FactFiles/Article/2169316/aim-9-sidewinder/), [AIM-9](https://en.wikipedia.org/wiki/AIM-9_Sidewinder) |
| AIM-120C AMRAAM | 161.5 kg, 3.66 m, 178 mm; boost-sustain motor (24 kN 2.6 s, 9.8 kN 6 s) to Mach 3 to 4; 40 G; inertial guidance on the radar's target over the datalink and an active radar seeker that searches on its own if fired without a target; WDU-41/B 18 kg warhead | [USAF AIM-120 fact sheet](https://www.af.mil/About-Us/Fact-Sheets/Display/Article/104576/aim-120-amraam/), [AIM-120](https://en.wikipedia.org/wiki/AIM-120_AMRAAM) |
| Mk-82 | 241 kg with fuze, 2.21 m, 273 mm, 87 kg of Tritonal; Cd 0.22; 1.5 m/s ejection; fuze armed after 2 s; lethal to 8 m, damage to 45 m | [Mk 82](https://en.wikipedia.org/wiki/Mark_82_bomb), [GlobalSecurity](https://www.globalsecurity.org/military/systems/munitions/mk82.htm) |
| Countermeasures | 30 chaff and 30 flare cartridges (a simplification of the ALE-47 dispensers); each press drops one of each | [AN/ALE-47](https://en.wikipedia.org/wiki/AN/ALE-47) |

The missiles fly true proportional navigation (N = 4): the lateral
acceleration is N times the closing speed times the line-of-sight rate,
plus the lift that holds off gravity. It is capped by the structural G
limit and by the normal force the body gives at its maximum angle of attack
and the dynamic pressure of the moment, so a slow or high missile turns
little. That normal force brings induced drag (N times the angle of
attack), on top of the wave drag that rises through the transonic, so a
missile that manoeuvres hard loses energy. The fuze arms after 0.5 s
(AIM-9) or 1 s (AIM-120) and bursts at the closest approach if that passes
within 9 or 12 m; damage falls off linearly with distance. The tests give
tail-chase ranges of about 8 km for the AIM-9M at 1,000 m and about 11 km
for the AIM-120C, more with altitude and head on.

- The gun sight follows the EEGS idea: virtual rounds fired every 1/30 s
  with the real ballistics make up the funnel, scaled for a 35 ft wingspan,
  and the LCOS pipper sits on that stream at the locked target's range
  (2,000 ft with no target). It is exact if the shooter keeps tracking for
  the time of flight.
- CCIP flies a whole bomb with the same step and ballistics as a real one,
  so a bomb released on the pipper lands on it (a test checks it). The fall
  line runs from the flight-path marker to the pipper.
- The AIM-9 searches a 2.5° field along the jet's boresight or where the
  helmet looks, and slaves to the radar target inside its 40° gimbal. The
  IN RNG cue comes from simulating the shot against the target's present
  track; the AI pilot uses the same check.
- A JHMCS-style helmet: turning the head off the HUD brings up an aiming
  cross, flight data, the target box and the seeker, which the helmet cues
  off the boresight. T locks the target nearest the line of sight.
- Flares only fool infrared seekers (probability 0.35, for the AIM-9M's
  rejection logic) and chaff only radar seekers, best with the target on
  the beam (between 0.08 and 0.63); neither works with the missile beyond
  6 km.

The bandits fly the same six-degree-of-freedom model and the same FLCS as
the player; the AI pilot only decides where the lift vector goes, how hard
to pull and where the throttle goes. It uses basic fighter manoeuvres: lead
pursuit for the gun, pure to close and lag to avoid an overshoot, a max-G
break when the player is at its six, beaming then breaking into a missile,
and energy recovery. Above all that sits an F-16 Auto-GCAS-style
prediction: it flies the recovery pull (a roll and 6 G) ahead of time and
flies it when it would come within 180 m of the terrain, choosing between a
straight climb or a turn if a valley wall demands it. It fires the gun
inside 900 m with a firing solution, launches AIM-9s inside the launch zone
and drops countermeasures when a missile closes.

Ground targets in STRIKE, by the river: a convoy of four trucks with a
ZSU-23-4, an SA-6 battery (2P25 launcher and 1S91 radar) with a truck and
another ZSU, the road bridge with a third, and a depot with stores and
tanks. The ZSU-23-4 uses four 23 mm guns, 3,400 rounds a minute in all,
970 m/s and about 2.5 km of effective range; it fires bursts with tracers
and tracks with a lag. The SA-6's 3M9 missile weighs 599 kg, measures
5.8 m and 335 mm, reaches Mach 2.8 on its integral ramjet, carries 57 kg
and is semi-active: it stops guiding if the radar dies. The battery only
fires if its radar sees the jet more than 300 m above the terrain, so the
valley protects. Sources: [2K12 Kub](https://en.wikipedia.org/wiki/2K12_Kub)
and [ZSU-23-4](https://en.wikipedia.org/wiki/ZSU-23-4_Shilka).

Limitations: there is no radar with real modes (RWS, TWS, ACM), IFF,
guided bombs or HARM; countermeasures do not show on the enemy's radar; the
stores' mass does not change the inertias.

## Verification

- `cargo test --release`, in `src/sim.rs` and `src/aero.rs`: surface signs,
  table interpolation, engine thrust and dynamics, hands-off 1 g trim, 9 G
  in a second without overshooting the limit, the AOA limiter at 25°,
  negative G, a coordinated roll of about 300 °/s, fuel exhaustion,
  afterburner, speedbrake, numerical stability across the envelope, maximum
  level speed, gate crossing, scoring and battle damage.
- `src/physio.rs`: the relaxed pilot's thresholds with gradual and rapid
  onset, 9 G for 15 s without G-LOC and a G-LOC after a minute, the
  push-pull effect, redout, the length of the incapacitation and Auto-GCAS
  in a dive.
- `src/structure.rs` and `src/sim.rs`: the loss of lift and roll with
  dynamic pressure, divergence always beyond flutter, the transonic dip,
  damping below flutter and wing failure above it, ultimate load; with
  1,500 kg under the wings the jet flutters at Mach 1 at 1,500 m and the
  clean one does not.
- `src/vapor.rs`: fog in a hard pull low down and not in cruise or in dry
  air high up, the vapour cone only near Mach 1, contrails only in cold
  air, and the boom only inside the Mach cone.
- `src/weapons.rs`: time of flight, speed and drop of the 20 mm round;
  swept hits against the jet's ellipsoid and boxes; damage with distance;
  every missile hits targets flying straight from behind and across; finite
  and minimum ranges, ranked as published; CCIP matches the simulated bomb;
  rounds fired at the LCOS pipper hit; each countermeasure fools only its
  own seeker type.
- `src/ai.rs`: the command law turns to a heading; the AI pilot does not
  hit the terrain in the valley for 90 s; two pilots fight for two minutes
  without crashing or breaking the limits; an ace tracks a target with the
  gun.
- `src/combat.rs` and `src/targets.rs`: stores mass and drag, the mode
  cycle, locking the target nearest the boresight, damage and kills, a
  locked AIM-9 downs a bandit, a Mk-82 released on the pipper drops the
  bridge, targets on the valley floor and armour against the gun, the
  weapon families each jet fills, the launch rules by seeker, and the
  MiG-29's IRST.
- `src/terrain.rs`: the 32 m grid matches the analytic function at the
  vertices and strays less than 6 m between them; collision uses the same
  triangles that are drawn.
- `src/cockpit/consoles.rs` and `src/cockpit/pilot.rs`: the panels do not
  overlap, fit their console and the atlas, measure multiples of 3/8 inch,
  the top faces have the reference height and tilt, and the stick and
  throttle sit at their reference position against the eye.
- `src/airframe.rs`: measured dimensions and proportions, finite geometry,
  unit normals, valid texture coordinates and the design eye inside the
  canopy.
- Screenshots with `--shot`, `--screen` and `--look` of the menu, pause,
  results, cockpit and outside view, and `--bench` to time the frames.
- Visual inspection from the front, both sides, the pilot and the tail. Alt
  and the mouse turn the head up to 155 degrees each way and 85 degrees
  vertically. Releasing Alt brings the view back forward and the outside
  camera back to its normal place. C also centres the view. The cockpit and
  the exterior share position, scale and attitude; the exterior model's
  opaque canopy is left out in first person.
