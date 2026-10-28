# Hornet and Flanker geometry adaptation

Revised 27 October 2026. Actual editable FlightGear meshes replace the earlier
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

The Su-27SK contributes detailed side-console meshes, switches and canopy
frames. The main front board, optical projector and coaming are rebuilt as
one game assembly, preserving the source combiner-glass coordinates. Obsolete
front boards, the source ground plane and the unfinished sight diagram are
removed from the interior import. The original front wall is opened for the
pedal tunnels; a lower continuous floor closes the aircraft beneath them.

The front instruments share an orthonormal mounting frame, including the
depth of the raised screen cases. Full mounting flanges, rather than only
round faces, are included in spacing checks. Every housing has solid backing.
The Su-27's rounded projector cover, lens, combiner rails, top hood, grab handle,
brightness/reticle controls and upper-left sighting panel have separate 3D
geometry. Its throttle pair uses an outboard quadrant; source console panels
are cut around that quadrant with their UVs preserved. Gear, flaps, speedbrake,
sensor and weapon selectors retain their keyboard/controller commands.

The Hornet retains the requested central UFCD, side DDIs, lower MPCD, left
engine/fuel display and right standby instruments. A shallow optical base and
transverse header replace the oversized block beneath the HUD. Slotted side
channels, diagonal feet, fastening hardware and a separate optical rim detail
the HUD structure. The UFCD case
joins the board as a solid housing. The engine/fuel module and auxiliary gauges
clear the consoles and knee tunnels. Main switches and the throttle quadrant
are fixed to their console surfaces.

Both cockpits have a horizontal crown mirror and two tall side mirrors, with
live reflections, following the supplied references. The older FlightGear
Hornet exterior does not provide a detailed production mirror assembly.
Its simplified cockpit geometry alone is insufficient to establish those
details; the supplied Hornet photographs and instrument references are used
for the interior adaptation.

The Hornet shoulder panes follow the tangent of the mating bow, following
the same arrangement as the MiG mirrors.
The mounting layout follows the MiG-29: one crown pane and two shoulder panes
seated against the pilot-facing canopy joint. The Hornet's forward bow replaces
the coarse triangular source cap with tapered posts, distinct shoulders and a
shallow crown. Its contiguous mating lip carries the mirror cases; the original
sill and windshield rails still meet its feet. The Flanker's crown case rests
directly against its imported bow, with short hinges instead of the elevated
stalk. Shoulder hinges attach to the rear face of that same joint.

The Su-27's thin HUD upper lip clears the lower crown rim without changing
the original combiner glass. Both ends of the left HUD lead enter the solid
projector housing through collars, closing the previously detached loop.
The rounded cover joins the optical head, control case and foundation without
leaving a gap beneath the lens.
Blender review includes focused bow and HUD-lead views as well as the complete
cockpit and leg wells.

Hornet/Flanker pane normals bisect the design-eye ray and the intended aft
view. Each vertex samples the reflected ray, rather than fitting a cropped
rear panorama independently to each pane. A dedicated 1536x480 texture is
sampled directly in the final cockpit pass, preserving depth occlusion and
avoiding the previous 512x160 atlas copy. Optical bulge is small enough to
avoid the exaggerated curved horizon. The MiG keeps its existing panorama
placement and also uses the sharper texture. A common rear camera remains
a performance approximation, so near-field parallax between mirror positions
is not simulated independently.

The Su-27 and MiG-29 use a round KI-13-style compass bowl with a 3D spherical
card, fixed lubber line, hood and bracket. The markings wrap around the sphere
without changing their pixel proportions. The Hornet has its own compact
housing with a horizontal reading window and curved heading scale. It does
not reuse the Russian bowl. NATOPS A1-F18AC-NFM-000 section 2.12.9 and the
[Eagle Dynamics Hornet manual](https://www.digitalcombatsimulator.com/upload/iblock/8d7/2s3e89jqknz7xmti8hrhe1bjt2uw1s3e/DCS%20FA-18C%20Early%20Access%20Guide%20EN.pdf)
identify its standby magnetic compass on the right windshield arch.
The adapted scale follows the game's heading reference; magnetic variation,
dip and acceleration errors are not simulated.

The pilot's twin-engine-aircraft leg pose and shorter pedal travel clear the
front boards throughout rudder movement. The Su-27 seat and design eye are
12 cm aft of the original placement; its centre stick is 18 cm aft. The pilot,
hands, pedals, exterior crew and ejection origin share that seating adjustment.
The fixed panel and console datum remains unchanged. Full stick-travel tests
include both procedural and imported textured hardware. Pedals have floor supports. The
MiG-29 and F-16 retain their existing leg poses. Continuous sill seals, cheek
returns, floor and bulkheads close the interior around these assemblies.

Live faces preserve pixel aspect ratio. Source console panels use a cleaned
64-colour palette, while exterior paint uses 48 colours with reserved colours
for insignia, numbers and warnings. Source aspect, UVs and images up to 2048
pixels are preserved. Original sources remain unchanged; QOI packages the styled
pixels losslessly. Instrument faces and compass markings are drawn
sharply with hardware and pilot depth occlusion.

Flight instruments, radar, HUD, weapon state, gear/flap/brake indicators and
mirrors remain live. The game retains its avionics and flight model; it does
not run FlightGear's cockpit scripts or implement every original switch.

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
full flange separation, parallel mounting frames, housing backing, pixel aspect
and mirror geometry. Lower perimeter rays cover 47 stations on both sides,
four heights and three head positions. Leg-mesh edges are checked against
imported and generated hardware at full left, neutral and full right rudder.
Compass checks cover card geometry, heading and dynamic vertex-buffer capacity.
Type-specific ejection handles and the complete pilot sequence are described in
[PILOT-HARNESS.md](PILOT-HARNESS.md).

Python checks cover nested object matrices, concave outlines, UV-preserving cuts,
lossless image coding, source hashes and pixel-for-pixel preservation of every
embedded cockpit image. Bundled sources can be restored and regenerated offline.

Linux Blender exterior review scenes show top, side, head-on and three-quarter
views. To inspect the assembled cockpit, including source consoles and the
moving pilot, use:

```sh
cargo run -- --export-cockpit target/model-review/cockpit
blender --background -t 4 --python tools/review_cockpit.py -- target/model-review/cockpit
```

These workbench scenes focus on geometry and include a side cutaway of the
leg wells. Native game captures verify the live textures and cockpit sightlines.
The release is inspected inside the game on Linux and Windows, including
forward view, panel, consoles, stores, gear, rudders and speedbrakes.
See [VALIDATION.md](VALIDATION.md) for final release checks.

These adapted simulation assets are calibrated to public references. They are
not manufacturer CAD or surveyed production geometry.
