# Multiplayer voice

Voice starts disabled. Creating VoiceChat or configuring preferences never opens
an input device. The UI must require an explicit choice of Push to talk or Voice
activation for the current multiplayer session.

## Integration

Use VoiceChat::new(), then configure(VoiceSettings). Call tick(ptt_held, focused)
each frame, including when menus are open, and playback(&audio) immediately
afterwards. Forward drain_outgoing() packets through the authenticated session
transport. Deliver received voice with receive(authenticated_peer_id, seq, data).
Do not trust a peer ID claimed inside a voice message.

Call disconnect() when leaving multiplayer and then playback(&audio) to clear
the mixer. Disconnect returns the mode to Disabled, so a later session requires
opting in again. Loss of focus, local mute, deafen and releasing PTT cancel the
capture worker, discard pending input and stop transmission. Changing only
output volume does not restart the microphone.

VoiceSettings fields:

| Field | Meaning |
| --- | --- |
| mode | Disabled, PushToTalk or VoiceActivation |
| input_gain | 0 to 4, applied before the gate and encoder |
| output_gain | 0 to 2, independent from game engine volume |
| activation_threshold | RMS after input gain, 0.001 to 0.5 (default 0.02) |
| hold_ms | Tail hold, 0 to 1000 ms (default 240) |
| muted | Local microphone mute |
| deafened | Discard all incoming voice and mute the microphone |

Use set_peer_muted(peer, bool), set_peer_gain(peer, 0..2) and remove_peer(peer).
These controls flush queued output through the next playback() call. Muted and
deafened streams retain a sequence floor, preventing previously discarded
packets from being replayed when listening resumes. Input and output queues
also discard stale audio after stalls.

capturing(), transmitting(), input_level(), capture_error() and peer_talking(id)
provide UI state. Capture failures are reported and retried at most once every
five seconds while capture is still requested. PTT opens the device only while
held; its first packet therefore includes the platform's device startup delay.
Voice activation uses an amplitude threshold, a 40 ms pre-roll and a bounded
tail hold; it is not a learned speech classifier.

## Format and bounds

The voice payload is one independent IMA ADPCM block: 16 kHz mono, 320 samples,
20 ms, exactly 164 bytes (65.6 kbit/s before transport overhead). The first two
bytes are the predictor, signed 16-bit little endian; byte 2 is the IMA step
index (0..88); byte 3 is zero. The next 160 bytes contain 319 difference
nibbles, low nibble first; the unused final high nibble must be zero. Packets
with any other length or invalid headers are rejected.

The transport carries the separate wrapping u32 sequence from VoicePacket.
Lost blocks do not alter the predictor of subsequent blocks. A per-peer 60 ms
jitter delay reorders nearby packets, rejects duplicate/old sequences and holds
at most 12 frames. Two missing frames use a decaying previous frame, then stop;
a long UI stall resumes from fresh speech instead of replaying a backlog.

There are at most 16 peers, eight captured frames, eight outgoing packets and
48 decoded pending playback frames. Audio::voice_pcm adds at most 120 ms of
resampled PCM per participant; it accepts only 8 to 48 kHz mono and at most
200 ms per call. Excess old audio is discarded. Individual and combined gains
are limited, edges fade, game sound ducks smoothly and aircraft warnings take
priority. Crew voice remains available while the game audio bus is paused.

## Capture backends

Windows uses WinMM waveIn with prepared 20 ms buffers, polling, reset,
unprepare and close on the owning worker. Linux loads PulseAudio dynamically
(including a PipeWire PulseAudio server), uses a nonblocking mainloop and
releases the stream on cancellation. ALSA default capture is the fallback,
with nonblocking reads and bounded polling. Suspend errors close the stream
instead of entering an unbounded driver recovery loop. Neither backend adds a
build-time library dependency to the game.

References:

- [Windows waveInOpen](https://learn.microsoft.com/en-us/windows/win32/api/mmeapi/nf-mmeapi-waveinopen)
- [Windows waveInReset](https://learn.microsoft.com/en-us/windows/win32/api/mmeapi/nf-mmeapi-waveinreset)
- [Windows polling audio buffers](https://learn.microsoft.com/en-us/windows/win32/multimedia/managing-data-blocks-by-polling)
- [PulseAudio asynchronous mainloop](https://www.freedesktop.org/software/pulseaudio/doxygen/mainloop_8h.html)
- [PulseAudio recording streams](https://www.freedesktop.org/software/pulseaudio/doxygen/stream_8h.html)
- [ALSA PCM capture and recovery](https://www.alsa-project.org/alsa-doc/alsa-lib/group___p_c_m.html)

This implementation has no acoustic echo cancellation. A headset avoids feeding
game audio back into voice activation. Hardware/OS permissions can still reject
the default input device; the UI exposes the reported error. Automated tests
exercise synthetic PCM and never enable a physical microphone. Both platform
backends are compiled, and the synthetic tests run on Linux and Windows;
physical microphone capture is intentionally not claimed as hardware-tested.

## Aircraft warnings

F-16 and F/A-18 use the existing English neural warning family; MiG-29 and Su-27
use the existing Russian family. New-aircraft phrasing is a game reconstruction,
not a recording or a claim of exact NACES, NATOPS or Su-27 warning-system fidelity.
The high-AoA speech threshold for the Russian family is one degree below each
airframe's nominal game control cap. Landing-configuration cue speed scales
with the gear limit; those thresholds remain simplified gameplay rules.
