# Warning speech

These ten mono, 16-bit PCM WAV clips were generated specifically for this project
with eSpeak NG. They are original speech synthesis, not original aircraft tapes,
voice imitations or audio extracted from DCS, FlightGear or other games.

Regenerate from the repository root with:

```sh
python3 tools/generate_warning_voices.py
```

The generator requires `espeak-ng`, uses the `en-us+f3` and `ru+f3` voices at
170 words/minute, trims silence and normalises peaks to 24000/32768. The engine
loads and resamples the embedded clips to its 44.1 kHz mono output, plays one
voice at a time and ducks engine/wind beneath it. Only a higher priority cue
interrupts speech; continuous conditions have repeat cooldowns.

| Files | Spoken cue |
| --- | --- |
| f16-pullup, f16-altitude | Pull up, Altitude (each repeated twice) |
| f16-warning, f16-caution | Warning, Caution (each repeated twice) |
| mig-altitude | Опасная высота |
| mig-aoa | Предельный угол атаки |
| mig-speed | Предельная скорость |
| mig-g | Предельная перегрузка |
| mig-gear | Выпусти шасси |
| mig-fire | Пожар двигателя |

F-16 low-speed/high-AOA and landing configuration cues use tones, rather than
inventing spoken STALL or AOA announcements. The MiG Russian phrases are game
reconstructions of the documented warning categories; their cadence and exact
wording are not certified reproductions of a particular RI-65 tape or variant.
No redistributable, authenticated cockpit recordings were identified during
this pass. The runtime wiring and replacement points are in `src/audio.rs`.
Research sources and limitations are in [COCKPIT-REFINEMENT.md](../../docs/COCKPIT-REFINEMENT.md).
