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

- Panel tilted 12 degrees, with wings folded 22 degrees toward the pilot.
  Centre: KPP over PNP; on the left the airspeed indicator in km/h with a
  Mach window, the altimeter in metres (long needle per kilometre, short per
  ten, and a counter) and the UUAP (angle of attack above, load factor
  below); on the right the variometer in m/s, the twin tachometer and the
  twin exhaust gas temperature. Right wing: fuel in kg, hydraulics and the
  clock. Left wing: gear and flap lights and the autopilot and navigation
  panel (SAU, RSBN, ARK).
- Black coaming with the ILS-31 HUD (a big body and a heavy frame round the
  combiner glass) and the IT-23 display with a B-scope radar picture.
- A column of ten warning lights by the right rail; the legends are
  transliterated because the game font only has Latin characters.
- Centre stick on the floor, with a boot, and a two-lever throttle in a
  quadrant on the left console; gear handle on the left wall, ahead of the
  throttle.
- Canopy frames on the exterior shape (`mig29::canopy_point`): windscreen
  posts, the arch over the knees with its three mirrors and the rails; a
  bulkhead behind the seat and the decking under the rear of the canopy.

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
