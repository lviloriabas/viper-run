# Cockpit, ejection and warning refinements

## References and limits

The user's F-16 and MiG-29 cockpit photographs guided proportions and equipment
placement. Coordinates remain visual estimates in the existing low-poly models;
this pass changes mounts, small clearances, mirrors and the pilot's reach, without
rebuilding either airframe.

- [F-16 flight manual, T.O. GR1F-16CJ-1](https://info.publicintelligence.net/HAF-F16.pdf),
  ejection system and page 3-41: two-handed central grip, upward pull, retain grip
  until pilot/seat separation, head at headrest, feet at pedals. The firing handle
  stays with the seat. VMS and aural warnings distinguish spoken messages from
  low-speed and landing configuration tones.
- [MiG-29 flight manual, GAF T.O. 1F-MIG29-1](https://www.airwar.ru/other/manuals/%5Baviation%5D%20-%20%5Bmanuals%5D%20-%20MiG-29%20Flight%20Manual.pdf),
  pages 1-86 through 1-89, figure 1-50: linked firing grips; harness and lap-belt
  retraction, arm protection and leg lift; canopy clearance before the ejection
  gun; rocket initiation after 104.5 to 107.5 cm of rail travel. Its figure shows
  a 0.5 s clearance delay; the backup initiation branch has a separate delay.
- [Eagle Dynamics MiG-29 flight manual](https://www.digitalcombatsimulator.com/upload/iblock/463/DCS%20MIG-29%20Flight%20Manual%20EN.pdf),
  pages 40-41: radio altitude below 1000 m resolves to 1 m, barometric altitude
  above that to 10 m. Pages 104-105 describe voice warning categories, including
  angle of attack, G, critical speed, altitude, engine fire and landing gear.
- [Eagle Dynamics MiG-29A manual](https://www.digitalcombatsimulator.com/upload/iblock/aa2/ulzl67k42kj99ushy9m29hlvmtk0y2tg/DCS%20MiG-29A%20Early%20Access%20Manual%20EN.pdf),
  cockpit section: IT-23 flight/sight picture and VIWAS priority ordering.

No source above establishes a universal five-unit step for both HUD speed and
height. This implementation removes the former forced 5 km/h rounding; integer
km/h is an explicitly provisional display assumption. Height uses the documented
1 m/10 m source-dependent rule. This is display resolution, not sensor accuracy.
The IT-23 repeats the simplified flight/sight picture; the game still does not
model every real navigation or radar mode.

## Changes

F-16 outer coaming depth now tapers smoothly. The accelerometer and four engine
indicators clear their shelves and outer edge. Engine mounts move inward by
2 cm; gauge faces sit 3 mm behind the front of their bezels instead of 24 mm.
The fuel-flow strip has its own space above the right MFD. MiG raised controls
mount on the front of the coaming, with backing, and the countermeasure readout
sits below the HUD housing. The SPO-15 has a rectangular face and the IT-23
shows a flight repeater. MiG mirrors retain their outline at 70% of the previous
size and sit close to the canopy bow, above the HUD sight line.

Both handle grips rise with the hands, with elbows forward and tucked in clear
of the vest. The head stays at the cockpit eye, avoiding the former camera jump
when the external pilot replaces the cockpit pilot. The K-36DM has deployed
arm guards, raised legs and a short catapult-to-rocket interval. Handles remain
with the empty seat after separation. Suspension cords are 3.6 mm across and
join separate 32 mm fabric risers, shoulder rings and the visible harness.
These dimensions and animations are visual approximations, not surveyed seat
geometry or a certified escape-system simulation. The existing three-mode ACES
and speed/height-dependent K-36 parachute model remains simplified.

Engines use continuous filtered noise with gently drifting turbine partials;
MiG engine sources are independent. Pitch follows engine RPM rather than aircraft
speed. VMS uses original embedded speech synthesis, not authenticated recordings.
Warning arbitration suppresses repeated per-frame calls and separates F-16 tones
from MiG speech. Altitude calls are suppressed during a normal gear-down approach.

Gear air loads continue after a jam and can break individual legs. Jam and break
are distinct states and warnings. Selecting gear up cannot reset either failure;
free-flight ground servicing repairs the broken flags along with other damage.
The existing 300 KCAS game limit is retained for both airframes; this is not a
claim that both real aircraft share identical operating limitations.

## Verification

Unit tests cast sight lines against the assembled cockpit triangles for the
outer F-16 gauges and raised MiG controls. They also check the warning column,
primary instruments through rudder travel, atlas separation, finite animation
meshes, grip contact, first-person head clearance and camera/handle continuity.
Other tests exercise progressive gear failure, warning cooldowns, voice decoding,
nonrepeating unclipped engine output, menu direction selection and stick repeat.

Visual verification uses the Windows OpenGL executable: front and side cockpit
views, downward grip poses, canopy clearance, the seat and parachute in both
views, and menus at small and normal resolutions with keyboard and controller
prompts. `--audio-demo file.wav --jet f16|mig29` writes the runtime mixer for
idle, spool-up, dry thrust, afterburner and a voice cue. Rendering a demo does
not claim an acoustic match to recordings of the real engines.
