# Air-to-air missiles: locks, launch and guidance

Revised 23 October 2026. Which missiles of the F-16C Block 50 and the
MiG-29S (9.13S) can be fired without a lock, which need the target acquired
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

| Family | F-16C Block 50 | MiG-29S 9.13S | Before launch | After launch |
| --- | --- | --- | --- | --- |
| Infrared, short range (Fox 2) | AIM-9M | R-60M, R-73 | LOBL: seeker tone, on the boresight, slaved to the radar or cued by the helmet | Fire and forget; the jet is free to manoeuvre |
| Infrared, short range (Fox 2) | AIM-9X Block II | none | LOBL with a tone, or LOAL with no tone on the radar lock's cue | Datalink until it acquires; searches on its own with no cue |
| Infrared, medium range (Fox 2) | none | R-27T | LOBL: the seeker must see the target, cued by the radar or the IRST | Fire and forget; no radar warning for the target |
| Semi-active radar (Fox 1) | AIM-7M | R-27R | Radar lock (STT) required | The radar must hold the lock to impact; if it drops, the missile goes blind |
| Active radar (Fox 3) | AIM-120C, AIM-120D | R-77 | With a lock: inertial and datalink. Without: active off the rail (maddog) | Datalink while the radar holds the lock; near the target its own radar takes over. If the lock drops far out, it flies on blind and searches when it gets there |

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
  in their 20 degree field. With a lock they follow the datalink; if the lock
  drops with the target beyond 15 km they lose it and search when they get
  there.
- With a datalink a missile can turn toward a target outside its seeker's
  field during its first 8 s (an off-boresight or over-the-shoulder shot);
  without one, a target outside the field is lost.
- The MiG-29S's IRST track cues its heat seekers but guides no radar
  missile; its laser ranges inside 6.5 km. The IRST is passive, and neither
  jet has a missile approach warner, so a bandit sees an infrared missile
  only by its smoke (inside 6 km while the motor burns) or close in (3 km).
- Flares fool only infrared seekers and chaff only radar ones.
- A burst damages the target by where its fragments and blast reach it
  (`src/damage.rs`): a ring of fragments round the missile's axis out to
  its outer radius and a blast inside its lethal radius, so bigger
  warheads (the R-27s' 39 kg against the AIM-9M's 9.4 kg) kill from further.

## Sources

- [Lock-on after launch (Wikipedia)](https://en.wikipedia.org/wiki/Lock-on_after_launch)
- [NAVAIR: AIM-9X Block II, LOAL performance](https://www.navair.navy.mil/node/22731)
- [Flightglobal: AIM-9X Block II performing better than expected](https://www.flightglobal.com/aim-9x-block-ii-performing-better-than-expected/108582.article)
- [R-77 (Wikipedia)](https://en.wikipedia.org/wiki/R-77)
- [Bharat Rakshak: R-77 RVV-AE](https://www.bharat-rakshak.com/IAF/Galleries/Weapons/AAMs/R-77/)
- [f-16.net thread on Block 50 stores and the Sparrow](https://www.f-16.net/forum/viewtopic.php?p=387492)
- [GlobalSecurity: MiG-29 design](https://www.globalsecurity.org/military/world/russia/mig-29-design.htm)
