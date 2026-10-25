# Multiplayer

Open ONLINE from the main menu. The flight school and normal local missions
remain available without a network connection.

## Direct play on a local network

1. Choose a callsign and aircraft. The host selects a mode and room capacity.
2. Leave DIRECTORY blank, choose a UDP port (42860 by default), then HOST.
3. COPY INVITE copies the complete address and random invitation key.
4. Other pilots paste the invitation with Ctrl+V into INVITATION and choose JOIN.
5. Each pilot marks READY. The host launches when at least two pilots are ready.

The host's firewall must allow the chosen UDP port. A direct invitation uses
the host's local interface address. For direct Internet play, substitute the
public address and forwarded UDP port before the #, preserving the key.

## Internet directory and relay

Run the included server on a machine reachable by every pilot:

~~~sh
./dist/viper-server-linux-x86_64 0.0.0.0:42861
~~~

On Windows run viper-server-windows-x86_64.exe with the same bind argument.
Permit inbound UDP 42861 on that machine. This is the directory/relay service;
the player who creates a room remains the simulation host.

Enter that machine's address and port in DIRECTORY before hosting. The
invitation then starts with relay: and routes encrypted game and voice packets
through the server. The host does not need an inbound forwarded game port.
The directory never needs the key for a private room.

PRIVATE rooms require their full invitation. PUBLIC rooms publish their
invitation through BROWSE. Choose a listed room or QUICK MATCH to join an open
room matching the selected mode. Public invitations are available to everyone
who can browse that directory. A running sortie stops advertising its room.

Direct rooms use peer UDP. Directory-backed rooms use the relay explicitly;
there is no claim of automatic NAT traversal, a hosted default service,
accounts, ranked matchmaking or dedicated authoritative physics servers.
Deploying the supplied directory on an Internet-reachable host is required
for Internet relay play. Direct invitations can be used without a directory.

## Play, objectives and voice

The six modes are PVP DOGFIGHT, PVP TEAMS, COOP COMBAT, COOP STRIKE, COOP
ESCORT and COOP AIR DEFENSE. Their real objective rules, waves, scoring and
failure conditions are in [MULTIPLAYER-MISSIONS.md](MULTIPLAYER-MISSIONS.md).
PVP respawns a lost aircraft after five simulation seconds. Cooperative
missions give each human one life; a lost pilot watches the ongoing sortie.
Losing every human pilot ends a cooperative operation. Aircraft collisions
are resolved by the host, including swept crossing paths.

Blue markers identify allied pilots and the protected escort. ONLINE / VOICE
in the configuration drawer returns to the room panel for participant mute,
gain and scores. Resume returns to the sortie. Leaving removes the local
session. Opening configuration pauses local controls while the shared world
continues. Restarting a local mission is unavailable while a room is active.

The microphone starts disabled in every new connection. Select PUSH TO TALK
or VOICE DETECTION to enable it. The default keyboard PTT key is U and is
remappable in configuration. Controller View/Back is PTT online; Start opens
configuration. Voice provides microphone mute, deafen, separate input/output
gain, detection threshold and individual participant mute/gain. The live
input meter and capture errors appear in the room panel. See
[VOICE-CHAT.md](VOICE-CHAT.md) for capture and codec details.

## Simulation and transport

The host integrates every human aircraft at the existing 120 Hz physics step.
Remote controls reuse the local aircraft's sensors, missile launch rules,
gun, countermeasures, damage and stores accounting. Clients submit controls
at up to 60 Hz and receive world snapshots at 20 Hz. They predict their local
aircraft and extrapolate remote motion between snapshots; they do not decide
damage, kills, ammunition or objectives.

Packets use XChaCha20-Poly1305 authenticated encryption and operating-system
random keys/nonces. The invitation is a room credential: share private
invitations only with intended participants. Packet replay windows accept
bounded reordering; sizes, fragmentation, reassembly, peers, directory leases,
voice jitter and receive work are bounded. Inputs expire to neutral after
300 ms without an update. Peers time out after twelve wall-clock seconds.

Sessions support at most eight pilots. The host must remain running and able
to simulate the game; there is no host migration or reconnect into an active
sortie. If the host disconnects, clients stop the sortie. When every opponent
leaves a PVP room, the host ends the sortie. Internet relay adds network delay;
high latency and loss can produce visible prediction corrections. This
implementation does not promise competitive anti-cheat against room members.

The directory is an intentionally small service, capped at 128 live rooms.
BROWSE returns up to four public rooms per response. Room, peer and
per-source request limits bound memory and traffic. Use a trusted directory;
public room discovery is not an authenticated identity service.

Library references:
[Rust UDP sockets](https://doc.rust-lang.org/stable/std/net/struct.UdpSocket.html),
[RustCrypto authenticated encryption](https://docs.rs/chacha20poly1305/0.10.1/chacha20poly1305/),
[Postcard serialization](https://docs.rs/postcard/1.1.3/postcard/).

## Reproducible verification

~~~sh
cargo test --release
python3 tools/check_multiplayer.py
~~~

The Python check launches actual game processes for all six modes. It checks
control reception, replicated frames and selected airframe, remote ammunition
use and microphone inactivity; the Teams case uses the real directory/relay
process. A graphical OpenGL session is required. Use --game and --server to
select custom builds, --modes to narrow a diagnostic run, and --output to retain
reports. All connections are loopback. The report switches deliberately use
scripted controls even when the QA windows are in the background and never
save user settings.

~~~sh
viper-run --future-ui-check --screen menu --shot future-menu.png --frames 200
viper-run --tutorial landing --jet su27
viper-run --host 127.0.0.1:42860 --callsign Lead --online-mode combat
viper-run --join 'HOST:PORT#KEY' --callsign Wing
~~~

Voice codec, gates and jitter are verified with synthetic audio on Linux and
Windows. Physical microphone capture and Internet paths across independent
networks still require a hardware/deployment acceptance check; the included
tests do not silently record a microphone.
