# Validation

## Unified model style, cockpit clearance and tactical units, 27 October 2026

The Hornet/Flanker exterior palettes use 48 clean colours; source cockpit
panels use 64, with reserved colours for insignia, warning lights and labels.
Images retain aspect, UV layout and resolution up to 2048 pixels. QOI packaging
is lossless after styling. Every original source hash still matches. Comparing
the embedded meshes with the previous revision confirms that their geometry
bytes are identical; painted simplification does not change aircraft dimensions.

The Su-27 seat and eye move 12 cm aft and its centre stick moves 18 cm aft.
Hands, legs, pedals, outside crew and ejection origin remain aligned. Fifteen
full pitch/roll combinations check stick edges against both procedural and
imported textured hardware. Reach tests retain the arm lengths. The fixed panel
and console geometry is not moved.

Hornet shoulder mirrors now follow the tangent of the mating bow. HUD details
include slotted rails, diagonal feet, fasteners and an optical rim. The
Hornet airbrake's outer-face winding is corrected and its UVs interpolate
the underlying spine paint. Both tactical contact labels and locked-target
HUD range stay in nautical miles, including close range.

FlightGear reference measurements guide the wider D-section inlet and compact
interrogator blades on the F-16, and the five sloped MiG auxiliary-intake leaves.
Original overall-dimension and moving-surface checks still pass. See
[COMPACT-MODEL-REFERENCES.md](COMPACT-MODEL-REFERENCES.md) for URLs, hashes,
measurements and variant limits. Full reference models are not embedded.

The final debug and release suites pass 303 game tests and 12 server tests.
All eight Python importer/style checks pass. Linux and native Windows each
complete 67 airframe/cockpit captures, 21 mirror captures and 12 Su-27 control
and ejection captures. Blender inspection uses eight views per aircraft,
including a close lateral cutaway of the stick and panel. Native captures
verify the live textures; Blender's workbench faces focus on geometry.

The Windows game is 34,840,576 bytes, down from 50,138,112 bytes (about 30.5%).
The Linux game is 35,000,728 bytes, down from 50,297,072 bytes. Final distribution
files match their target build outputs and the capture manifests' SHA-256.
The native Windows executable also passes the local Defender custom scan;
the ignored report records its exact hash and scanner output.

~~~sh
cargo test --quiet
cargo test --release --quiet
python -m unittest discover -s tools -p test_flightgear_import.py -v
./build.sh
python tools/check_airframe_views.py dist/viper-run-linux-x86_64 --output target/model-review/style-final-linux
python tools/check_mirror_views.py dist/viper-run-linux-x86_64 --output target/model-review/style-final-linux/mirrors
python tools/check_pilot_animation.py dist/viper-run-linux-x86_64 --output target/model-review/style-final-pilot-linux --jets su27
dist/viper-run-linux-x86_64 --export-cockpit target/model-review/style-final-blender
blender --background -t 4 --python tools/review_cockpit.py -- target/model-review/style-final-blender --jet f18
blender --background -t 4 --python tools/review_cockpit.py -- target/model-review/style-final-blender --jet su27
~~~

Run the corresponding Windows executable with native Windows Python for its
three capture matrices. Distribution hashes are in dist/SHA256SUMS.txt.

## Canopy joint, mirror seats and HUD lead, 27 October 2026

This revision supersedes the mounting description in the earlier mirror pass.
The Hornet forward bow now has tapered posts, defined shoulders and a shallow
crown, with a continuous mating lip. Its three mirror cases are seated on that
joint in the same arrangement as the MiG-29. The old triangular bow is replaced
rather than overlaid. The Flanker crown is lowered onto its imported mating bow,
with short hinges, and its side hinges attach to the joint's rear face.

Both ends of the Su-27 HUD lead now enter the solid projector through collars.
A rounded cover connects the optical head, control case and foundation. The
thin top lip clears the crown mirror without moving the original combiner glass.

Debug and release suites pass 297 game tests and 12 server tests, with no
failures or ignored tests. New checks cover the shallow bow crown, original
sill contact, absence of duplicate bow geometry, crown rim contact in the
pilot's projection, short hinges, embedded lead endpoints and uninterrupted
projector cover. Mirror visibility is checked across a denser pane grid
against the assembled hardware; optical direction and UV coverage still pass.

Final native Linux and Windows executables each complete 21 captures, including
forward, crown, left/right, panel, oblique HUD lead, bank and disabled mirrors.
MiG-29 and F-16 views cover the shared rendering path. The final geometry is
also inspected in Blender, with seven views per aircraft including close
views of the bow and HUD lead. The saved scenes use the exact exported game
triangles; their white display faces are workbench geometry previews, while
the native captures verify the live textures.

~~~sh
cargo test --quiet
cargo test --release --quiet
./build.sh
python3 tools/check_mirror_views.py dist/viper-run-linux-x86_64 --output target/model-review/bow-final-linux
python tools/check_mirror_views.py dist/viper-run-windows-x86_64.exe --output target/model-review/bow-final-windows
dist/viper-run-linux-x86_64 --export-cockpit target/model-review/bow-final-blender
blender --background -t 4 --python tools/review_cockpit.py -- target/model-review/bow-final-blender --jet f18
blender --background -t 4 --python tools/review_cockpit.py -- target/model-review/bow-final-blender --jet su27
~~~

Run each executable on its native platform. Capture manifests identify the
tested binary hash; distribution hashes are in dist/SHA256SUMS.txt.

## Mirror mounting and reflected view, 27 October 2026

The Hornet side panes now stand nearly upright on the actual imported forward
canopy bow; the old mounting station placed the panes and hinges ahead of it.
Hinges follow the painted bow instead of ending in empty space. Flanker
shoulder panes face the pilot on the aft side of the frame, and the crown
clears the HUD hood. Surface normals and reflected sight rays determine
sampling for each pane.

The reflection is rendered at 1536x480 and sampled directly in the final
cockpit pass, avoiding the previous 512x160 atlas copy. Reduced optical bulge
removes the exaggerated curved horizon. Disabled mirrors retain dark glass
instead of displaying unrelated instrument-atlas fragments. The MiG keeps
its established placement and receives the sharper texture.

Debug and release suites pass 293 game tests and 12 server tests. Added
checks cover bow placement, upright Hornet panes, visibility against actual
hardware, reflection direction, unclipped UVs, texture aspect, geometry/depth
preservation and inactive glass. Native Linux and Windows each completed
19 reviewed 1440x900 captures: frontal, crown, left/right, panel, bank and
disabled views, plus MiG-29 and F-16 regression views. Blender inspection of
the exported final geometry checks front, side and top placement.

~~~sh
cargo test --quiet
cargo test --release --quiet
./build.sh
python tools/check_mirror_views.py dist/viper-run-linux-x86_64 --output target/model-review/mirrors-final-linux
python tools/check_mirror_views.py dist/viper-run-windows-x86_64.exe --output target/model-review/mirrors-final-windows
dist/viper-run-linux-x86_64 --export-cockpit target/model-review/mirrors-final-blender
blender --background -t 4 --python tools/review_cockpit.py -- target/model-review/mirrors-final-blender --jet f18
blender --background -t 4 --python tools/review_cockpit.py -- target/model-review/mirrors-final-blender --jet su27
~~~

Run each executable on its native platform. Capture manifests record the
binary hash; distribution hashes are in dist/SHA256SUMS.txt. One common rear
camera still approximates parallax between mirror positions.

## Multiplayer server browser and TCP relay, 27 October 2026

The rework passes 288 game tests and 12 standalone server tests in release mode.
Coverage includes concurrent host/client startup, encrypted fragmented state,
voice forwarding, no UDP listener on TCP players, filtering, pagination beyond
four rooms, private-room visibility, started-room removal, disconnects, malformed
TCP frames, bounded queues, browser response timeout and saved preferences.

Real game processes completed all six modes on Linux through both direct UDP
and the TCP relay (12 cases). Native Windows processes completed all six modes
through TCP. Checks require replicated state, received controls, remote gun
ammunition consumption and microphone inactivity.

The graphical input check passed on Linux and Windows: a real public room is
discovered in SERVERS, selected with the controller, joined, marked ready,
launched and left through the live UI handlers. Existing private LAN hosting
and flight school checks also pass. The menu audit passes 96 multiplayer layouts
on each platform at 640x360, 1280x720 and 1920x1080 with keyboard/controller
prompts. Browser, create, invitation, LAN, empty/error states, eight-pilot lobby,
voice and all six create-mode briefings are included. Browser/create/lobby
captures were visually inspected. The Windows capture/overlay check passes.

Windows loopback tests must run with local networking available. The restricted
automation sandbox denies these connections; running the same checks with
normal network access passes. No Windows security settings were changed.

The Windows game and relay received local Microsoft Defender custom scans with
no threats found. Real-time protection and antivirus were enabled, with
signatures 1.461.224.0 (updated 26 October 2026). The scan script records the
exact final hashes and output in target/multiplayer-review/defender-scan.json.
This is a local result, not a guarantee for other PCs. The earlier reported
detection occurred on another PC and its exact threat name is unavailable.

Reproduce with:

~~~sh
cargo test --release --quiet
./build.sh
python3 tools/check_multiplayer.py --transport both
python3 tools/check_menu_layout.py --exe dist/viper-run-linux-x86_64 --out target/multiplayer-review/layout-linux --full --filter network
dist/viper-run-linux-x86_64 --future-ui-check --screen menu --shot target/multiplayer-review/future-ui.png --frames 400
~~~

On Windows run the same Python commands with the Windows executable; use
--transport tcp for the Windows multiplayer matrix, and run
python tools/check_windows_defender.py.
Distribution hashes are in dist/SHA256SUMS.txt. Separate-network Internet
acceptance still requires an actually deployed relay; loopback tests do not
establish Internet reachability. Physical microphone hardware is not exercised.

## Cockpit closure, compass and pilot animation, 27 October 2026

This revision supersedes the earlier Hornet/Flanker geometry checks below.
The final source passes 280 game tests and 6 standalone server tests in both
debug and release profiles. All 6 Python importer tests pass, including upstream
source hashes, UV-preserving cuts and pixel-for-pixel cockpit texture preservation.
The bundled AC3D/XML sources were unpacked and all six generated model assets
and manifests matched the delivered files byte for byte after offline regeneration.

The native Linux and Windows release binaries each completed 55 airframe/cockpit
views and 48 pilot/control/ejection views at 1440x900. All 206 captures were
visually reviewed. Views include panel/console sightlines, leg wells, the three
mirrors, Hornet and Russian compass installations, rear cockpit closure,
exterior profiles, mounted weapons, moving surfaces, detached landing gear,
control extremes, handle reach/pull, canopy clearance, seat flight, separation,
parachute grip and touchdown. The Windows external capture/overlay check passed.

Linux Blender rendered 15 cockpit views with editable scenes and eight seat/
handle/pose comparisons from geometry exported by the final game binary.
Earlier exterior orthographic review scenes remain available. Sources and
simulation limits are documented in [HORNET-FLANKER-MODELS.md](HORNET-FLANKER-MODELS.md)
and [PILOT-HARNESS.md](PILOT-HARNESS.md).

Reproduce the checks from the repository root:

```sh
cargo test --quiet
cargo test --release --quiet
python -m unittest discover -s tools -p test_flightgear_import.py -v
python tools/check_airframe_views.py dist/viper-run-linux-x86_64 --output target/model-review/release-verified-linux/airframe
python tools/check_airframe_views.py dist/viper-run-windows-x86_64.exe --output target/model-review/release-verified-windows/airframe
python tools/check_pilot_animation.py dist/viper-run-linux-x86_64 --output target/model-review/release-verified-linux/pilot
python tools/check_pilot_animation.py dist/viper-run-windows-x86_64.exe --output target/model-review/release-verified-windows/pilot
python tools/check_windows_capture.py dist/viper-run-windows-x86_64.exe
dist/viper-run-linux-x86_64 --export-cockpit target/model-review/release-final-blender
blender --background -t 4 --python tools/review_cockpit.py -- target/model-review/release-final-blender
blender --background -t 4 --python tools/review_ejection.py -- target/model-review/release-final-blender
```

Run each executable on its native platform. Capture directories contain
`captures.json` and `binary.sha256`, tying the matrix to its binary.
Final release SHA-256:

| Executable | SHA-256 |
| --- | --- |
| Linux game | `2c4b8d4236d40280af7f2bc8998b585dc3e5664b921874fdc26c5d27dcea7490` |
| Windows game | `01a06d45c64665977b3019965c87a61dbff77007a4b702959714b7d3a5567e37` |
| Linux server | `8c9a53339387b381012746bad45e40f4b687a726ddac055935a08d8f592c95af` |
| Windows server | `3834289a9ec0caac9d67c03be60812b5b0eab0ca24b9fcab2ac544fae8b055e6` |

## Earlier validation history

The delivered source and release executables were checked with:

- cargo test --release: 258 game tests and 6 standalone server tests, no failures.
- Original menu integration checks for F-16, MiG-29, F/A-18 and Su-27 on native Windows.
- Flight school and online menu integration on native Windows, including click,
  Enter, controller A, remapped PTT, ready/launch gating, local configuration,
  online resume/leave and protection against a local restart during a session.
- Rendered school, room and configuration screens at 640x360 and 1280x720,
  plus the five tutorial starts.
- Actual game processes for all six multiplayer modes on Linux. Each case
  received authoritative state, control updates and remote ammunition changes.
  The team-combat case also ran the real directory/relay process.
- Actual native Windows game processes for direct dogfight and relay team combat.
- A full eight-pilot transport test with fragmented world state, encrypted
  handshake, replay/reordering checks, incorrect-key rejection and mode-filtered
  public discovery.
- Remote gun and bomb ownership, shared ammunition accounting, damage between
  remote pilots, replicated airframe/heading/inventory and swept aircraft collision.
- Independent flight, speed-envelope, gear, takeoff and landing checks for the
  Hornet and Flanker. Their modeled approach route was checked against terrain.
- 27 synthetic voice/audio and mission-objective checks on Linux and native
  Windows, plus 3 Python warning-audio checks.
- External Windows capture/overlay compatibility and byte hashes matching the
  executable used for the final visual checks.

The physical microphone input and Internet paths across independent networks
were not exercised. Voice checks used synthetic audio and kept microphones
disabled during game-process tests. The Internet relay code was exercised
locally; an Internet-reachable deployment and hardware acceptance check remain
necessary before relying on a particular network or microphone setup.

Reproduce the automated checks using README.md and MULTIPLAYER.md. Aircraft
data sources and modeling limits are recorded in NEW-AIRFRAMES.md.

## Menu layout regression checks

Menu action rows use measured widths shared by keyboard and controller prompts.
Rows wrap before adjacent panels. Briefing values, result tips and bottom notes
wrap using the same glyph and controller-token measurements as their renderer;
result panels reserve the measured height of their content.

The `--layout-check --shot FILE.png` diagnostic records text, buttons, fields and
panel boundaries from the actual drawing calls. It rejects overlapping controls
or text, content crossing a control/panel edge, panels painting over controls,
and content outside the window. Failed cases still save their screenshot.
This diagnostic is disabled in normal play. Snapshot fixtures do not save settings
or poll network/voice input, so full lobbies and voice options are checked without
starting microphone capture.

Run the Windows matrix against a local copy of the executable:

```sh
python tools/check_menu_layout.py --exe PATH_TO_LOCAL_EXE --out OUTPUT_DIRECTORY --full
```

The default sizes are 640x360, 1280x720 and 1920x1080. Use `--sizes` for comma-separated
additional sizes, `--prompts keyboard` or `--prompts controller` to isolate input
prompts, and `--filter` to select a case by name. Each run writes PNG captures,
text logs and `layout-results.json`. The full matrix includes all four aircraft,
all five single-player modes, maximum threat setups, all mission outcomes,
school/result screens, every training instruction, both controller layouts,
rebinding, long fields, four public listings and eight-pilot lobbies.
The existing `--ui-check` and `--future-ui-check` also accept `--layout-check`
to validate the rendered menus during their click, Enter and controller-A flows.

## Model and gear integration, 25 October 2026

The integrated release passes 258 game tests and 6 server tests. The Hornet and
Flanker review includes 22 reproducible model/cockpit captures, Windows forward
views, exact gear release poses for all four aircraft and replicated missing-leg
flags. See [model references and limits](HORNET-FLANKER-MODELS.md) and
[pilot harness checks](PILOT-HARNESS.md). The menu review checked 960 Windows
captures and eight navigation runs across the four aircraft.

## Supplied Hornet and Flanker references (25 October 2026)

The cockpit correction uses the supplied Hornet UFCD photograph and the baseline
analogue Su-27 photographs, with a canardless Flanker exterior. Final release
validation passed 261 game tests and 6 server tests. The expanded airframe matrix
produced 28 reviewed Linux captures; both Windows cockpit captures also completed
successfully with the final distribution executable. Checks include panel and
console sightlines, leg/throttle clearance, distinct HUD apertures, fixed HUD
readout fit, continuous wing/flap skin and the Su-27 forebody envelope.

## Completed OpenCode model revision (26 October 2026)

The `test` branch passes 271 game tests and 6 standalone server tests with
`cargo test --release`. Additional regressions cover the two-pane Hornet mirror
installation, clear canopy bubbles and both sill rails, the Hornet fixed-fin
boundary, five-point Flanker insignia and the stowed Flanker speedbrake's fit
against the curved spine. Existing wing/flap, gear, instrument sightline and
pilot-clearance tests also pass.

`./build.sh` rebuilt both Linux and Windows distributions. The 30-case
`tools/check_airframe_views.py` matrix completed on Linux and native Windows,
using the final distribution executables (the Linux build output was checked
byte-for-byte against `dist/`). All 60 images were reviewed for cockpit and
exterior geometry, gear, detached gear, rudders and deployed speedbrakes. Four
additional lateral speedbrake captures completed, two per platform. The expanded
overhead camera framing includes the complete aircraft.

Reproduce the matrix with:

```sh
python3 tools/check_airframe_views.py dist/viper-run-linux-x86_64 --output target/model-review/opencode-final
python tools/check_airframe_views.py dist/viper-run-windows-x86_64.exe --output target/model-review/opencode-final-windows
```

Run the second command on native Windows. Reports and captures stay under the
ignored `target/model-review/` directory. FlightGear reference models, their
hashes and variant differences are documented in HORNET-FLANKER-MODELS.md.

## Detailed exterior and live cockpit pass (26 October 2026)

Final validation passes 276 game tests and 6 standalone server tests, with no
failures or ignored tests. The added checks cover independent outboard wing-chord
reference samples, native image aspect in every instrument aperture, active atlas
allocation boundaries, separate moving gear/flap/brake/weapon/sensor controls and
aircraft-specific sensor annunciations. All previous sightline, HUD, canopy,
structural, flight, gear, networking and weapon tests still pass.

`./build.sh` rebuilt Linux and Windows. The expanded airframe matrix ran against
the final Linux build and native Windows distribution, producing 34 captures per
platform. All 68 captures were visually reviewed. In addition to the existing
exterior/cockpit/gear views, the matrix now includes gear, flaps and speedbrake
commands together, Hornet weapon selection and Flanker IRST selection. Snapshot
commands also update their mechanical cockpit levers, so the inside and outside
views represent the same command state.

The tested Linux build is byte-identical to the Linux distribution file, and both
distribution files match their final target build outputs. Distribution SHA-256:

- Linux game: `c451ac32934b6b1b20aaa1e1ac007df09363dcd0b79081da9a8c9ba075932752`.
- Windows game: `b8d62f86c3e4ad00e63a0f80f926b219c3a3705a45fef5393df66676f4cf09f7`.

Reproduce with:

```sh
cargo test --release
./build.sh
python3 tools/check_airframe_views.py dist/viper-run-linux-x86_64 --output target/model-review/detailed-final
python tools/check_airframe_views.py dist/viper-run-windows-x86_64.exe --output target/model-review/detailed-final-windows
```

Run the Windows command on native Windows. The ignored output directories contain
`captures.json`, full-resolution PNGs and reviewed contact sheets. The Linux
folder also contains `overview.png` with both exteriors and forward cockpits.
The exterior reference measurements and cockpit modelling limits are recorded in
HORNET-FLANKER-MODELS.md. Controls remain on the existing keyboard and controller
bindings.
