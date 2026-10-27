# Pilot controls, ejection handles and parachute grip

Revised 27 October 2026.

The ACES II in the F-16 has one flattened D-shaped yellow/black loop. The
Hornet SJU-17 NACES has its own oval yellow/black loop. The MiG-29 and Su-27
K-36DM use paired red loops, flaring out at the crown and converging at the
base, following the supplied cockpit photographs. Brackets join the seat
pan, and the fixed mounting block stays in place while the pull rods extend.
All are between the knees at the front of the seat, with separate type
dimensions and vertical placement.

The Hornet seat identity and seat-bucket initiation location are checked
against [Martin-Baker's NACES data sheet](https://martin-baker.com/wp-content/uploads/2023/11/NACES-Seat-Data-Sheet-2023.pdf)
and [NAVAIR's NACES description](https://www.navair.navy.mil/product/Navy-Aircrew-Common-Ejection-Seat-NACES).
These are visual game adaptations; handle travel and ejection timings remain
simulation approximations.

Cockpit and exterior use the same handle mesh, harness reference and grip
points on the actual tube. Both gloves reach the handle before its pull
begins. K-36 stabilisers and limb guards remain stowed until seat firing.
Their final closed-finger mesh is shared across the cockpit/seat
transition. Initiation retains the completed pull instead of restarting it.
The first-person eye remains continuous at seat firing.

The normal pilot follows the stick, throttle pair and pedals. The Hornet
and Flanker use their adjusted leg pose and pedal travel; the Flanker throttle
and left hand move together on the outboard quadrant.

After separation, 24 cached poses carry the hips, knees, boots, elbows and
gloves from the seated position to suspension. Body attitude and the downward
camera bias ease in during inflation. The helmet remains outside first-person
views. Finger geometry is built in a stable local frame before rotation.

The ejected pilot uses the same torso surface in first-person and external
views. Shoulder straps and the waist belt follow its cross-sections,
including the seated lean, with slightly embedded thickness. Closed gloves
enclose the fabric risers. Each suspension cord runs from the moving riser
tip to the canopy skirt in world space during inflation and sway.

Touchdown starts a 0.8-second eased release. Sixteen cached poses lower both
hands beside the hips and fold the loose risers down. Cord roots use the same
cached pose index. The parachute then settles beside the pilot.

## Verification

The complete suite passes 280 game tests and 6 server tests in debug and
release profiles. Eleven ejection checks cover type-specific finishes, tube/grip
contact, camera and handle continuity, finite deployment poses, bounded glove
motion, identical phase endpoints, fitted harness geometry, enclosing fingers,
landing release, connected suspension cords and complete ejection/landing
for all four jets. A separate cockpit test checks the dynamic GPU buffer
through control extremes and the full reach/pull range.

The reproducible animation matrix produces 48 native game captures per
platform: left/right control extremes, reach, pull, canopy clearance in both views, seat
flight in both views, separation, parachute grip/body and touchdown, for all
four jets. Capture manifests include the binary SHA-256.

```sh
python tools/check_pilot_animation.py dist/viper-run-linux-x86_64 --output target/model-review/pilot-linux
python tools/check_pilot_animation.py dist/viper-run-windows-x86_64.exe --output target/model-review/pilot-windows
dist/viper-run-linux-x86_64 --export-cockpit target/model-review/pilot
blender --background -t 4 --python tools/review_ejection.py -- target/model-review/pilot
```

Run each executable on its native platform. Blender renders the exported game
geometry, including unobstructed handle close-ups and seated/deploying/hanging
pose comparisons, and saves an editable review scene. Images and scenes stay
in the ignored target directory.
