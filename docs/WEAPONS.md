# Air-to-air missiles: locks, launch and guidance

Revised 29 October 2026. Which missiles of all six playable airframes
can be fired without a lock, which need the target acquired
before they leave the rail, which can acquire it afterwards, and what the
launching jet has to do while they fly. The code follows these rules in
`weapons.rs` (the families and each missile's data) and `combat.rs`
(`launch`, `sensors`, `step_missiles`).

## Terms

- **LOBL** (lock-on before launch): the missile's seeker must have the target
  before it is fired. With no tone or no lock it leaves unguided.
- **LOAL** (lock-on after launch): the missile can leave without the target
  acquired and find it in flight, with its own seeker or with data from the
  jet.
- **Datalink**: the jet sends midcourse corrections while its radar holds
  the lock, so the missile flies to where the target is, not to where it was
  at launch.
- **Semi-active (SARH)**: the missile homes on the jet's radar energy
  reflected off the target; the jet has to illuminate it (hold the lock)
  until impact.
- **Active (ARH)**: the missile carries its own radar for the terminal
  phase. Fired without a lock (a "maddog" or visual shot) it goes active off
  the rail and takes the first target it sees.

## By family

Every airframe offers Fox 1, Fox 2 and Fox 3 in the loadout selector.
This means available stores; the pilot chooses which families to carry.

| Airframe | Fox 1 | Fox 2 | Fox 3 |
| --- | --- | --- | --- |
| F-16C | AIM-7M | AIM-9M, AIM-9X Block II | AIM-120C, AIM-120D |
| MiG-29S | R-27R | R-60M, R-73, R-27T | R-77 |
| F/A-18C | AIM-7M | AIM-9M | AIM-120C |
| F/A-18E | AIM-7M | AIM-9M | AIM-120C |
| EA-18G | AIM-7M | AIM-9M | AIM-120C |
| Su-27SK (game weapon-system extension) | R-27R | R-73, R-27T | R-77 |

The R-77 is selectable on the Su-27's two inner station groups, including
the fuselage stations that mirror the innermost group. This represents an
upgraded weapon system in the game, not historical R-77 integration on a
stock Su-27SK. The F-16's Sparrow and the shared Hornet-family store lists
also remain gameplay loadout choices rather than certification claims for
every depicted variant or operator. Missile guidance follows its seeker
family regardless of the carrying aircraft.

The EA-18G's permanent ALQ-218 receivers reserve both wingtips: the tip
selector remains EMPTY and even a stale configuration cannot mount missiles
there. Its dogfight preset carries two AIM-9Ms, two AIM-7Ms and two AIM-120Cs
on the three underwing station groups.

| Family | F-16C Block 50 | MiG-29S 9.13S | Before launch | After launch |
| --- | --- | --- | --- | --- |
| Infrared, short range (Fox 2) | AIM-9M | R-60M, R-73 | LOBL: seeker tone, on the boresight, slaved to the radar or cued by the helmet | Fire and forget; the jet is free to manoeuvre |
| Infrared, short range (Fox 2) | AIM-9X Block II | none | LOBL with a tone, or LOAL with no tone on the radar lock's cue | Datalink until it acquires; searches on its own with no cue |
| Infrared, medium range (Fox 2) | none | R-27T | LOBL: the seeker must see the target, cued by the radar or the IRST | Fire and forget; no radar warning for the target |
| Semi-active radar (Fox 1) | AIM-7M | R-27R | Radar lock (STT) required | The radar must hold the lock to impact; if it drops, the missile goes blind |
| Active radar (Fox 3) | AIM-120C, AIM-120D | R-77 | With a lock: inertial and datalink. Without: autonomous radar search (maddog) | Updates while supported; without support, extrapolate the last measurement. The missile's own radar must acquire the target for autonomous homing |

The gaps the table shows (`combat::arsenal`, test `arsenal_by_family`):

- The F-16 has no medium-range infrared missile like the R-27T.
- The MiG-29S has no infrared missile with LOAL or a 90 degree seeker like
  the AIM-9X; its modern counterpart, the R-74M, is not a 9.13S store.

## Missile by missile

- **AIM-9M**: all-aspect infrared seeker, 40 degrees off the nose. LOBL: it
  is cooled and uncaged on the target, slaved to the radar or the helmet, and
  the pilot hears the tone. With no tone it flies ballistic.
- **AIM-9X Block II**: imaging infrared seeker that looks 90 degrees off the
  nose with the JHMCS. Block II added a datalink and LOAL: it can be fired
  before the seeker has the target and find it afterwards by its infrared
  emission or by the jet's data (NAVAIR).
- **R-60M**: infrared dogfight missile with a seeker of about 20 degrees.
  LOBL only.
- **R-73**: all-aspect infrared, 45 degrees off the nose with the Shchel-3UM
  helmet sight. LOBL only; the versions with LOAL (R-74M) came later.
- **R-27T**: the R-27R's body and motor with a passive infrared seeker. It
  must acquire before launch (the radar or the IRST cues it); afterwards it
  needs nothing from the jet and gives no radar warning.
- **AIM-7M Sparrow**: semi-active; inertial midcourse, then guided by the
  jet's radar illumination to impact. Not fired without a lock. The USAF
  never adopted it on its F-16Cs; it was test-fired from the Block 30 and
  export jets carry it on the same stations.
- **R-27R**: semi-active, inertial with radio correction in midcourse. Needs
  the MiG's radar lock from launch to the end.
- **AIM-120C**: inertial with a datalink from the jet's radar, active radar
  at the end. It can be fired at a locked target or, with no lock, active
  off the rail to search on its own.
- **AIM-120D**: the same body with GPS-aided inertial navigation, a two-way
  datalink and more range. The same launch rules.
- **R-77 (RVV-AE)**: inertial with datalink corrections from the N019M radar
  and active radar at the end; at short range it is fired already active.
  Its lattice fins let it manoeuvre at high angles of attack.

## How the game models it

- An infrared missile leaves for what its seeker holds (`Combat::seeker`).
  With no tone the AIM-9M, R-60M, R-73 and R-27T leave unguided; the AIM-9X
  takes the radar lock as its cue (LOAL) or, with nothing, searches within
  8 km in a 20 degree cone.
- The AIM-7M and R-27R are not fired without a lock and go blind as soon as
  the lock changes or drops.
- The AIM-120C/D and R-77 with no lock search with their radar out to 15 km
  in their 20 degree search field. A cued missile can acquire within its
  gimbal limits. With support they receive updated position and velocity;
  without it, they extrapolate only the last measurement. A manoeuvre by an
  unseen target cannot update that estimate. Within 15 km, with clear terrain
  visibility and the target inside the gimbal, the onboard radar acquires
  and no longer needs the launcher's lock. Losing support does not erase
  an existing onboard lock.
- The AIM-9X distinguishes a prelaunch infrared lock from a radar cue.
  A cue supplies midcourse data until its own infrared seeker acquires
  within the game's 8 km acquisition range and gimbal limits. An existing
  infrared lock needs no continuing radar support.
- Datalink/inertial guidance can steer toward an estimated target outside
  the seeker's field; it has no arbitrary eight-second cutoff. Terrain and
  Doppler rejection apply to seeker homing, not to an inertial estimate.
- Player, AI and remote-player missiles use their actual launcher and
  target for support checks. SARH requires a live illuminator tracking the
  same target; an IRST track supplies no radar illumination. A SAM requires
  a live local tracking radar with terrain visibility.
- Destroyed, crashed, structurally broken, ejected or removed aircraft
  invalidate missile tracks and stored estimates. Kill processing clears
  locks immediately so a respawn cannot inherit them. Autonomous seekers
  may subsequently acquire a living enemy through a fresh search. Unguided
  missiles continue flying until ground impact or their lifetime limit.
- The MiG-29S's IRST track cues its heat seekers but guides no radar
  missile; its laser ranges inside 6.5 km. The IRST is passive, and neither
  jet has a missile approach warner, so a bandit sees an infrared missile
  only by its smoke (inside 6 km while the motor burns) or close in (3 km).
- Flares fool only infrared seekers and chaff only radar ones. Flares work
  best against a target seen from ahead or abeam, with its tailpipe hidden,
  and worst against an afterburner seen from behind; chaff works best with
  the target on the beam, where its Doppler matches the chaff cloud's.
- Each seeker then rejects a share of the decoys by its counter-
  countermeasures (`Spec::ccm`; estimates, the real figures are secret):

  | Missile | Seeker | Rejects |
  |---|---|---|
  | AIM-9X | imaging infrared | 85 % |
  | AIM-120D | active radar | 75 % |
  | AIM-120C | active radar | 60 % |
  | AIM-9M | infrared, two-colour IRCCM | 55 % |
  | R-77 | active radar | 50 % |
  | R-73 | cooled infrared | 45 % |
  | AIM-7M | semi-active monopulse | 45 % |
  | R-27T | infrared | 30 % |
  | R-27R | semi-active | 30 % |
  | SA-6 | semi-active | 20 % |
  | R-60M | early cooled infrared | 15 % |
- A burst damages the target by where its fragments and blast reach it
  (`src/damage.rs`): a ring of fragments round the missile's axis out to
  its outer radius and a blast inside its lethal radius, so bigger
  warheads (the R-27s' 39 kg against the AIM-9M's 9.4 kg) kill from further.

## Simplified radar controls and presentation

T designates the return nearest the nose or helmet cue; pressing T again
cycles the available returns. Air weapons select aircraft, while CCIP
selects ground targets. Changing between air and ground modes drops the
previous designation. Undesignate also cancels acquisition.

An air designation takes 0.6 seconds of uninterrupted visibility before it
becomes a track. This delay is a gameplay choice. Until then the display
shows ACQ and the target cannot supply radar missile guidance. Search
(RWS) shows visible returns; a completed radar lock changes to single-target
track (STT), stops the search sweep and displays only the tracked aircraft.
Target range, speed and altitude replace invented callsigns. The upper
notification and aircraft overlay no longer identify targets as BANDIT 1/2.

Detection, designation and tracking share terrain visibility and antenna
limits in aircraft axes. The chosen search field is 60 degrees each side
and 30 degrees above/below; tracking allows 60 degrees in elevation.
Search/track ranges remain the game's 74/80 km. These are simplified game
limits, not a reproduction of classified radar performance. Losing line of
sight, range or antenna coverage drops the track before weapon firing and
missile guidance are evaluated. The infrared seeker also needs terrain
visibility, but can track independently without a radar lock.

The MiG radar uses the same visibility rules and marks acquisition and
tracking on its B-scope. Its IRST stays passive and cannot guide radar
missiles. Semi-active missiles still require radar support to impact;
active missiles retain their own terminal seekers.

The search-to-STT presentation follows the radar workflow described in
[Eagle Dynamics' F-16C manual](https://shared.steamstatic.com/store_item_assets/steam/apps/1090850/manuals/DCS_F-16C_Early_Access_Guide_EN.pdf).
The implementation deliberately omits radar cursor/elevation controls,
Doppler/notch filtering, TWS, jamming and detailed scan-bar timing.

These rules reproduce the public distinction between SARH, independent IR
homing and inertial/datalink plus active terminal homing. They do not claim
an exact seeker simulation: the 15 km radar acquisition range, 8 km LOAL IR
range, search cones and 0.6 s radar acquisition delay are game approximations.
SARH support loss ends homing in this model; detailed missile-specific
inertial coast and reacquisition logic for Sparrow/R-27R is not simulated.
The game also excludes wrecks from acquisition even though real wreckage
can retain heat and radar reflections.

Regression coverage includes every available missile on all six airframes,
IR lock versus radar cue, IRST restrictions, lost illumination, launcher
loss, autonomous terminal acquisition, loss of datalink without access to
unseen target manoeuvres, gimbal/terrain limits and invalid targets before
and after respawn (`src/combat/missile_tests.rs`, `src/weapons.rs`).

## Sources

- [USAF: AIM-7 Sparrow, semi-active CW/pulsed Doppler guidance](https://www.af.mil/About-Us/Fact-Sheets/Display/Article/104575/aim-7-sparrow/)
- [US Navy: AIM-9X, passive infrared acquisition and tracking](https://www.navy.mil/Resources/Fact-Files/Display-FactFiles/Article/2168989/aim-9x-sidewinder-missile/)
- [NAVAIR: AIM-9X Block II, LOAL performance](https://www.navair.navy.mil/node/22731)
- [Raytheon: AMRAAM, inertial guidance, updates and onboard active radar](https://www.rtx.com/raytheon/what-we-do/air/amraam-missile)
- [Rosoboronexport: RVV-AE, radio-updated inertial guidance and active terminal seeker](https://roe.ru/pdfs/pdf_4709.pdf)
- [Rosoboronexport: R-27 family, semi-active and infrared variants](https://roe.ru/pdfs/pdf_4712.pdf)
- [f-16.net thread on Block 50 stores and the Sparrow](https://www.f-16.net/forum/viewtopic.php?p=387492)
- [GlobalSecurity: MiG-29 design](https://www.globalsecurity.org/military/world/russia/mig-29-design.htm)
