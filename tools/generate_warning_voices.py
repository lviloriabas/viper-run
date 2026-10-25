#!/usr/bin/env python3
"""Generate embedded neural VMS clips; see assets/voices/README.md for setup."""

import argparse
import hashlib
import io
import json
from pathlib import Path
import wave

import numpy as np
import onnxruntime
from piper import PiperVoice, SynthesisConfig
from vosk_tts import Model, Synth

ROOT = Path(__file__).resolve().parents[1]
RATE = 22050
SEED = 20261025
CLIPS = {
    "f16-pullup": "Pull up.",
    "f16-altitude": "Altitude.",
    "f16-warning": "Warning.",
    "f16-caution": "Caution.",
    "mig-altitude": "Опасная высота.",
    "mig-aoa": "Предельный угол атаки.",
    "mig-speed": "Предельная скорость.",
    "mig-g": "Предельная перегрузка.",
    "mig-gear": "Выпусти шасси.",
    "mig-fire": "Пожар двигателя.",
}


def biquad(samples: np.ndarray, cutoff: float, highpass: bool) -> np.ndarray:
    """Second-order Butterworth section at the source sample rate."""
    omega = 2 * np.pi * cutoff / RATE
    cosine = np.cos(omega)
    alpha = np.sin(omega) / np.sqrt(2)
    a0 = 1 + alpha
    if highpass:
        b0 = (1 + cosine) / 2 / a0
        b1 = -(1 + cosine) / a0
    else:
        b0 = (1 - cosine) / 2 / a0
        b1 = (1 - cosine) / a0
    b2 = b0
    a1 = -2 * cosine / a0
    a2 = (1 - alpha) / a0
    output = np.empty_like(samples, dtype=np.float64)
    x1 = x2 = y1 = y2 = 0.0
    for index, x in enumerate(samples):
        y = b0*x + b1*x1 + b2*x2 - a1*y1 - a2*y2
        output[index] = y
        x2, x1, y2, y1 = x1, x, y1, y
    return output


def cockpit_speaker(samples: np.ndarray) -> np.ndarray:
    """Limited-bandwidth cockpit intercom with gentle amplifier saturation."""
    samples = biquad(samples, 280, highpass=True)
    samples = np.tanh(samples * 1.6 / 32768) * (32768 / 1.6)
    return biquad(samples, 3600, highpass=False)


def prepare(pcm: np.ndarray) -> np.ndarray:
    """Keep consonant tails, remove DC, match level and soften cut boundaries."""
    samples = pcm.astype(np.float64)
    if not len(samples) or not np.isfinite(samples).all():
        raise ValueError("Invalid synthesized speech")
    samples -= samples.mean()
    peak = np.max(np.abs(samples))
    if peak < 100:
        raise ValueError("Synthesized speech is silent")
    # Leave 35 ms before and 70 ms after the audible phrase. Do not gate
    # interior pauses; keep consonant endings before the speaker processing.
    audible = np.flatnonzero(np.abs(samples) > max(60, peak * 0.006))
    start = max(0, audible[0] - round(RATE * 0.035))
    end = min(len(samples), audible[-1] + 1 + round(RATE * 0.070))
    samples = samples[start:end].copy()
    rms = np.sqrt(np.mean(samples**2))
    # RMS matching keeps short and long cues equally intelligible; peak limiting
    # leaves over 2 dB of headroom without compression or clipped syllables.
    samples *= min(4800 / rms, 24000 / np.max(np.abs(samples)))
    samples = cockpit_speaker(samples)
    rms = np.sqrt(np.mean(samples**2))
    samples *= min(4800 / rms, 24000 / np.max(np.abs(samples)))
    fade = min(round(RATE * 0.008), len(samples) // 2)
    ramp = np.linspace(0, 1, fade)
    samples[:fade] *= ramp
    samples[-fade:] *= ramp[::-1]
    return np.rint(samples).astype("<i2")


def sha256(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--models-dir", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, default=ROOT / "assets/voices")
    parser.add_argument("--only", choices=CLIPS, action="append",
                        help="Regenerate only this clip, preserving the remaining manifest entries")
    args = parser.parse_args()
    selected = set(args.only or CLIPS)
    needs_russian = any(name.startswith("mig-") for name in selected)
    english_path = args.models_dir / "en_US-ljspeech-high.onnx"
    russian_path = args.models_dir / "vosk-model-tts-ru-0.9-multi"
    model_files = [english_path, Path(str(english_path) + ".json")]
    if needs_russian:
        model_files += [russian_path / name for name in ("model.onnx", "config.json", "dictionary", "bert/model.onnx", "bert/vocab.txt")]
    if not all(path.is_file() for path in model_files):
        parser.error("Models missing; follow assets/voices/README.md to download them")

    # Seed before session creation: ONNX random nodes initialise their own RNGs.
    onnxruntime.set_seed(SEED)
    english = PiperVoice.load(english_path)
    russian = Synth(Model(model_path=russian_path)) if needs_russian else None
    settings = SynthesisConfig(length_scale=0.95, noise_scale=0.55,
                               noise_w_scale=0.65, normalize_audio=False, volume=0.7)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    manifest = {
        "sample_rate": RATE,
        "seed": SEED,
        "engines": {"piper-tts": "1.4.2", "vosk-tts": "0.3.61"},
        "models": {str(path.relative_to(args.models_dir)): sha256(path) for path in model_files},
        "russian_speaker_id": 2,
        "english_settings": {"length_scale": 0.95, "noise_scale": 0.55, "noise_w_scale": 0.65},
        "russian_settings": {"speech_rate": 1.04, "noise_level": 0.55, "duration_noise_level": 0.65},
        "cockpit_speaker": {"highpass_hz": 280, "lowpass_hz": 3600, "saturation_drive": 1.6},
        "clips": {},
    }
    manifest_path = args.output_dir / "manifest.json"
    if args.only and manifest_path.is_file():
        previous = json.loads(manifest_path.read_text(encoding="utf-8"))
        previous["models"].update(manifest["models"])
        manifest = previous
    for index, (name, text) in enumerate(CLIPS.items()):
        if name not in selected:
            continue
        onnxruntime.set_seed(SEED + index)
        if name.startswith("f16-"):
            stream = io.BytesIO()
            with wave.open(stream, "wb") as output:
                # This narration model merges the two syllables on a short
                # "Pull up." command. A word break and fixed duration predictor
                # preserve both words through the cockpit speaker effect.
                spoken_text = "Pull, up." if name == "f16-pullup" else text
                cue_settings = SynthesisConfig(length_scale=1.10, noise_scale=0.4,
                    noise_w_scale=0.0, normalize_audio=False, volume=0.7
                ) if name == "f16-pullup" else settings
                english.synthesize_wav(spoken_text, output, syn_config=cue_settings)
            stream.seek(0)
            with wave.open(stream, "rb") as source:
                if source.getframerate() != RATE or source.getnchannels() != 1 or source.getsampwidth() != 2:
                    raise ValueError("Unexpected Piper output format")
                pcm = np.frombuffer(source.readframes(source.getnframes()), dtype="<i2")
            pcm = prepare(pcm)
            # Fixed gap and identical repeats give a concise warning cadence,
            # avoiding the sentence-length pauses of a narration model.
            pcm = np.concatenate((pcm, np.zeros(round(RATE * 0.16), dtype="<i2"), pcm))
        else:
            pcm = prepare(russian.synth_audio(text, speaker_id=2, speech_rate=1.04,
                          noise_level=0.55, duration_noise_level=0.65, scale=0.7))
        duration = len(pcm) / RATE
        if not 0.4 <= duration <= 3.0:
            raise ValueError(f"{name}: warning duration {duration:.2f}s is outside 0.4-3.0s")
        path = args.output_dir / (name + ".wav")
        with wave.open(str(path), "wb") as output:
            output.setnchannels(1)
            output.setsampwidth(2)
            output.setframerate(RATE)
            output.writeframes(pcm.tobytes())
        manifest["clips"][name] = {
            "text": text, "repeats": 2 if name.startswith("f16-") else 1,
            "frames": len(pcm), "sha256": sha256(path),
        }
        if name == "f16-pullup":
            manifest["clips"][name]["synthesis_text"] = "Pull, up."
            manifest["clips"][name]["synthesis_settings"] = {
                "length_scale": 1.10, "noise_scale": 0.4, "noise_w_scale": 0.0,
                "normalize_audio": False, "volume": 0.7,
            }
        print(f"{name}: {duration:.2f}s", flush=True)
    manifest_path.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
