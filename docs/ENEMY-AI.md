# Enemy pilots and missile defence

The flight uses the same six-degree-of-freedom aircraft, fuel, stores mass,
drag, flight controls, structural limits and missile physics as the player.
There are no position changes, extra thrust or immunity during a manoeuvre.

## Coordination

One pilot attacks while the others cover from separated lateral and vertical
positions. Assignments favour nose position, range, energy and airframe condition.
They persist for seven seconds unless the attacker is threatened, too slow,
damaged or dead. A relieved pilot unloads and can drag the pursuer toward the
covering fighter. An unarmed or badly engine-damaged jet disengages.

Pilots predict nearby friendly closest approach and block fire when a friendly
crosses the firing lane. The flight spaces launches by at least four seconds
and keeps at most two tracking air-to-air missiles inbound. A supporting
launcher does not immediately fire another missile.

Radar and visual observations are shared. When every pilot loses sight of the
player, the flight extrapolates the last observed track for eight seconds and
then searches its last position. Terrain masking can therefore break both
weapon support and the flight's live picture.

## Weapons

Each enemy has six missiles on actual release stations, with the resulting
weight and drag. Leads carry short-range infrared missiles and active radar
missiles. Cover jets also carry semi-active radar missiles:

| Airframe | Lead | Cover |
| --- | --- | --- |
| F-16C | Four AIM-9M, two AIM-120C | Four AIM-9M, two AIM-7M |
| MiG-29S | Four R-73, two R-77 | Two R-73, two R-77, two R-27R |

Selection checks nose geometry, remaining stores, sensor support and a missile
fly-out. IR is preferred close in, radar weapons at longer ranges. Radar tracks
take time to acquire and require a living launcher, tracking cone and line of
sight. Semi-active missiles need illumination throughout flight; active missiles
can continue autonomously once close enough to acquire. Stations disappear
individually after release, and their mass and drag are removed.

Gun tracking includes the target's angular motion and ballistic lead. Bursts
are bounded, have a reassessment pause, and stop immediately if aim, terrain
clearance or friendly separation makes the shot unsafe.

## Energy and terrain

Corner speed follows air density and gross weight. Below the useful energy
band a pilot eases the pull, lowers the nose when clearance permits, accelerates
and waits until speed has recovered before turning hard again. It can use
lag pursuit and the speedbrake to prevent a close overshoot. Fuel reserves and
damage also affect its decisions.

Terrain recovery runs every physics step and overrides combat. Missile defence
can choose a safe descending corridor that puts a ridge across the missile's
line of sight; it does not aim at the terrain itself.

## Defeating missiles

Missiles retain finite motor burn, aerodynamic turn authority, induced drag,
seeker field of view, arming range, proximity fuze and lifetime. Guidance does
not imply an inevitable hit.

- Turn away early from a distant launch to make it spend its energy in pursuit.
- A sustained beam in low-altitude look-down geometry can merge a radar target
  with clutter. This simplified Doppler model needs time; better ECCM requires
  a tighter and longer notch. IR seekers are unaffected by radar notching.
- Break the missile's line of sight with terrain while maintaining clearance.
- Combine crossing manoeuvres with countermeasures. Chaff affects radar seekers
  and flares affect IR seekers. Their success remains probabilistic and depends
  on geometry, range, afterburner and the missile's counter-countermeasures.
- For IR defence, reduce afterburner to make the flare more competitive. The
  engine takes time to cool, and a close high-energy missile can still hit.

Enemy pilots select detected threats by closing time, with no heat-seeker
warning outside visual detection range. They dispense finite programs when a
closing missile is near, avoid wasting them on receding missiles, cool the tail
against IR threats, and combine beam, drag and terminal crossing manoeuvres.
Free-flight patrols continue to hold fire and countermeasures.

## Verification

Run `cargo test --release`. Tests cover role relief, firing lanes, mixed
loads and their physics, shot spacing, radar support loss, track memory,
energy recovery in both airframes, continuous gun solutions, countermeasure
timing, terrain masking, radar notching and finite missile envelopes. Paired
launch simulations compare an undefended target with an escaping target,
including protected F-16 and MiG-29 flight physics without decoys or immunity.
