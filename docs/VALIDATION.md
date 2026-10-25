# Validation

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
