# Compact exterior references, 27 October 2026

FlightGear models were downloaded for measurement and comparison. Their full
meshes and textures are not added to the executable. The existing procedural
F-16C and MiG-29S retain their production envelopes, animated surfaces,
hardpoints and current cockpit geometry.

## F-16

Source: [NikolaiVChr/f16, Models/f16.ac](https://github.com/NikolaiVChr/f16/blob/master/Models/f16.ac).
Downloaded AC3D SHA-256:
`bd0e90c58456f64baef675b66f7b298ae8f7d72d6c3018f9459a64bcd751ff20`.

The source AirIntake mouth spans about 1.51 m laterally and 0.525 m vertically,
with a broad upper lip and rounded lower section. The game's previously narrow
mouth is widened from 1.00 to 1.24 m, with a flatter upper profile and a smooth
transition into the existing trunk. It retains the game's longitudinal datum
and inlet height. Four compact interrogator blades follow the source
Link16Antennas arrangement ahead of the windshield; their bases follow the
existing fuselage surface. Overall length, span and landing-gear height remain
guarded by the original geometry tests. This is a reference-led adaptation,
not a claim that every FlightGear variant matches the current F-16C.

## MiG-29

Source: [FlightGear aircraft archive, Mig-29.zip](https://mirrors.ibiblio.org/flightgear/ftp/Aircraft-trunk/Mig-29.zip),
Models/Mig-29.ac. Downloaded archive SHA-256:
`38832d383f180c8a2f7f14145c06812fd5bc94429e6e11d1be573922f55f0d7e`.

The auxiliary-intake assembly spans about 1.205 m along the source fuselage
and 0.615 m across each LERX, with five separate leaves. The game adapts those
to about 1.16 by 0.59 m, immediately aft of the canopy, replacing the earlier
flat narrow dark rectangles with painted sloping leaves, side walls and
shallow gaps. Its existing 9.13S dorsal spine remains specific to that variant.
The source's legacy dimensions and variant are not applied as a global scale.
Original 17.32 m length, 11.36 m span and landing-gear height tests remain intact.

All additions are small procedural meshes. Downloaded reference geometry stays
under ignored target/model-review/compact-references. The source URLs and
hashes make the measurements traceable without shipping large source models.
