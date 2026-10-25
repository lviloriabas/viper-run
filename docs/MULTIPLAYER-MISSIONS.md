# Multiplayer mission rules

Operation is a deterministic host-side objective engine. It does not move,
spawn, damage or render aircraft. The multiplayer host applies directives to
real combat actors and supplies current observations from that simulation.
Clients display the host's outcome, objective summary and scoreboard.

## Call order

1. Create Operation::new(mode, duration_seconds), and register roster actors.
2. Combat, Strike and Escort use the host's initial scene. After those actors,
   primary targets and the escort really exist, call wave_spawned(1).
3. Defense emits SpawnWave { wave, fighters } from update. Create every actor
   in that directive, then call wave_spawned(wave). A wrong or repeated
   acknowledgement returns false.
4. Process authoritative kills and deaths. If every human pilot has been lost
   in a cooperative mission, call fail(Reason::AllPilotsLost) before update.
5. Call update(dt, Observation) with live enemy counts, primary target counts,
   escort state and measured route progress, base health and cumulative breaches.
6. Apply directives once. Completed(outcome) is emitted once; outcome() remains
   available afterwards. summary() returns at most 160 ASCII bytes for the HUD
   and replication.

Without the initial acknowledgement, an empty observation cannot grant a
cooperative victory or accidentally declare a missing escort dead. While a
Defense wave awaits acknowledgement, zero enemies cannot clear it. Its timer
continues running, so a failed spawn is never treated as a completed objective.

Observation::default() has full base health and otherwise zero/false values.
Progress and health are clamped to 0..1; nonfinite progress becomes zero and
nonfinite health becomes zero. Breaches are cumulative and cannot decrease.
Nonfinite or negative dt is ignored. The duration is 1 to 7200 seconds, with
900 seconds used for a nonfinite input. Pausing is represented by dt = 0.

## Rules by mode

| Mode | Objective | Timeout |
| --- | --- | --- |
| Dogfight | First pilot to five credited enemy kills | Unique highest scorer wins; ties or no kills draw |
| Teams | First team to ten credited enemy kills | Higher team score wins; ties draw |
| Combat | All hostile aircraft destroyed | Defeat |
| Strike | All primary ground targets destroyed; optional hostiles may remain | Defeat |
| Escort | A living allied aircraft reaches measured coast progress 1.0 | Defeat |
| Defense | Clear three acknowledged waves while protecting the base | Defeat |

Defense spawns four, six and eight fighters. A cleared wave starts an eight
second intermission before the next directive. The HUD distinguishes preparation,
deployment, active waves and the intermission countdown. Three cumulative
breaches or base health at zero end the mission immediately.

Escort progress must come from the actual allied aircraft's position along its
route toward the coast. Advancing time alone does not advance the escort.
Losing the escort fails even if it reaches the destination in the same frame.
Base destruction and breaches likewise take priority over clearing a wave.
Completing an objective exactly at the time limit wins before timeout is applied.
All outcomes are terminal.

Cooperative human pilots have one life, as enforced by the host. A dead pilot
may spectate until the operation ends. PVP respawn timing and actor creation
remain host responsibilities, not objective-engine timers.

## Score and death identity

Actor contains id: u64, team: u8 and life: u32. Human IDs come from the session
roster; AI IDs may use u64::MAX - index. Increment life whenever the same ID is
reused for a respawn or a later wave. Replaying a death for the same life, or
sending an older life after a respawn, cannot add another death or kill.

record_kill(killer, victim) returns true when it records a new death. Suicides
and friendly fire record the victim's death but award no kill. In Dogfight,
every different pilot is an opponent regardless of the team field. In Teams,
only team 0 and team 1 are accepted. Team identity cannot change mid-operation.
record_death(victim) records uncredited events such as a crash or disconnect.
Do not call it before record_kill for the same credited event.

A projectile may credit its owner after the owner has respawned; the victim's
life still provides deduplication. All calls must come from authoritative host
simulation events, not from a client's claimed score. Scores and team scores
are available through read-only maps. The engine stores at most 64 identities,
one current-life marker and one last-death marker each; event history does not
grow with the number of respawns.

## Verification

Pure tests cover scene and wave acknowledgements, all three waves, intermissions,
Escort arrival and loss, primary-target-only Strike completion, defense breaches,
base damage, timeout winners and ties, suicide/friendly fire, duplicate deaths,
old-life projectiles, life counter wrap, actor churn, finite timing and bounded
ASCII HUD summaries. They require neither an audio device nor a network peer.

Run after integration with:

    cargo test operation::tests

The module can also be compiled in an isolated rustc harness that supplies only
the GameMode enum. Simulation integration must separately verify that spawns,
movement, combat damage and observations refer to real actors.
