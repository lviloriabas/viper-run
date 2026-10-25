# Warning speech

The ten embedded mono, 16-bit PCM WAV clips use neural synthesis at 22,050 Hz.
The F-16 uses Piper's US English female `en_US-ljspeech-high` voice. The MiG
uses Vosk TTS `vosk-model-tts-ru-0.9-multi`, female speaker 2
(`female_2`). These replace the former eSpeak formant voices, especially
its metallic Russian vowels and consonants.

Phrases keep natural pitch and articulation. English cues repeat twice with a
fixed 160 ms gap between the padded phrases; Russian cues play once at 1.04
times the model's normal speaking rate. Pull up has an explicit word break,
a 1.10 length scale and zero duration noise so both words remain distinct through
the speaker effect. Its written synthesis input is `Pull, up.`; the intended
spoken warning remains "Pull up". The generator removes DC and excess
leading/trailing silence, preserves consonant tails, adds 8 ms boundary fades
and matches RMS levels with a peak ceiling of 24000/32768. A cockpit speaker
effect uses a 280 Hz high-pass, gentle amplifier saturation and a 3.6 kHz
low-pass, then restores the cue level. It gives the warnings an intercom/radio
character while retaining intelligible consonants. The effect is baked into the
WAV files, so previews and gameplay use the same audio. No pitch shifting or
runtime speech service is used.

The engine resamples the clips to 44.1 kHz, plays one voice at a time and ducks
engine/wind beneath it. Only a higher priority cue interrupts speech;
continuous conditions retain their repeat cooldowns. The neural models and
Python packages are authoring tools only and are not needed to play the game.

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

## Regeneration

Use Python 3.12 or later, with the pinned authoring dependencies:

```sh
python3 -m venv /tmp/viper-voice-env
/tmp/viper-voice-env/bin/pip install -r tools/requirements-voices.txt
mkdir -p /tmp/viper-voice-models
/tmp/viper-voice-env/bin/python -m piper.download_voices \
    --download-dir /tmp/viper-voice-models en_US-ljspeech-high
curl -fL -o /tmp/viper-voice-models/russian.zip \
    https://alphacephei.com/vosk/models/vosk-model-tts-ru-0.9-multi.zip
python3 -m zipfile -e /tmp/viper-voice-models/russian.zip /tmp/viper-voice-models
/tmp/viper-voice-env/bin/python tools/generate_warning_voices.py \
    --models-dir /tmp/viper-voice-models
```

Run the speaker-effect checks with `python -m unittest discover -s tools -p test_warning_voices.py` in the authoring environment.

The generator accepts `--output-dir` to preview a set elsewhere and
`--only f16-pullup` to replace one cue while preserving other clips. It records
texts, repetition counts, settings, seeds, model hashes and output hashes in
`manifest.json`. Neural inference is stochastic and ONNX kernels can differ between platforms;
the committed WAV files are the playback reference.

## Sources and licenses

- [Piper LJSpeech model card](https://huggingface.co/rhasspy/piper-voices/blob/main/en/en_US/ljspeech/high/MODEL_CARD).
  The [LJ Speech dataset](https://keithito.com/LJ-Speech-Dataset/) is public domain;
  the [Piper voice repository](https://huggingface.co/rhasspy/piper-voices) lists MIT.
- [Vosk TTS, Alpha Cephei](https://github.com/alphacep/vosk-tts), Apache 2.0.
  The Russian model archive linked above includes its Apache 2.0 license
  declaration in `README.md`. It includes the SberDevices ruBert encoder
  (Apache 2.0), used by the offline authoring pipeline.
- [Piper synthesis software](https://github.com/OHF-Voice/piper1-gpl) is GPL-3.0.
  Only the generated WAV files are embedded; the software and models are not
  linked into or distributed with the executable.

These are newly synthesized game phrases, not original aircraft tapes or audio
extracted from other games. F-16 low-speed/high-AOA and landing configuration
cues use tones. The MiG phrases reconstruct documented warning categories;
their cadence and wording are not certified reproductions of an RI-65 tape.
Runtime replacement points are in `src/audio.rs`; aircraft references are in
[COCKPIT-REFINEMENT.md](../../docs/COCKPIT-REFINEMENT.md).
