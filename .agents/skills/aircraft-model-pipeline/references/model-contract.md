# Viper Run model contract

## Geometry ownership and integration

- Body axes: +X right, +Y up, +Z aft. Units: metres.
- Exterior datum belongs to the aircraft builder. `Type::eye()` transforms
  that datum to common cockpit design space `airframe::EYE`.
- `src/airframe.rs`: F-16, ACES II, right sidestick.
- `src/mig29.rs`: MiG-29S 9.13S, K-36DM, centre stick.
- `src/hornet.rs`: legacy F/A-18C, SJU-17 NACES, centre stick.
- `src/su27.rs`: Su-27SK, K-36DM, centre stick.
- `src/articulation.rs`: attributed mesh partition, Hinge, Drive, Surface.
- `src/cockpit/modern.rs`: Hornet/Flanker boards, HUD, mirrors, control
  hardware. Imported source and replacement hardware are checked together.
- `src/cockpit/pilot.rs` and `src/eject.rs`: shared seats, handles,
  hand targets, control sweep and ejection sequence.
- Adding Type requires CLI/menu/settings, network bounds and compatibility,
  physics/contacts, weapons/stations, damage, exterior/cockpit renderer,
  ejection arrays, capture tools and regression coverage. Search exhaustive
  matches and literal four-aircraft arrays. Never silently map a new wire ID
  to another aircraft or permit it in an incompatible protocol.
- `hornet::Airframe` is the compact twin-engine builder result: fixed body,
  canopy, paired stabilators/flaps/rudders, optional empty brake, three gear
  meshes and extra articulated `controls`. An additional procedural builder
  can implement this contract without embedding a donor aircraft.

## Reference register

Legacy/Soviet sources and their licences are documented in
`assets/models/README.md`, `docs/HORNET-FLANKER-MODELS.md` and
`docs/COMPACT-MODEL-REFERENCES.md`.

OpenHornet reference cockpit:
https://github.com/jrsteensen/OpenHornet
revision `77c7b2c07240d9d6062c16ed6e533bf2ed8bfce6`,
`reference/OpenHornet Reference Cockpit.step`,
SHA-256 `37915276a25973030f98d8c851e72cface9f3a7b589697a106940a515da1c840`.
Reference use only, not embedded in the game.
The STEP reader uses millimetres. The converted valid-face bounds are
X -523.016..523.016, Y 5041.900..7202.865, Z 2581.910..3844.544 mm.
40 of 37,577 faces lack triangulation. Do not derive geometry from those faces.
DDI face/case bounds and board slope can be measured independently of missing faces.
The current legacy C uses a physical UFC keypad. E/G cockpit arrangements must
be identified from their own references rather than inheriting C-specific labels.

Super Hornet and Growler primary dimensions:
https://www.boeing.com/defense/fighters-and-bombers/fa-18-super-hornet-and-ea-18-growler
https://www.airforce.gov.au/aircraft/ea-18g-growler
https://www.navair.navy.mil/product/EA-18G-Growler
Length 18.3 m, span 13.7 m, height 4.9 m.
F/A-18E is single seat; EA-18G is tandem, based on F/A-18F.
These are distinct airframes from the legacy C, with rectangular intakes,
larger LEX and wings. Growler has wingtip receiving pods and EW stores.
Treat local planform/control/seat fits as estimates unless directly measured.
Consult primary references for the seat configuration, station layout and
flight-control braking; do not invent a dorsal speedbrake from legacy C.

## Existing review commands

```sh
cargo test
python -m unittest discover -s tools -p test_flightgear_import.py -v
cargo run -- --export-cockpit target/model-review/current
blender --background -t 4 --python tools/review_cockpit.py -- target/model-review/current --jet f18
python tools/check_airframe_views.py PATH_TO_BINARY --output target/model-review/native
python tools/check_pilot_animation.py PATH_TO_BINARY --output target/model-review/pilot
python tools/check_mirror_views.py PATH_TO_BINARY --output target/model-review/mirrors
```

Use an isolated development Python environment for the optional STEP converter:
`python tools/convert_cad_reference.py SOURCE.step OUTPUT.obj`, requiring
`cadquery-ocp-novtk==7.9.3.1.1`. Blender imports OBJ in metres after a 0.001
scale; `tools/review_cad_reference.py` supplies comparison views.
Generated reviews and reference downloads stay in `target/model-review`.
Runtime must continue to work offline without Blender, Python or CAD libraries.
