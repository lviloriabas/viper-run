# Validation

The delivered source and release executables were checked with:

- cargo test --release: 246 game tests and 6 standalone server tests, no failures.
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
