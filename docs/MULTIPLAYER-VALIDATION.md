# In-game hosting validation, 2026-10-28

Base: 70d9809. The multiplayer change was isolated from the unfinished aircraft
integration in the original checkout. No model or aircraft geometry was changed
in this worktree.

The delivered Windows game is 35,475,456 bytes. SHA-256:
`2482c40c5d4315ea4efcb5936ff9b4448a254b67259406d0325d5fd7f8f12883`.
`dist/SHA256SUMS.txt` covers the Linux game and dedicated servers as well.

## Completed checks

- `cargo test --release --quiet`: 307 game tests and 16 server tests passed.
  The hosting checks connect two pilots through the owned relay, check listener
  cleanup, discover a public room while hiding a private room, and validate public
  invitation addresses and the actual host port.
- Linux real-process multiplayer: all six modes passed with direct UDP, the
  external TCP relay and the embedded relay, 18 cases total. Inputs, remote frames,
  airframe, ammunition and disabled microphone state were checked.
- Windows real-process multiplayer: all six modes passed with two game processes
  and the embedded server. No viper-server process was launched for these cases.
- Native Windows `--future-ui-check`: passed hosting from the menu, launch gating,
  ready/launch/leave with mouse/keyboard/controller, external browser join, native
  clipboard PASTE AND JOIN, connection help, public address editing, generated
  invitation credential preservation and copied router instructions.
- Network menu layout: 84 scenes passed at 640x360, 1280x720 and 1920x1080,
  with keyboard and controller prompts. The host-port help scene was inspected.
- Microsoft Defender scanned both final Windows executables without threats.
  The scan used `-DisableRemediation` and changed no security settings.
- `git diff --check`: passed.

## Acceptance limits

Process connection tests use loopback. Broadcast discovery was exercised locally;
two independent physical PCs and two separate Internet connections still need an
acceptance check. UPnP discovery/mapping is best effort and was not established
through a physical router in this validation. Router forwarding, Windows policy,
double NAT or CGNAT can prevent remote access. Manual invitation generation does
not assert that a public port is reachable. The application provides actual host
IP/port instructions, public-address validation and an external-relay option.
The successful Defender scan applies to these hashes and this scanner state.

Rebuild and repeat affected checks after integrating aircraft or other changes;
the executables and hashes above represent this isolated source revision only.
