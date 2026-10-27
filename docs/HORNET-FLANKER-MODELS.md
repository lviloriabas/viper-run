# Hornet and Flanker geometry adaptation

Revised 26 October 2026. Actual editable FlightGear meshes replace the earlier
procedural exterior reconstruction. Both aircraft retain Viper Run's faceted
rendering, live avionics and keyboard/controller input.

## Geometry and references

The legacy Hornet uses Fabrice Kauffmann's FGAddon mesh. The Flanker uses the
Su-27SK from xcvb85/Su-27 at commit
`30f1cb45c87d4f2443de198d7577a3a9b6b0455a`. A Su-33 mesh is no longer the basis
of the Flanker. Inputs, original credits, GPL licences and hashes are bundled in
[assets/models](../assets/models/README.md).

Independent dimensional references are the legacy Hornet three-view in
[NASA, A New Twist in Flight Research](https://www.nasa.gov/wp-content/uploads/2015/03/A_New_Twist_in_Flight_Research1.pdf),
[NAVAIR's A-D Hornet reference](https://www.navair.navy.mil/product/FA-18-D-Hornet),
and the general-arrangement and cockpit material in the
[Eagle Dynamics Su-27 manual](https://www.digitalcombatsimulator.com/upload/iblock/ed7/Su-27%20DCS%20Flaming%20Cliffs%20Flight%20Manual%20EN.pdf).
The supplied UFCD and analogue-panel photographs remain cockpit layout references.
They are not redistributed as textures.

| Structural envelope | Legacy Hornet | Su-27SK |
| --- | --- | --- |
| Span, excluding stores | 11.43 m | 14.70 m |
| Length | 17.10 m | 21.90 m |
| Canopy crown above model datum | 1.371 m | 1.638 m |
| Pilot eye above model datum | 1.004 m | 1.369 m |

AC3D axes (aft, up, left) become game axes (right, up, aft). Hornet transverse/
vertical scale is 1.035; longitudinal scale is 17.1/15.39131 with a 0.15 m
datum adjustment. Su-27 transverse/vertical scale is 14.7/15.25; longitudinal
scale is 21.9/22.07678 with a -10.95587 m datum adjustment. Geometry, cockpit,
nozzles, gear and weapons share these transforms.

Source topology supplies radomes, curved LEX, wings, tails, nacelles and canopies.
This restores the Hornet's rounded inlet outlines and removes the inflated
canopy profiles. The baseline Flanker has no canards. Its common upstream gear
is adapted to one 680 x 260 mm nose wheel; extra outboard weapon alternatives
are omitted from the game's ten-station configuration.

Object matrices and parent transforms are retained. Concave polygons are
triangulated without filling their recesses. The Hornet speedbrake samples the
imported spine in both directions, closing within 8 mm of its surface.

## Cockpit

The Su-27SK contributes panel meshes, coaming, individual console panels,
switches, HUD projector housing and canopy frame. Its monitor fits a separate
opening clear of the HUD side wall. A continuous tub follows actual canopy
sills and flares below them to enclose the consoles. Floor, walls and bulkheads
close the gaps previously visible from the seat.

The Hornet retains the requested central UFCD, side DDIs, lower MPCD, left
engine/fuel display and right standby instruments. Panel, central support,
coaming and projector overlap as solid pieces. Old narrow inner walls are
removed from the interior copy of the canopy frame to clear the new board.

Live faces share their mounting frames with the bezels and preserve pixel
aspect ratio. Native Su-27 panel images retain every original pixel. Unfinished
white construction guides on the upstream main panel are replaced with panel
material. Exterior atlases copy source pixels without resampling. QOI packaging
is lossless. Instrument faces are rendered sharply with hardware and pilot depth
occlusion.

Flight instruments, radar, HUD, weapon state, gear/flap/brake indicators and
mirrors remain live. Existing keyboard/controller commands also move the visual
levers. The game retains its avionics and flight model; it does not run
FlightGear's cockpit scripts or implement every original switch.

## Stores, gear and collisions

`Station::position` maps canonical loadout stations onto native hardpoints,
accounts for missile nose origins and retains bomb centre origins. TER and
tandem offsets remain distinct. The R-27's asymmetric source origin is included.
`Station::mounted` adds the same wing deflection for drawing, launch, jettison
and CCIP prediction.

Hornet stores use wingtip rails, two underwing pylon pairs and cheek stations
below the intakes. The Su-27 uses wingtip, outer/inner wing, nacelle and centreline
stations. Fuselage stores remain with the fuselage after wing loss and are
excluded from underwing flutter mass.

Tyre contact points match the simulated wheels. Gear pivots and surface axes
follow the source references; Su-27 gear retracts forward. The single-rigid-part
gear animation approximates the full multi-link mechanism. Damage volumes and
ground contacts follow the visible envelope. Dry braking is calibrated for the
new wheelbase; takeoff, approach and stopping tests remain required.

## Verification

Rust checks cover native dimensions, finite geometry, normals, part sides,
wheel contact, Su-27 nose-wheel dimensions, weapon origins and rack separation.
Cockpit checks test every active face against procedural and imported hardware,
pixel aspect, mirrors and opaque closure below the sills in 72 directions.

Python checks cover nested object matrices, concave outlines, UV-preserving cuts,
lossless image coding, source hashes and pixel-for-pixel preservation of every
embedded cockpit image. Bundled sources can be restored and regenerated offline.

Linux Blender review scenes show top, side, head-on and three-quarter views.
The release is inspected inside the game on Linux and Windows, including
forward view, panel, consoles, stores, gear, rudders and speedbrakes.
See [VALIDATION.md](VALIDATION.md) for final release checks.

These adapted simulation assets are calibrated to public references. They are
not manufacturer CAD or surveyed production geometry.
