# Pilot harness and parachute grip

The ejected pilot uses the same torso surface in first-person and external
views. Shoulder straps and the waist belt follow its cross-sections, including
the seated lean. Their thickness intersects the flight suit slightly so the
webbing reads as fitted fabric rather than a separate floating frame.

The airborne gloves close around the fabric risers, with four curved fingers
and an opposing thumb. The risers belong to the pilot mesh. Each suspension
cord is drawn between the pilot's riser tip and the canopy's skirt in world
space, keeping both ends connected as the canopy inflates and the pilot sways.

Touchdown starts a 0.8-second eased release. Sixteen cached poses lower the
hands beside the hips and fold the loose risers down. The cord roots use the
same cached pose index, preserving the connection throughout the transition.
The canopy retains its existing fall and deflation beside the pilot.

## Verification

`cargo test --release` passes 258 game tests and 6 server tests, including
seven ejection checks. The checks cover harness clearance on the actual suit
cross-section, identical harness geometry in both views, fingers enclosing
the riser, descending glove height through every landing pose, cord endpoints
during inflation and sway, and complete ejection/landing for all four jets.

Visual captures were rendered with the release executable at 1280 by 720:

```sh
viper-run --shot harness.png --jet f16 --mission airfield --input eject --sim 5 --look 0,-50 --no-performance
viper-run --shot grip.png --jet f16 --mission airfield --input eject --sim 5 --look 65,55 --no-performance
viper-run --shot inflating.png --jet f16 --mission airfield --input eject --sim 2.5 --look 65,55 --no-performance
viper-run --shot landed.png --jet f16 --mission airfield --input eject --sim 13 --look 65,-45 --no-performance
viper-run --shot landed-external.png --jet f16 --mission airfield --input eject --sim 13 --chase --zoom 0.4 --look 65,-20 --no-performance
```

The same harness and grip views were inspected for `--jet mig29`. The local
review captures are in the ignored `target/harness-shots/` directory. They
show webbing against the suit, closed fingers around the riser during both
inflation and full descent, and lowered arms with loose risers after landing.
