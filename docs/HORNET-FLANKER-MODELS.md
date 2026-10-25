# Hornet and Flanker model reconstruction

Revised 25 October 2026. The models retain the game's procedural, faceted art
style. As with the F-16 and MiG-29, the process uses aircraft-specific stations,
cross sections, planforms and visual references, followed by cockpit clearance
tests and screenshots. Dimensions inside the cockpit are visual estimates,
not surveyed production drawings.

## User reference revision

The follow-up photographs supplied on 25 October 2026 are the primary visual
reference for this revision. The first Hornet photograph shows a tall central
UFCD display, two side DDIs and a lower MPCD. The second shows another cockpit
configuration and supplies the canopy-arch, mirror and console proportions.
The implemented legacy Hornet uses the first photograph's UFCD arrangement.
Its radio/menu labels are illustrative; this change does not implement a
complete UFCD avionics system.

For the baseline Su-27, the annotated analogue panel and the in-flight cockpit
photograph determine the dial groups, raised projector control housing,
right-hand hooded display, fuel tape and warning receiver. The photograph
with three large modern displays depicts a different instrument arrangement;
the model consistently uses the analogue reference. Exterior side views guide
the pointed radome, canopy and tail, and the Su-27 has no canards. Integrated
leading-edge strakes blend into the main wing without the old broad lobes.

## Earlier technical references

- F/A-18A/B/C/D NATOPS, A1-F18AC-NFM-000, February 2000 with changes:
  [public reference copy](https://server.3rd-wing.net/public/Bureau_4thMEG/VMFA-232/Documentation/Support/A1-F18AC-NFM-000_-_NATOPS_Flight_Manual_-_February_2000.pdf).
  Figure FO-5, FO-19, is the single-seat C/D front panel for aircraft 163985
  and later. It locates the two DDIs, central UFC and HUD controls, low MPCD,
  IFEI on the left and standby instruments on the right.
- [NASA, A New Twist in Flight Research](https://www.nasa.gov/wp-content/uploads/2015/03/A_New_Twist_in_Flight_Research1.pdf),
  appendix 2 three-view. The drawing supplies legacy Hornet proportions,
  not the larger Super Hornet geometry. The long HARV research nose boom
  is not part of the production Hornet model.
- [NAVAIR, F/A-18 A-D Hornet](https://www.navair.navy.mil/product/FA-18-D-Hornet).
- [Eagle Dynamics, Su-27 reference manual](https://www.digitalcombatsimulator.com/upload/iblock/ed7/Su-27%20DCS%20Flaming%20Cliffs%20Flight%20Manual%20EN.pdf).
  Figure 16, page 20, shows the blended lifting body and separated nacelles;
  Figure 20, page 23, shows a real single-seat Su-27P cockpit; page 24 lists
  the 21.9 m length and 14.7 m span. The game represents the related Su-27SK;
  the photograph guides the analogue layout and proportions.

References are used to reconstruct geometry. Their photographs and drawings
are not redistributed or used as in-game textures.

## Exterior

Each aircraft has its own smoothly interpolated longitudinal stations. The
Hornet has a compact, squared twin-engine aft fuselage, narrow pointed
radome, curved LEX, moderately swept wings, strongly canted swept fins,
wing fold seams, twin nose wheels, launch bar and stowed arresting hook.
The Flanker has a longer drooped radome and pitot, central IRST, broad
shallow lifting body, wider spaced square-to-round AL-31 nacelles,
swept twin fins, ventral fins and central tail sting.

The wings and stabilators use cosine-spaced NACA thickness sections rather
than extruded flat plates. Intake lips open onto recessed ducts; the engine
bays, exhaust tunnels and individual nozzle petals are separate features.
Flaps, rudders, stabilators, speedbrake and gear remain articulated.
The Flanker's dorsal speedbrake has its own position and hinge height.
Tyre radii locate the axles above the same contact points as the ground
simulation. The canopy is described by width, sill and crown stations;
the first-person bow and rails use precisely that same surface.

## Cockpit

The design eye is normalised to the existing F-16 cockpit coordinate space.
The exterior eye remains aircraft-specific. This preserves the established
pilot, centre stick, two-lever throttle and pedal positions.

The Hornet now uses the supplied photograph's central UFCD and four-display
arrangement. HUD controls sit beneath the UFCD; the low MPCD, left IFEI,
right standby group and shoulder warnings have separate mounts. Yellow
striped jettison handles flank the main panel. The Su-27 has an attitude director above the HSI, airspeed and
altitude to the left, engine and vertical-speed instruments to the right,
and an upper-right radar display with the SPO-15 below it.

Both boards have tilted, orthonormal instrument frames, folded outer
cheeks, continuous shaped coamings and separate knee wells. The console
inboard edges stay outside the seat bucket, the floor is below the boots,
and the left console leaves the full throttle and hand travel clear.
Native canopy bows and three rear-view mirrors replace the generic arch.
The bows sit farther forward, and the side mirrors follow the arch tangent.
The Su-27 tub and bow use the reference blue-green finish. The main boards
are denser and farther forward, with closed joins to the cockpit sidewalls.

Each model has its own physical HUD aperture, supports and projector; its
clipping polygon uses that same aperture. Fixed HUD readouts fit the smaller
fields while conformal flight-path, horizon and weapon directions retain their
original angular projection. The radar/navigation pages and pilot body still
share existing game systems. Engine readouts use the flight
model's common engine state for both engines. Decorative controls and UFC
options are not a complete interactive avionics simulation.

## Torn-off landing gear

A broken leg disappears from the attached assembly and becomes a separate
wheel-and-strut mesh. It starts at its current articulation, including a
partially extended or bent pose. Velocity includes aircraft translation,
angular motion and a separation impulse; the debris then spins, experiences
drag and falls under gravity. Gear debris produces a metal impact rather
than a burning-fuel trail. A leg is emitted once and does not return when
the gear handle is moved. Individual missing legs are replicated in network
actor flags; the local emission marker resets on a repaired/respawned actor.

The --break-gear screenshot/test option detaches all three legs at the start.
Combine with --sim 0.15 --chase to show separation.

## Verification

Regression tests cover finite meshes, external dimensions, canopy eye and
helmet clearance, tyre contact height, readable instrument sightlines,
non-overlapping instrument faces, orthonormal mounting frames and pilot
clearance. Physical tests cover all four aircraft, exact release poses,
one-shot emission, ballistic motion and per-leg network replication.

Screenshots are reviewed from the forward seat, down toward the panels and
consoles, and from frontal, overhead and rear exterior angles. Temporary
reference documents and captures stay under ignored target/model-review.


The completed review used 22 Linux captures from the reproducible
tools/check_airframe_views.py matrix and Windows captures of both forward
cockpits. It caught and corrected blocked displays, an incorrectly placed HUD
projector, console joining shelves in front of low gauges, open coaming gaps
and inverted normals on sloping wing panels. Regression tests preserve those
geometry fixes. Final release verification also includes the independent
menu-layout and pilot-harness changes coordinated in the same checkout.

## Additional regression coverage

The fixed wing and moving flap now sample the same airfoil with continuous
spanwise thickness. Their shared hinge vertices coincide when stowed, and the
flap end caps never project ahead of the hinge. Tests cover both conditions,
end-cap normal orientation, the absence of outboard Su-27 foreplanes, and the
fit of fixed readouts inside both model-specific HUD apertures. Existing
instrument ray tests still require every sampled face to be visible.

The follow-up matrix adds straight-ahead canopy, side profile and head-on views
for both aircraft (28 captures total). Console panels now have separate legend
sets, two rows of switch hardware, rotary controls and guarded switches. Narrow
outboard strips beside the throttle retain the full lever and hand travel.
Console labels identify control groups; decorative switches do not add a new
interactive systems simulation.
