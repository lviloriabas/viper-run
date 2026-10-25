"""Authoring checks for the cockpit speaker effect; no neural models required."""

import unittest

import numpy as np

from generate_warning_voices import RATE, cockpit_speaker, prepare


class WarningVoiceTests(unittest.TestCase):
    def tone_level(self, frequency):
        time = np.arange(RATE) / RATE
        source = np.sin(2 * np.pi * frequency * time) * 2000
        filtered = cockpit_speaker(source)[RATE // 5:]
        return np.sqrt(np.mean(filtered**2))

    def test_speaker_restricts_bandwidth_while_preserving_speech_midrange(self):
        middle = self.tone_level(1000)
        self.assertGreater(middle, 1200)
        self.assertLess(self.tone_level(100), middle * 0.2)
        self.assertLess(self.tone_level(8000), middle * 0.1)

    def test_prepared_audio_has_headroom_faded_edges_and_no_dc(self):
        time = np.arange(RATE) / RATE
        phrase = np.sin(2 * np.pi * 900 * time) * 12000
        source = np.concatenate((np.zeros(RATE // 2), phrase, np.zeros(RATE // 2))) + 300
        output = prepare(source)
        self.assertEqual(output.dtype, np.dtype("<i2"))
        self.assertEqual(output[0], 0)
        self.assertEqual(output[-1], 0)
        self.assertLessEqual(np.abs(output.astype(np.int32)).max(), 24000)
        self.assertLess(abs(output.mean()), 5)
        self.assertGreater(len(output), RATE)
        self.assertLess(len(output), round(RATE * 1.12))

    def test_empty_silent_or_nonfinite_speech_is_rejected(self):
        for source in (np.array([]), np.zeros(100), np.ones(100), np.array([np.nan])):
            with self.subTest(source=source):
                with self.assertRaises(ValueError):
                    prepare(source)


if __name__ == "__main__":
    unittest.main()
