# Su-27 nozzle and F/A-18C canopy revision, 28 October 2026

The Su-27SK's exposed engine shrouds, nozzle petals and dark exhaust interiors
now use four flat grey materials. The pinned source atlas identifies the bare
shroud islands; only those faces and the named nozzle objects stop sampling it.
Paint, insignia and cockpit textures are preserved. No new raster asset is used.

The Su-27 exterior still contains 8,802 triangles. Positions and normals match
the preceding editable mesh exactly; 1,491 vertices change only colour and UV.
The camouflage QOI is unchanged, SHA-256
`6959e3b9bb5ad66ed1f1034a3a6d7683a069dde0f54147c6427c9fb96ce17793`.
The packed Su-27 mesh shrinks from 7,002,643 to 6,999,266 bytes.

The Hornet's upper bow is a circular arc with a fitted 0.365 m radius. Short
lower posts retain the source sill contacts. Crown and shoulder cases mount
inside the aft lip, with short paired hinges, curved panes and more rounded
outlines. The crown pane no longer stands above the frame. Pane normals and
reflection UVs retain the physical eye and pane geometry.

Visual reference: [Navy photograph 050705-D-D0439-7026](https://www.dvidshub.net/image/9214059/plane-captain-cleans-canopy-f-18c-hornet),
showing an open legacy F/A-18C canopy. The original FlightGear envelope and
the previously documented OpenHornet CAD retain the scale context. The bow
radius, case dimensions and mounting angles are visual estimates. The photo
is used for inspection and is not embedded or redistributed with the game.

Regression checks cover the circular section, symmetry, original sill
contacts, rim seating, both hinge roots, all pane reflection bounds and
visibility from the design eye with 15 mm lateral and 10 mm vertical movement.
Importer checks preserve source hashes, source cockpit pixels, camouflage
styling and exact VRZ1 decompression to the editable VRM1 mesh.

Review evidence is generated under `target/model-review/texture-bow/`:
native captures, exact cockpit exports, the named Blender scene with front,
side, top, pilot, bow and cutaway views, and asset verification JSON. The
native captures verify the actual colours and reflections; Blender verifies
the assembled geometry. Final binaries, hashes and combined multiplayer
verification are recorded in `docs/VALIDATION.md`.

The checkout initially failed to compile because an interrupted prior change
had added Hornet variant metadata without completing exhaustive matches and
ejection cache sizes. Those compilation errors were repaired while preserving
the existing work. This revision makes no fidelity claim for those unfinished
Super Hornet and Growler models.
