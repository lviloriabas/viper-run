---
name: aircraft-model-pipeline
description: Implement or revise Viper Run aircraft exterior and cockpit geometry, articulated controls, stores and pilot clearances, using measured references and compact assets. Apply to requests for another aircraft or corrections to an existing aircraft model.
---

# Aircraft model pipeline

Use this workflow when adding or correcting an aircraft in this repository.
Preserve the game's faceted style and live keyboard/controller controls.

## Reference and variant contract

Read [references/model-contract.md](references/model-contract.md) for coordinates,
integration points, reference provenance and the verification commands.
Identify the exact variant before building. Record authoritative dimensions and
at least a three-view or inspectable source model plus a cockpit reference.
Distinguish measured dimensions from fitted estimates. FlightGear sources are in
`assets/models/source`; `tools/import_flightgear.py --unpack-source PATH`
restores them. Track hashes, revisions, authors, licences and datum transforms.
Use commercial simulator screenshots as visual reference only, not redistributable
meshes or textures. Inspect downloaded licences before adopting geometry.

Open the reference and the exact assembled game geometry in Blender. A screenshot
alone cannot establish scale. Compare top, side, front, pilot-eye and cutaway views.
CAD conversions belong under ignored `target/model-review`. The optional
`tools/convert_cad_reference.py` records invalid reference faces; do not use
missing faces as evidence for dimensions.

## Geometry and animation

Use metres, right/up/aft axes, a single airframe datum, and one documented
pilot-eye transform shared by cockpit, stores, pilot and ejection. A different
variant needs its actual planform, canopy, intakes and stations.

Each movable skin belongs to exactly one articulated mesh. Remove its footprint
from the fixed skin. Preserve parent UVs at rest and close hinge, root and tip
edges. `src/articulation.rs` interpolates attributes when partitioning an
existing skin. Check partitioned faces and caps visually, especially concave cuts.
Align hinges with measured hinge endpoints, not a generic aircraft centre.
Drive tests and rendering with the same pose functions. Check neutral, each
extreme, intermediate and combined pitch/roll/yaw/flap/brake commands. Include
leading-edge devices when present. Do not add a dorsal brake to an aircraft
that brakes through its flight controls.

Fit pylon roots to the fixed wing skin behind moving leading edges. Check contact
along the root contour, not just a mounting point. Align store suspension points
with the pylon rails, accounting for each store's model origin.

Build instrument faces in orthonormal board frames. Space complete flanges and
casings, preserving image aspect. Keep important instruments readable from the
actual pilot eye. Close the tub below the sills while leaving physical knee and
pedal tunnels. Check source hardware and replacement hardware together.

Seat mirrors on the windshield/canopy mating bow and follow its local curve.
Calculate reflection mapping from the actual pane normal and pilot eye.
Check crown height, two physical hinge roots, full pane visibility and aft-sector
reflection, including a small head displacement.

Use the correct seat and handle type. The handle must mount on the seat, clear
the complete stick sweep and remain reachable. Check the boot, trigger, handle
loops and mounting bars, not only centre points. Use the same reference for
in-cockpit hands and external ejection animation. For tandem models, account for
both seats and occupants in the visible geometry and ejection sequence.

## Compact delivery and acceptance

Prefer reusable procedural geometry and a small flat palette. Keep silhouette,
seams, instruments and distinctive hardware before decorative polygons.
Keep editable inputs separate from embedded assets. Lossless `VRZ1` packing
compresses the editable `VRM1` data without reducing texture resolution;
verify byte-for-byte decompression. Do not package Blender/CAD files in releases.
Record vertices, embedded bytes and final binary sizes before and after.
For a procedural additional variant, target no new raster assets and under
30,000 exterior triangles, revisiting the budget only for a concrete feature.

Run relevant Rust and importer checks, and inspect native game captures as well
as Blender renders. Fix geometry or the source transform when a check fails.
A successful render process or finite vertices alone does not establish fidelity.
Export a named review scene with separate fixed, moving, pilot and reference
objects so the result is reproducible.

Before final release captures, freeze source, build Linux and Windows, record
their hashes, then capture both binaries. A later source change requires a new
build and evidence for affected views. Finish with documented references,
measured clearances, remaining approximation limits and the repository's normal
commit/push workflow.
