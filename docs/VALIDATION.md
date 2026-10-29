# Validation

## Hornet canopy, E/G exteriors and damage separation, 29 October 2026

This revision replaces the shared C exterior for F/A-18E and EA-18G, reshapes
the C windshield bow, fits slender mirrors and attaches the compass to the
right arch. Store mounts follow the actual pylons; Growler tip receivers
reserve their stations. Nozzle interiors no longer sample exterior paint.
Structural cracks and weapon impact/burn masks use independent shader channels.
The integrated missile changes also reject destroyed, crashed and abandoned
targets and distinguish SARH support, independent IR and active terminal radar.
References, source permission, transforms and approximation limits are in
[HORNET-CANOPY-SUPER.md](HORNET-CANOPY-SUPER.md) and [WEAPONS.md](WEAPONS.md).

Final source was frozen before building and capturing both distributions:

- Rust: 351 game tests and 16 server tests passed, with zero failures or ignored
  tests. Both release builds completed without warnings.
- Python: all 16 importer, source reconstruction and warning-speaker tests passed.
  Pinned GPL sources reconstruct both variants offline, byte for byte, and VRZ
  decompression restores the editable meshes exactly.
- Native Linux and Windows each produced 48 acceptance captures: 13 views of
  each Hornet variant, plus nozzle and paired damage views of F-16, MiG-29 and
  Su-27. Reviewed views cover mirrors, compass, panels, gear, brake, underside
  mounts, full planform and separate damage causes.
- Native UI and future-UI integration checks passed on both platforms. All 120
  loadout layouts passed at 640x360: six aircraft, five missions and keyboard
  and controller prompts on both operating systems.
- The cheek-clearance regression checks all five western missile families
  against the original C intake skin, allowing 8 mm at contact. Mirror tests
  check the complete panes and optical mapping with small head displacement.
  E/G tests check each rail centre/orientation, fixed Growler tips, separate
  canopy lengths and tyre-to-ground contacts within 15 mm.
- Control travel tests use each variant's actual surfaces, including E/G,
  with neutral, intermediate, extreme and combined flap/roll/yaw states.
- Blender generated three editable assembled scenes and 21 top/front/side,
  quarter, underside, pilot-eye and bow renders from the final exported meshes.
  The scenes include separate source-reference objects and mounted inventory.
- Real Linux processes passed dogfight and combat over direct UDP, external TCP
  relay and embedded TCP hosting. Native Windows passed both embedded modes.
  Reports confirm remote inputs, snapshots, simulation ticks, gun inventory
  consumption and disabled microphone capture. These are loopback checks.

The first Windows future-UI attempt could not create its local network session
inside the restricted execution environment. The same executable and assertions
passed with loopback socket access. Python reconstruction tests also passed after
granting the temporary-directory access denied by the Windows sandbox. No check
was skipped or relaxed.

After correcting the test to inspect E/G's own controls, all tests passed again.
Rebuilding left Linux byte-identical; Windows differed only in two bytes of its
COFF link timestamp (offsets 128 and 129). The captured Windows build is retained
as the distribution, with identical runtime code and resources. This comparison
is recorded in `target/canopy-super-review/test-only-rebuild.json`.

Evidence is under `target/canopy-super-review/`; the native Windows copy and
captures are in this chat's `hornet-revision/` artifact directory. Capture and
layout manifests record their executable SHA-256. The four distribution hashes
are listed in `dist/SHA256SUMS.txt`.

Resource comparison includes the complete fixed and movable exterior and gear,
excluding separately mounted inventory meshes:

| Resource | Previous | Final |
| --- | ---: | ---: |
| Hornet C exterior triangles | 9,033 | 9,177 |
| Super Hornet E exterior triangles | 9,033 shared | 25,392 |
| Growler exterior triangles | 9,033 shared | 28,076 |
| E editable / packed model bytes | shared | 3,163,028 / 701,057 |
| G editable / packed model bytes | shared | 3,559,556 / 753,785 |
| E / G styled QOI bytes | shared | 221,028 / 194,511 |
| Linux game bytes | 12,991,624 | 14,890,048 |
| Windows game bytes | 12,798,976 | 14,697,984 |

Both new exteriors remain below 30,000 triangles. The two new palette-styled
images retain their source UVs and dimensions; the original sources and explicit
author permission are bundled in the repository. The single-seat roof and pilot
fit remain reconstructed approximations, and the rear cockpit/EW equipment is
not a complete systems simulation.

Final game SHA-256:

- Linux: `36f1fff38f30f48c0bfac29eafe509509b6fea15ec6b6dcadca84406869dd5cd`.
- Windows: `7c0fa51363ed0cc2afa8d46fad38ec1a33a9bbdf9db8fd2c150409ed28fa4f97`.

The older sections below describe historical snapshots and retain their hashes.

## Pending TODO completed, 29 October 2026

All 16 open items are implemented and marked complete in TODO.md. Behavior,
research references and model limitations are recorded in
[PENDING-TASKS.md](PENDING-TASKS.md). The final distributions include draggable
settings, separate flight/look sensitivity, empty tip stations, adjustable gun
ammunition and mass, adaptive Hornet protection, persistent regional damage,
manual escape after structural failure, stabilising drogues, two Growler seats,
pilot-relative audio, ballistic spent missiles, corrected Su-27 cuts and brake,
Hornet-family E/G materials and instruments, opaque cockpit wells and Windows
window/class icons. E/G retain the shared compact Hornet exterior and a simplified
front panel; the limitations document defines that scope.

Final validation:

- Rust: 332 game tests and 16 server tests passed, with no failures or ignored
  tests. Release builds for Linux and Windows completed with no warnings.
- Python: all 10 importer tests and three warning-speaker DSP tests passed.
- Native UI checks passed on Linux and Windows, including actual pointer drag,
  clamping outside the slider, release, clean loadout and manual ejection after
  structural failure. Windows future-UI checks also passed.
- All 144 layout cases passed: 42 Linux and 42 Windows settings/controller cases
  at 640x360, 1280x720 and 1920x1080, plus 60 Linux loadout cases at 640x360 across
  all six aircraft, five missions and both prompt styles.
- The native visual matrix produced 27 Linux and 27 Windows captures of panels,
  exterior canopy wells, damage and drogues, including the Su-27 slats and brake.
  All six Windows aircraft views were visually reviewed in contact sheets.
- Blender produced four editable scenes and 16 top/side/front/quarter renders.
  Ten mesh exports from the final Linux binary are byte-identical to the meshes
  in those reviewed scenes.
- Real Linux game processes passed direct UDP, external TCP relay and embedded
  TCP host dogfights. Native Windows passed the embedded host dogfight. Reports
  verify remote input, frames, simulation ticks, gun inventory consumption and
  disabled microphone capture. These are loopback checks, not two-PC/router tests.
- Windows PE resources contain 16, 24, 32, 48, 64 and 256 px icons. A native HWND
  probe confirmed nonzero small/large window and class icons.
- The four distribution checksums match `dist/SHA256SUMS.txt`; the inspected local
  Windows copy is identical to the distributed executable.

One Linux multiplayer run timed out while other native graphics suites were
active. Serial execution passed all three transports with the original deadlines
and assertions. No test was skipped or relaxed. Logs and Linux captures are in
`target/todo-review/`; native Windows evidence is in this chat's local
`todo-review/` artifact directory. The definitive multiplayer log is
`target/todo-review/multiplayer-linux-final.log`.

Resource review:

| Resource | Before | Final |
| --- | ---: | ---: |
| Su-27 source brake triangles | 116 | 100 |
| Su-27 raw model bytes | 29,177,391 | 29,175,279 |
| Su-27 packed model bytes | 6,999,266 | 6,998,954 |
| Linux game bytes | 12,961,536 | 12,991,624 |
| Windows game bytes | 12,769,792 | 12,798,976 |

The assembled reviewed airframes contain 9,033 triangles for each Hornet-family
variant and 9,826 for Su-27, including gear and movable parts. No new raster
texture is added. Other imported packed resources retain their original bytes.

Final game SHA-256:

- Linux: `e05c8ba00b4a4a04fff6b6b47c151f17fea8c318ce04f4d0898becc3980b72e8`.
- Windows: `d05e1a142481a137ccedcaa3f50770c0cf0ae2924885a1b15a3a83d979b3c4c9`.

The sections below retain their historical snapshot hashes and validation scope.

## Integrated controller precision, audio and multiplayer, 28 October 2026

Final distributions contain the controller setup page, optional resting-drift
calibration, independent radial dead zones, center response and zoom precision,
optional L3 rudder, revised jet audio, final flat Su-27 metal and Hornet bow,
and the in-game multiplayer host. Sources and binaries were integrated after
the other active chats completed. Controller choices and sources are documented
in [CONTROLLER-PRECISION.md](CONTROLLER-PRECISION.md).

Final checks:

- Rust: 321 game tests and 16 server tests passed, with no failures or ignored tests.
- Python: all 10 importer checks and three warning-speaker checks passed. Neural
  synthesis dependencies now load only for regeneration; DSP checks need NumPy.
- Controller UI integration passed on Linux and native Windows, using actual
  mouse, keyboard and controller dispatch, L3 rudder and calibration cancellation.
- All 48 controller/configuration layouts passed at 640x360, 1280x720 and
  1920x1080, with both prompt styles. The final native Windows executable also
  passed nine setup/calibration/L3 layouts at those three sizes.
- Native Windows future-UI checks passed, including hosting, ready/launch/leave
  and clipboard invitation handling.
- Real game processes passed all 18 Linux cases (six modes with UDP, external TCP
  and embedded TCP) and all six native Windows embedded-host modes.
- Microsoft Defender reported no threats in either final Windows executable.
- The four distribution checksums match `dist/SHA256SUMS.txt`.

Native multiplayer QA now uses a dedicated configuration directory, rather than
inheriting the player's fullscreen, key or controller preferences. Two inherited
fullscreen windows could suspend one another on Windows and prevent reports from
exiting. The isolated run passed all six modes with the original assertions and
deadlines. No test was skipped or relaxed. Results remain loopback checks, not
physical router or two-PC acceptance tests. Controller drift and button behavior
were checked with synthetic readings and dispatch; physical-controller feel still
depends on the player's hardware and the chosen settings.

Final game hashes:

- Linux: `8ed73162d282dac9d84d65fc4b459ea1789056899f70c3869520743acc81292c`.
- Windows: `e4300d2ec6882b9cc9b61c89266a82e15dabdc6a4fc3b433361e3822a8d77b74`.

Reports, native captures and integration logs are under
`target/controller-review/`. The inspected Windows copy matches the distributed
binary byte for byte. The following older sections retain their own snapshot
hashes and validation scope.

## Su-27 metal and circular Hornet bow, 28 October 2026

The Su-27 engine shrouds, underside metal and nozzle petals use four flat grey
materials. Original positions, normals and the camouflage QOI are preserved.
The F/A-18C has a circular upper bow, rounded mirrors seated inside its aft lip
and short paired hinges. The original sill contacts remain connected. Reference,
dimensions, resource counts and limitations are in
[NOZZLE-CANOPY-STYLE.md](NOZZLE-CANOPY-STYLE.md).

The combined model and in-game multiplayer snapshot passed 311 game tests and
16 server tests. All 10 importer tests passed. Native inspection produced 67
Linux airframe/cockpit captures and 21 Windows mirror captures. Blender review
saved the exact cockpit, cutaway views, source frame and Navy photo in
`target/model-review/texture-bow/cockpit/blender/f18-cockpit-with-references.blend`.
Regression coverage includes the circular cap, physical mounts, reflection
bounds and small head movements. Asset comparison confirms that only colour
and UV change on 1,491 metal vertices; no camouflage pixel changes.

The pre-underside build passed all 18 Linux real-process multiplayer cases
(six modes, direct UDP, external TCP relay and embedded relay). Initial runs
overlapped several graphics capture suites and timed out. Serial execution
passed with the same assertions and deadlines. This is an environment scheduling
requirement for native QA, rather than a relaxed test. Per-case reports and logs
are under `target/model-review/texture-bow/`.

The same retained Windows snapshot passed native future-UI interaction checks,
external Su-27 views and all six embedded-host modes, verified in separate runs.
Repeated suite launches intermittently exceeded the unchanged eight-second
invitation startup deadline during concurrent development. Four modes passed in
`windows-local-qa.log`; escort and defense passed in `windows-remaining.log`.
Windows per-case reports are retained in the chat's local visualization directory,
under `model-qa/`. The definitive underside material extension passed all ten
importer checks and was built and inspected in the Linux native view
`texture-bow/su27-underside-final.png`.

That snapshot's Windows game is 12,753,408 bytes, SHA-256
`4be9e57b3c635ae85a603819dcf4ab5bee049c02dede40a048dc90f936acafec`;
Linux game is 12,944,304 bytes, SHA-256
`3c799fddbedafb83aa30090b27c7c8890abc5b0cfabf86c3d8374fb583cdefd0`.
Both Windows executables passed Defender without threats. These hashes identify
the retained `texture-bow/bin/` snapshot, before the final underside material
extension and concurrent controls/audio edits. Rebuild final distributions and
refresh affected native captures and hashes after all active chats finish.

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
