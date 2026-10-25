# F/A-18C and Su-27SK implementation

The new selectable types are `Type::F18` (F/A-18C Lot 20) and
`Type::Su27` (base Su-27SK, the export counterpart of the Su-27S).
They use the same rigid-body integrator, damage pipeline and input path as
the existing aircraft. They have separate longitudinal polars and engine ratings,
rather than flying a scaled F-16 or MiG-29 polar.

## Reference data and calibration

| Property | F/A-18C | Su-27SK |
| --- | --- | --- |
| Reference wing area | 37.16 square metres | 62 square metres |
| Aerodynamic reference span | 11.406 m | 14.7 m |
| Empty mass used by the game | 10,900 kg | 16,300 kg |
| Internal fuel | 4,930 kg | 9,400 kg |
| Engines | Two F404-GE-402 | Two AL-31F |
| Static augmented thrust, combined | 35,400 lbf | 25,000 kgf |
| Nominal positive load ceiling | 7.5 g, reduced at high gross mass | 9 g, 8 g above 26,000 kg in this reconstruction |
| Game AoA protection onset/cap | 27/30 degrees | 24/26 degrees |

NAVAIR identifies the C as the single-seat legacy Hornet with twin F404-GE-402
engines and the compatible missile families. GE gives the -402 augmented rating.
NASA's F-18 simulation and flight-identification reports provide the reference
geometry and the structure of the aerodynamic/control problem. The Su-27
reference manual supplies the fuel capacity and clean performance targets.

The polars are engineering reconstructions, with independent lift curves,
drag polars, static pitch margins, stabilator effectiveness and inlet recovery.
Their level-flight thrust/drag intersections are checked at sea level and
11,000 m against the published performance envelopes. Installed engine lapse,
lateral derivative tables, actuator structure and the basic FLCS architecture
remain shared game approximations. The aerodynamic tables are not a complete
transcription of the NASA F-18 database or a Sukhoi wind-tunnel dataset.
Fuel reserves, detailed inertia scaling and some control thresholds are tuning
choices, not claimed operational flight-manual procedures.

## Stores and sensors

The Hornet represents a Lot 20 configuration with AIM-9M, AIM-7M, AIM-120C
and single Mk-82 carriage. Wingtip rails take Sidewinders; wing stations take
the supported missiles or bombs; fuselage cheeks take Sparrow/AMRAAM and
never bombs. Its nose-mounted M61A1 has 578 rounds. The game's eight-missile
default omits dual AMRAAM launchers, tanks and centreline stores.

The base Su-27SK carries R-73, R-27R/T and supported FAB bombs, with 150 rounds
for its starboard GSh-301. It has no R-77 capability in this configuration.
Wingtip, wing, nacelle and tandem fuselage positions provide ten missile
stations. The two centreline positions follow the nacelle-pair store selection,
a deliberate simplification of the existing four-pair loadout interface.
The IRST, helmet sight and radar-guidance rules follow the existing Russian
sensor family.

Stores are released from the same body positions used to draw them. Weight,
wing load, drag and ammunition mass change as stores are expended. Both types
can be chosen as player or opposition aircraft, including mixed formations.

## Geometry, cockpit and ground handling

Each model has its own fuselage stations, wing planform, root extensions,
canopy, nacelles, stabilators, twin fins and undercarriage. The Hornet has its
canted fins and twin nose wheels; the Flanker has its wide engine spacing,
ventral fins, IRST and long tail sting. Moving flaps and rudders have separate
footprints from the fixed surfaces. Gear, stabilators, airbrake, flap damage,
broken legs and detached surfaces are rendered from the flight state.

Wheel contact points, spring/damping values, cycle times and speed limits are
type dependent. The Hornet's game gear/flap warning limit is 250 KCAS; the
Flanker's is 500/600 km/h. The gear handle selects takeoff/half flap (30 degrees on the Hornet,
18 degrees on the Flanker); the existing flap switch selects full landing flap
(45/25 degrees). Flap drag is type dependent. This is a simplified schedule,
rather than a reproduction of the original flight-control software.

The Hornet has a DDI/UFC/AMPCD layout; the Flanker has a separate analogue panel
and radar display. Instruments, mirrors, flight cards, units and ejection views
follow the chosen type. The Hornet uses an SJU-17 NACES reconstruction and the
Flanker a K-36DM. Seat timing, parachute drag and seat/pilot geometry remain
stylised game models.

The cockpit layouts reuse the game's radar, navigation and engine pages.
UFC labels and decorative switches are not fully interactive avionics.
Original aircraft flight-control software, carrier launch/arresting physics,
folding wings, aerial refuelling, thrust vectoring and full weapon certification
are outside this implementation.

## Verification

Tests cover finite geometry and dimensions, distinct instrument panels,
coefficient signs and pitch stability, trim and protected manoeuvres,
sea-level/high-altitude speed calibration, gear overspeed damage, takeoff,
landing and braking. Windows screenshots verify cockpit forward views and
exterior proportions. Final integration also runs the existing physics,
weapons, AI, terrain, missions and damage tests.

## Sources

- [NAVAIR, F/A-18 A-D Hornet](https://www.navair.navy.mil/product/FA-18-D-Hornet).
- [GE Aerospace, F404 engine specifications](https://www.geaerospace.com/military-defense/engines/f404).
- [NASA TM 107601, Simulation Model of a Twin-Tail, High Performance Airplane](https://ntrs.nasa.gov/api/citations/19920024293/downloads/19920024293.pdf).
- [NASA, Automated Simulation Updates using Flight Data](https://ntrs.nasa.gov/api/citations/20070031030/downloads/20070031030.pdf).
- [US Army ODIN, Su-27SK](https://odin.t2com.army.mil/WEG/Asset/69a009aadf0c0a290f7eb7dbc63ee6e4).
- [Eagle Dynamics, Su-27 reference manual](https://www.digitalcombatsimulator.com/upload/iblock/ed7/Su-27%20DCS%20Flaming%20Cliffs%20Flight%20Manual%20EN.pdf).
- [NAVAIR, Navy Aircrew Common Ejection Seat](https://www.navair.navy.mil/product/Navy-Aircrew-Common-Ejection-Seat-NACES).
