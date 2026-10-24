# MiG-29 cockpit

Revised 23 October 2026. The MiG-29S (9.13S) cockpit is its own, not a copy
of the F-16's. The photos are used as visual reference and are neither
included as textures nor redistributed.

## Visual references

Photos of real 9.12 cockpits on Wikimedia Commons (the 9.13 kept the same
cockpit almost unchanged):

- [MiG-29 cockpit 3](https://commons.wikimedia.org/wiki/File:MiG-29_cockpit_3.jpg):
  the whole cockpit from above. It sets the general layout: blue-grey panel
  and tub, centre stick between the legs, side consoles at thigh height, the
  ILS-31 HUD with its heavy frame on the black coaming, the IT-23 display to
  its right and the column of warning lights by the right canopy rail.
- [Polski MiG-29 kokpit](https://commons.wikimedia.org/wiki/File:Polski_mig_29_kokpit.JPG):
  the panel of an upgraded Polish MiG-29; black-faced round instruments, the
  KPP (attitude director) over the PNP (horizontal situation) in the middle,
  airspeed and altimeter on the left.
- [MiG-29 cockpit 1](https://commons.wikimedia.org/wiki/File:MiG-29_cockpit_1.jpg):
  the stick grip: black and chequered, its head swelling forward, the red
  weapon button, the trim hat and the wheel brake lever in front.
- [MiG-29 cockpit 2](https://commons.wikimedia.org/wiki/File:MiG-29_cockpit_2.jpg):
  the right panel in detail: twin exhaust gas temperature in x100 °C,
  hydraulic and oxygen gauges and the intake ramp position.
- [MiG-29 cocpit 1](https://commons.wikimedia.org/wiki/File:MiG-29_cocpit_1.jpg)
  and [cockpit 114th brta](https://commons.wikimedia.org/wiki/File:-29_cockpit_114th_brta.jpg):
  the heavy canopy arch with its three rear-view mirrors (big, rounded and
  convex) and the IRST ball ahead of the windscreen, to the right.

## Layout in the game

The cockpit is built in the same space as the F-16's, with the design eye at
the same point, and `cockpit::body_camera` carries the camera from the MiG's
eye (`mig29::EYE`) to it; so the seat, the pilot's body, the legs and the HUD
are shared. From the seat the exterior cuts the skin away in
`mig29::CUTOUT` and leaves the canopy out, so nothing blocks the view ahead
but the nose and the IRST ball.

- Panel tilted 12 degrees. Its broad flat flight/engine face ends at
  x = +/-0.29 m; only the outer shoulders fold 22 degrees toward the pilot.
  KPP above PNP sits left of the HUD axis, with IAS and barometric altitude
  further left. The UUAP and caution lamp stand on the raised left cheek
  beside the HUD. VVI, radio altitude and twin-pointer RPM occupy the upper
  right row; a separate airspeed/Mach instrument and two individual EGT
  gauges sit below. Clock below VVI, rectangular hydraulic/oxygen block
  below the engines, vertical fuel tape at the right edge, SPO-15 below it.
  The gear/flap panel sits below the left flight instruments; SAU/navigation
  controls are on the left console.
- Console inboard edges are parallel at x = +/-0.262 m, clearing the seat
  bucket. The panel is 15 cm farther forward to clear the unchanged pilot's
  knees through full rudder travel, preserving the HUD/canopy datums.
  Switch panels lie on the console's actual surface with an
  orthonormal frame, and short joining panels close the gap to the main
  panel. The floor is below the boot soles and pedal blocks. Warning lamps
  sit beside the panel under the right rail, rather than below the knees.
- Layout checked against the real photographs above and Figure 15, printed
  page 21, of the [Eagle Dynamics MiG-29 manual](https://www.digitalcombatsimulator.com/upload/iblock/463/DCS%20MIG-29%20Flight%20Manual%20EN.pdf).
  Dimensions and angles remain visual estimates. The supplied modernized
  cockpit photograph guides tub proportions and console orientation; the
  instruments retain the game's analogue 9.13S configuration. Both EGT
  gauges and RPM pointers share the flight model's single engine readout.
  Regression tests check instrument mounting clearances, console frames,
  boot/floor clearance and separation of atlas regions from the mirrors.
- Black coaming with the ILS-31 HUD (a big body and a heavy frame round the
  combiner glass) and the IT-23 display with a B-scope radar picture.
- A column of ten warning lights by the right rail; the legends are
  transliterated because the game font only has Latin characters.
- Centre stick on the floor, with a boot, and a two-lever throttle in a
  quadrant on the left console; gear handle on the left wall, ahead of the
  throttle.
- Canopy frames on the exterior shape (`mig29::canopy_point`): windscreen
  posts, the arch over the knees and the rails. Three separate mirrors sit
  on the arch: a broad rounded pane across the crown and two elongated
  panes following its shoulders down to the sills, matching the supplied
  cockpit photograph. Shallow dark housings and paired brackets attach
  them to the arch; the reflected horizon stays level in the side panes.
  A bulkhead sits behind the seat, with decking under the rear canopy.

## Tail airbrake

The 9.13S keeps two airbrake panels, opening above and below the fixed
braking-parachute container between the engines. The broad panels have
U-shaped aft cutouts around the container. The upper panel is larger;
this is the split arrangement of the 9.12/9.13 family.

- [DCS MiG-29 flight manual, General Design, page 10](https://www.digitalcombatsimulator.com/upload/iblock/463/DCS%20MIG-29%20Flight%20Manual%20EN.pdf):
  upper panel 0.75 m2 and 56 degrees, lower panel 0.55 m2 and 60 degrees.
- [Gabor Toth's MiG-29 walkaround, photo 27](https://www.arcair.com/awa01/101-200/awa144-MiG-29-Toth/27.shtm):
  original rear photograph of the open panels, their curved aft cutouts,
  stiffeners and the fixed chute container. Used as a shape reference;
  the photograph is not bundled or redistributed with the game.

`mig29.rs` fits the panel dimensions to these areas, with a 0.84 m span,
curved inner and outer skins, edge thickness and internal ribs. The shape
is an approximation from the photograph and published areas, not a factory
measurement. The fixed fuselage ends at the hinges; sidewalls and a
separate chute tube remain when the panels open, so there is no duplicate
fixed panel underneath. Geometry tests check panel areas, hinge stability,
opening direction and clearance from the container throughout deployment.
The toggle messages are `AIRBRAKE DEPLOYED` and `AIRBRAKE RETRACTED`.

## MiG-29S 9.13S stores

Published figures the game uses (`weapons.rs`, `combat.rs`):

- GSh-30-1 (9A-4071K) gun in the left LERX root: 150 rounds of 30 x 165 mm,
  1,500 to 1,800 rounds a minute, the 390 g OFZ at 860 m/s.
- R-73 (izdeliye 72, AA-11 Archer): 105 kg, 2.90 m, 170 mm; cooled
  all-aspect infrared seeker 45 degrees off the nose, cued by the Shchel-3UM
  helmet sight; 7.4 kg rod warhead and a radio fuze.
- R-60M (izdeliye 62M, AA-8 Aphid): 44 kg, 2.09 m, 120 mm; a 20 degree
  infrared seeker; 3.5 kg rod warhead. For close combat only.
- R-27R (izdeliye 470R, AA-10 Alamo-A): 253 kg, 4.08 m, 230 mm; inertial,
  then semi-active on the N019M radar, which must hold the target to
  impact; 39 kg rod warhead; up to 73 km head on at height.
- R-27T (izdeliye 470T, AA-10 Alamo-B): the R-27R with a passive infrared
  seeker; it must acquire the target before launch.
- R-77 (RVV-AE, AA-12 Adder): 175 kg, 3.60 m, 200 mm; inertial with the
  N019M's datalink and active radar at the end; lattice fins; 22.5 kg
  warhead. The 9.13S's N019M radar is what lets it carry the missile.
- FAB-250M-62: 250 kg, 1.92 m, 300 mm, 100 kg of TNT. FAB-500M-62: 497 kg,
  2.47 m, 400 mm, about 200 kg of TNT.
- Two BVP-30-26M dispensers of 30 26 mm cartridges (PPI-26 flares, PPR-26
  chaff).
- Six underwing pylons: R-73s or R-60Ms outboard; those, R-77s or FAB-250s
  on the middle pair; R-27Rs, R-27Ts, R-77s or bombs on the inner pair; the
  default air-to-air load is four R-73s and two R-77s. Each missile's lock
  rules are in docs/WEAPONS.md.

Sources: [R-27 (Wikipedia)](https://en.wikipedia.org/wiki/R-27_(air-to-air_missile)),
[FAB-500](https://en.wikipedia.org/wiki/FAB-500),
[FAB-250](https://en.wikipedia.org/wiki/FAB-250),
[FAB-500M-62 (GlobalSecurity)](https://www.globalsecurity.org/military/world/russia/fab-500m-62.htm).

## MiG-29 flight model

src/aero_mig29.rs gives the Fulcrum its own lift, drag, pitch and engine
data. No wind-tunnel tables of the type are public, so the tables are fitted
to these published figures, and its tests check that the model still meets
them:

| Figure | Published | Model |
|---|---|---|
| Zero-lift drag Cx0 | 0.022 | 0.022 |
| Lift Cy at 24 degrees AoA | 1.43 | 1.43 |
| Oswald factor at small lift | 0.87 | 0.87 (K = 0.108) |
| Sustained g, Mach 0.9, 5,000 m, 1,500 kg fuel | 6.6 | 6.8 |
| Speed gain in a 6 g afterburner turn, 1,000 m, 900 km/h | about 20 km/h/s | 20.3 km/h/s |
| Top speed at sea level | 1,500 km/h (Mach 1.26) | Mach 1.28 |
| Top speed at altitude | 2,450 km/h (Mach 2.3) | Mach 2.32 at 11 km |

Other inputs: 38.06 m² wing, 11.36 m span, 42 degree sweep; a 3.73 m mean
chord worked out from the planform; about 40 % of the lift from the body;
two RD-33s of 5,040 kgf dry and 8,300 kgf in afterburner, 0.77 and 1.85
lb/lbf/h, following the F110's lapse tables behind variable-ramp inlets;
9 g below Mach 0.85 and 7 g above. Lateral-directional data and rate damping
are still the F-16's, on the MiG's span and chord, and the static margin
(slightly stable subsonic) and stabilator power are estimates from the tail
geometry.

The drag polar, Cy and turn-chart figures come from the MiG-29A practical
aerodynamics manual as its charts are quoted in public discussions; the
manual itself was not available.

Sources: [MiG-29 (Wikipedia)](https://en.wikipedia.org/wiki/Mikoyan_MiG-29),
[МиГ-29 (Russian Wikipedia)](https://ru.wikipedia.org/wiki/МиГ-29),
[Klimov RD-33 (Wikipedia)](https://en.wikipedia.org/wiki/Klimov_RD-33),
[F-16 vs MiG-29 energy manoeuvrability (F-16.net)](https://www.f-16.net/forum/viewtopic.php?f=30&t=53852).
