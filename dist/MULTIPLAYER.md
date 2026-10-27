# Multiplayer

Open ONLINE from the main menu. The flight school and normal local missions
remain available without a network connection.

## Server browser and Internet play

Use the current build of both the game and viper-server. ONLINE opens SERVERS.
Set CALLSIGN and aircraft, enter a running relay's address in RELAY SERVER and
choose REFRESH. A hostname without a port uses TCP 42861. The callsign and last
server address are remembered; invitation keys are never saved.

The browser refreshes every three seconds and shows six rooms per page, with
ALL MODES / mode filtering and PREV / NEXT pages. It displays room host, mode,
pilot count and FULL / JOIN. Select an open room to connect to its lobby.
Full rooms remain visible. A failed connection clears stale results and gives
a reason; REFRESH retries.

CREATE ROOM uses the same relay address. Choose mode, capacity and PUBLIC or
PRIVATE, then create. Wait for ROOM ONLINE, TCP RELAY. Public rooms appear in
the browser; private rooms require COPY INVITE. Guests use JOIN INVITE, paste
the entire invitation with Ctrl+V, press Enter and select JOIN ROOM. Relay
invitations already contain the server address.

Clicking a text field selects its contents for replacement. Ctrl+A selects
all, Backspace deletes and Esc cancels the edit. In the lobby, guests select
READY / WAIT. The host starts ready and selects LAUNCH once at least two
pilots are present and everyone is ready.

## Run the browser/relay server

There is no bundled or automatically deployed public relay. Run the included
server on a machine reachable by all players, or ask its operator for an address:

~~~sh
./dist/viper-server-linux-x86_64 0.0.0.0:42861
~~~

On Windows launch dist/viper-server-windows-x86_64.exe. The same bind address
is the default. Permit incoming TCP 42861 on the server machine only.
For Internet play use a VPS with a public IP, or forward TCP 42861 in the
server's router. Every player enters that public IP or DNS name.
Starting the server on your PC alone does not make it Internet-reachable.

For a same-PC check use 127.0.0.1:42861. For a LAN, run the server once and
give the other PCs that machine's LAN IP. Do not give other PCs 127.0.0.1:
it always refers to the PC on which it is entered.

Players, including the simulation host, use outbound TCP connections for
discovery, encrypted game state and voice. They bind no UDP socket and accept
no inbound connections on this path. Under the default Windows outbound
policy they need no inbound firewall exception or router port forwarding.
Network policies can still deny outbound traffic. Only the relay listens
for incoming connections. TCP can add delay when packets are lost; use a
nearby server.

The service also accepts the previous UDP relay protocol on the same port.
New browser/host connections use TCP and invitations start with relay+tcp:.
Do not mix game versions with different simulation formats.

## Direct LAN option

LAN DIRECT needs no relay:

1. Choose callsign, aircraft, mode, capacity and UDP port (42860).
2. Select HOST LAN ROOM. The host allows incoming UDP on its private network.
3. COPY INVITE; guests paste it in JOIN INVITE and select JOIN ROOM.
4. Guests mark READY / WAIT; the host selects LAUNCH.

Direct rooms are invitation-only and are not listed in the relay browser.
Windows can ask to allow the host on private networks. For direct Internet
invitations replace the address before # with the public address and forwarded
UDP port, preserving the key. Prefer the TCP relay for Internet games.

## Windows Defender, firewall and SmartScreen

The relay path removes the game's need for an inbound listener. It does not
alter firewall rules, antivirus settings or exclusions, and does not request
administrator access. The separate viper-server still needs its inbound TCP port.

A malware detection and a SmartScreen reputation warning differ from a firewall
prompt. These executables are unsigned: a source change cannot guarantee
SmartScreen or Defender will trust every build. Use a trusted release, verify
its SHA-256 and retain the exact warning. Publishers should sign releases
with a trusted Authenticode certificate and submit suspected false positives
to Microsoft. Do not disable protection to troubleshoot multiplayer.

Scan rebuilt Windows binaries without changing security settings:

~~~powershell
python tools/check_windows_defender.py
~~~

The JSON report includes SHA-256, scanner path, timestamp and scan result.
A successful local scan is evidence for those files and signatures; it does
not guarantee the result on another PC or identify an earlier unknown detection.

References: [Windows firewall rules](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/rules),
[SmartScreen reputation](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation),
[Microsoft file submission](https://www.microsoft.com/en-us/wdsi/filesubmission).

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

The TCP directory supports 128 live rooms and 256 connections, with six rooms
per page. Legacy UDP browsing returns up to four rooms. Room, peer and
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

The Python check launches actual game processes for all six modes through
both direct UDP and the new TCP relay. It checks
control reception, replicated frames and selected airframe, remote ammunition
use and microphone inactivity; every relay case uses the real server
process. A graphical OpenGL session is required. Use --game and --server to
select custom builds, --modes and --transport direct|tcp|both to narrow a diagnostic run, and --output to retain
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
