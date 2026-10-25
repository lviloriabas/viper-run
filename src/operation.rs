//! Host-authoritative multiplayer objectives. No physics, rendering or networking.
//! Initial scenes and defense waves require an explicit spawn acknowledgement.
use crate::net::GameMode;
use std::collections::BTreeMap;

const MAX_ACTORS: usize = 64;
const DEFENSE_WAVES: u8 = 3;
const BREACH_LIMIT: u32 = 3;
const WAVE_DELAY: f64 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Actor {
    pub id: u64,
    pub team: u8,
    /// Increment when this actor respawns. Death events are idempotent per life.
    pub life: u32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Observation {
    pub enemies_alive: usize,
    pub primary_targets_remaining: usize,
    pub escort_alive: bool,
    /// Measured flight progress toward the coast, never a timer-based shortcut.
    pub escort_progress: f32,
    pub base_health: f32,
    /// Cumulative across all defense waves.
    pub breaches: u32,
}
impl Default for Observation {
    fn default() -> Self {
        Self {
            enemies_alive: 0,
            primary_targets_remaining: 0,
            escort_alive: false,
            escort_progress: 0.0,
            base_health: 1.0,
            breaches: 0,
        }
    }
}
impl Observation {
    fn sanitise(mut self, previous: Self) -> Self {
        self.escort_progress = if self.escort_progress.is_finite() {
            self.escort_progress.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.base_health = if self.base_health.is_finite() {
            self.base_health.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.breaches = self.breaches.max(previous.breaches);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    TimeExpired,
    EscortLost,
    BaseDestroyed,
    TooManyBreaches,
    AllPilotsLost,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Victory {
        team: Option<u8>,
        actor: Option<u64>,
    },
    Defeat(Reason),
    Draw,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Directive {
    SpawnWave { wave: u8, fighters: usize },
    Completed(Outcome),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Score {
    pub team: u8,
    pub kills: u32,
    pub deaths: u32,
    life: u32,
    last_death: Option<u32>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TeamScore {
    pub kills: u32,
    pub deaths: u32,
}

pub struct Operation {
    mode: GameMode,
    duration: f64,
    elapsed: f64,
    observation: Observation,
    ready: bool,
    wave: u8,
    awaiting_wave: bool,
    next_wave_at: Option<f64>,
    scores: BTreeMap<u64, Score>,
    teams: BTreeMap<u8, TeamScore>,
    outcome: Option<Outcome>,
    completion_pending: bool,
}
fn newer(a: u32, b: u32) -> bool {
    (a.wrapping_sub(b) as i32) > 0
}
impl Operation {
    pub fn new(mode: GameMode, duration_seconds: f64) -> Self {
        let duration = if duration_seconds.is_finite() {
            duration_seconds.clamp(1.0, 7200.0)
        } else {
            900.0
        };
        Self {
            mode,
            duration,
            elapsed: 0.0,
            observation: Observation::default(),
            ready: false,
            wave: 0,
            awaiting_wave: false,
            next_wave_at: None,
            scores: BTreeMap::new(),
            teams: BTreeMap::new(),
            outcome: None,
            completion_pending: false,
        }
    }
    pub fn mode(&self) -> GameMode {
        self.mode
    }
    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }
    pub fn remaining(&self) -> f64 {
        (self.duration - self.elapsed).max(0.0)
    }
    pub fn wave(&self) -> u8 {
        self.wave
    }
    pub fn scores(&self) -> &BTreeMap<u64, Score> {
        &self.scores
    }
    pub fn team_scores(&self) -> &BTreeMap<u8, TeamScore> {
        &self.teams
    }

    fn valid_identity(&self, actor: Actor) -> bool {
        if self.mode == GameMode::Teams && actor.team > 1 {
            return false;
        }
        self.scores
            .get(&actor.id)
            .is_none_or(|score| score.team == actor.team)
    }
    /// Register the roster before play. AI IDs may use u64::MAX - index.
    /// Team identity is immutable for the duration of an operation.
    pub fn register_actor(&mut self, actor: Actor) -> bool {
        if self.outcome.is_some() || !self.valid_identity(actor) {
            return false;
        }
        if let Some(score) = self.scores.get_mut(&actor.id) {
            if score.life != actor.life && !newer(actor.life, score.life) {
                return false;
            }
            score.life = actor.life;
            return true;
        }
        if self.scores.len() >= MAX_ACTORS {
            return false;
        }
        self.scores.insert(
            actor.id,
            Score {
                team: actor.team,
                kills: 0,
                deaths: 0,
                life: actor.life,
                last_death: None,
            },
        );
        self.teams.entry(actor.team).or_default();
        true
    }
    /// Returns true for the first death event for this actor/life.
    pub fn record_death(&mut self, victim: Actor) -> bool {
        if !self.register_actor(victim) {
            return false;
        }
        let score = self.scores.get_mut(&victim.id).unwrap();
        if score
            .last_death
            .is_some_and(|last| last == victim.life || !newer(victim.life, last))
        {
            return false;
        }
        score.last_death = Some(victim.life);
        score.deaths = score.deaths.saturating_add(1);
        let team = self.teams.entry(victim.team).or_default();
        team.deaths = team.deaths.saturating_add(1);
        true
    }
    /// Returns true when a new death was recorded. Suicides and friendly fire
    /// still record the death but never award a kill. A projectile may credit
    /// its owner after that owner's life has advanced.
    pub fn record_kill(&mut self, killer: Actor, victim: Actor) -> bool {
        if self.outcome.is_some()
            || !self.valid_identity(killer)
            || !self.valid_identity(victim)
            || killer.id == victim.id && killer.team != victim.team
        {
            return false;
        }
        if let Some(score) = self.scores.get(&victim.id) {
            if score.life != victim.life && !newer(victim.life, score.life)
                || score
                    .last_death
                    .is_some_and(|last| last == victim.life || !newer(victim.life, last))
            {
                return false;
            }
        }
        let required = usize::from(!self.scores.contains_key(&killer.id))
            + usize::from(killer.id != victim.id && !self.scores.contains_key(&victim.id));
        if self.scores.len() + required > MAX_ACTORS {
            return false;
        }
        if !self.scores.contains_key(&killer.id) && !self.register_actor(killer) {
            return false;
        }
        if !self.record_death(victim) {
            return false;
        }
        if killer.id == victim.id || self.mode != GameMode::Dogfight && killer.team == victim.team {
            return true;
        }
        let score = self.scores.get_mut(&killer.id).unwrap();
        score.kills = score.kills.saturating_add(1);
        let kills = score.kills;
        let team = self.teams.entry(killer.team).or_default();
        team.kills = team.kills.saturating_add(1);
        let team_kills = team.kills;
        if self.mode == GameMode::Dogfight && kills >= 5 {
            self.finish(Outcome::Victory {
                actor: Some(killer.id),
                team: None,
            });
        } else if self.mode == GameMode::Teams && team_kills >= 10 {
            self.finish(Outcome::Victory {
                actor: None,
                team: Some(killer.team),
            });
        }
        true
    }
    /// Combat/Strike/Escort: call wave_spawned(1) after initial actors and
    /// primary targets really exist. Defense: acknowledge each SpawnWave only
    /// after applying it. Repeated/wrong acknowledgements are rejected.
    pub fn wave_spawned(&mut self, wave: u8) -> bool {
        if self.outcome.is_some() || self.mode.pvp() {
            return false;
        }
        if self.mode == GameMode::Defense {
            if !self.awaiting_wave || self.wave != wave {
                return false;
            }
            self.awaiting_wave = false;
            self.ready = true;
        } else {
            if self.ready || wave != 1 {
                return false;
            }
            self.ready = true;
            self.wave = 1;
        }
        true
    }
    pub fn fail(&mut self, reason: Reason) {
        if self.outcome.is_none() {
            self.finish(Outcome::Defeat(reason));
        }
    }
    fn finish(&mut self, outcome: Outcome) {
        if self.outcome.is_none() {
            self.outcome = Some(outcome);
            self.completion_pending = true;
        }
    }
    fn victory(&mut self) {
        self.finish(Outcome::Victory {
            team: Some(0),
            actor: None,
        });
    }
    fn timeout(&mut self) {
        let outcome = match self.mode {
            GameMode::Dogfight => {
                let best = self.scores.values().map(|s| s.kills).max().unwrap_or(0);
                let winners: Vec<_> = self
                    .scores
                    .iter()
                    .filter(|(_, s)| s.kills == best && best > 0)
                    .collect();
                if winners.len() == 1 {
                    Outcome::Victory {
                        actor: Some(*winners[0].0),
                        team: None,
                    }
                } else {
                    Outcome::Draw
                }
            }
            GameMode::Teams => {
                let blue = self.teams.get(&0).map_or(0, |s| s.kills);
                let red = self.teams.get(&1).map_or(0, |s| s.kills);
                if blue == red {
                    Outcome::Draw
                } else {
                    Outcome::Victory {
                        actor: None,
                        team: Some(u8::from(red > blue)),
                    }
                }
            }
            _ => Outcome::Defeat(Reason::TimeExpired),
        };
        self.finish(outcome);
    }
    fn spawn_wave(&mut self, directives: &mut Vec<Directive>) {
        self.wave += 1;
        self.awaiting_wave = true;
        self.ready = false;
        self.next_wave_at = None;
        directives.push(Directive::SpawnWave {
            wave: self.wave,
            fighters: 2 + self.wave as usize * 2,
        });
    }
    /// Only the authoritative host calls update and applies directives.
    /// Failures beat objective completion in the same observation; completing
    /// an objective exactly on the deadline wins before the timeout is applied.
    pub fn update(&mut self, dt: f64, observation: Observation) -> Vec<Directive> {
        let mut directives = Vec::new();
        if self.outcome.is_none() {
            if dt.is_finite() && dt > 0.0 {
                self.elapsed = (self.elapsed + dt).min(self.duration);
            }
            self.observation = observation.sanitise(self.observation);
            let state = self.observation;
            if self.mode == GameMode::Defense && state.base_health <= 0.0 {
                self.fail(Reason::BaseDestroyed);
            } else if self.mode == GameMode::Defense && state.breaches >= BREACH_LIMIT {
                self.fail(Reason::TooManyBreaches);
            } else if self.mode == GameMode::Escort && self.ready && !state.escort_alive {
                self.fail(Reason::EscortLost);
            }
            if self.outcome.is_none() && self.ready {
                match self.mode {
                    GameMode::Combat if state.enemies_alive == 0 => self.victory(),
                    GameMode::Strike if state.primary_targets_remaining == 0 => self.victory(),
                    GameMode::Escort if state.escort_alive && state.escort_progress >= 1.0 => {
                        self.victory()
                    }
                    GameMode::Defense
                        if !self.awaiting_wave
                            && self.next_wave_at.is_none()
                            && state.enemies_alive == 0 =>
                    {
                        if self.wave == DEFENSE_WAVES {
                            self.victory();
                        } else {
                            self.next_wave_at = Some(self.elapsed + WAVE_DELAY);
                        }
                    }
                    _ => {}
                }
            }
            if self.outcome.is_none() && self.elapsed >= self.duration {
                self.timeout();
            }
            if self.outcome.is_none()
                && self.mode == GameMode::Defense
                && !self.awaiting_wave
                && (self.wave == 0 || self.next_wave_at.is_some_and(|at| self.elapsed >= at))
            {
                self.spawn_wave(&mut directives);
            }
        }
        if self.completion_pending {
            directives.push(Directive::Completed(self.outcome.unwrap()));
            self.completion_pending = false;
        }
        directives
    }
    /// HUD/replication text, ASCII and no more than 160 bytes.
    pub fn summary(&self) -> String {
        let mut text = match self.outcome {
            Some(Outcome::Victory {
                actor: Some(id), ..
            }) => format!("VICTORY, PILOT {id}"),
            Some(Outcome::Victory {
                team: Some(team), ..
            }) => format!("VICTORY, TEAM {}", team as u16 + 1),
            Some(Outcome::Victory { .. }) => "VICTORY".into(),
            Some(Outcome::Draw) => "DRAW".into(),
            Some(Outcome::Defeat(reason)) => format!(
                "MISSION FAILED, {}",
                match reason {
                    Reason::TimeExpired => "TIME EXPIRED",
                    Reason::EscortLost => "ESCORT LOST",
                    Reason::BaseDestroyed => "BASE DESTROYED",
                    Reason::TooManyBreaches => "DEFENSE BREACHED",
                    Reason::AllPilotsLost => "ALL PILOTS LOST",
                }
            ),
            None => {
                let time = self.remaining().ceil() as u32;
                let state = self.observation;
                match self.mode {
                    GameMode::Dogfight => format!("DOGFIGHT, lead {}/5 kills, {time}s",
                        self.scores.values().map(|s| s.kills).max().unwrap_or(0)),
                    GameMode::Teams => format!("TEAMS, Blue {} : Red {}, goal 10, {time}s",
                        self.teams.get(&0).map_or(0, |s| s.kills), self.teams.get(&1).map_or(0, |s| s.kills)),
                    GameMode::Combat if !self.ready => format!("COMBAT, preparing encounter, {time}s"),
                    GameMode::Combat => format!("COMBAT, {} hostiles remaining, {time}s", state.enemies_alive),
                    GameMode::Strike if !self.ready => format!("STRIKE, preparing objectives, {time}s"),
                    GameMode::Strike => format!("STRIKE, {} primary targets remaining, {time}s", state.primary_targets_remaining),
                    GameMode::Escort if !self.ready => format!("ESCORT, preparing ally, {time}s"),
                    GameMode::Escort => format!("ESCORT, coast progress {}%, {time}s", (state.escort_progress * 100.0).round() as u32),
                    GameMode::Defense if self.wave == 0 => format!("DEFENSE, preparing first wave, {time}s"),
                    GameMode::Defense if self.awaiting_wave => format!("DEFENSE, deploying wave {}/{DEFENSE_WAVES}, {time}s", self.wave),
                    GameMode::Defense if self.next_wave_at.is_some() => format!("DEFENSE, wave {} clear, next wave in {}s, base {}%, {time}s",
                        self.wave, (self.next_wave_at.unwrap() - self.elapsed).max(0.0).ceil() as u32,
                        (state.base_health * 100.0).round() as u32),
                    GameMode::Defense => format!("DEFENSE, wave {}/{DEFENSE_WAVES}, {} hostiles, base {}%, breaches {}/{BREACH_LIMIT}, {time}s",
                        self.wave, state.enemies_alive, (state.base_health * 100.0).round() as u32, state.breaches),
                }
            }
        };
        text.truncate(160);
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn actor(id: u64, team: u8, life: u32) -> Actor {
        Actor { id, team, life }
    }
    fn success() -> Outcome {
        Outcome::Victory {
            team: Some(0),
            actor: None,
        }
    }
    #[test]
    fn combat_requires_scene_acknowledgement_and_every_hostile_to_be_destroyed() {
        let mut op = Operation::new(GameMode::Combat, 60.0);
        assert!(op.update(0.0, Observation::default()).is_empty());
        assert_eq!(op.outcome(), None);
        assert!(!op.wave_spawned(0));
        assert!(op.wave_spawned(1));
        assert!(!op.wave_spawned(1));
        op.update(
            1.0,
            Observation {
                enemies_alive: 1,
                ..Default::default()
            },
        );
        assert_eq!(op.outcome(), None);
        assert_eq!(
            op.update(1.0, Observation::default()),
            vec![Directive::Completed(success())]
        );
        assert!(op.update(1.0, Observation::default()).is_empty());
        assert!(!op.record_death(actor(1, 0, 0)));
    }
    #[test]
    fn strike_only_counts_primary_targets_not_optional_hostiles() {
        let mut op = Operation::new(GameMode::Strike, 60.0);
        op.wave_spawned(1);
        op.update(
            0.0,
            Observation {
                primary_targets_remaining: 1,
                ..Default::default()
            },
        );
        assert_eq!(op.outcome(), None);
        op.update(
            0.0,
            Observation {
                enemies_alive: 8,
                primary_targets_remaining: 0,
                ..Default::default()
            },
        );
        assert_eq!(op.outcome(), Some(success()));
    }
    #[test]
    fn escort_arrival_requires_live_ally_and_measured_progress() {
        let mut op = Operation::new(GameMode::Escort, 600.0);
        op.update(0.0, Observation::default());
        assert_eq!(op.outcome(), None);
        op.wave_spawned(1);
        op.update(
            100.0,
            Observation {
                escort_alive: true,
                escort_progress: 0.9,
                ..Default::default()
            },
        );
        assert_eq!(op.outcome(), None);
        op.update(
            0.0,
            Observation {
                escort_alive: false,
                escort_progress: 1.0,
                ..Default::default()
            },
        );
        assert_eq!(op.outcome(), Some(Outcome::Defeat(Reason::EscortLost)));
        let mut op = Operation::new(GameMode::Escort, 60.0);
        op.wave_spawned(1);
        op.update(
            1.0,
            Observation {
                enemies_alive: 8,
                escort_alive: true,
                escort_progress: 1.0,
                ..Default::default()
            },
        );
        assert_eq!(op.outcome(), Some(success()));
    }
    #[test]
    fn defense_spawns_exactly_three_acknowledged_waves_with_intermission() {
        let mut op = Operation::new(GameMode::Defense, 600.0);
        assert_eq!(
            op.update(0.0, Observation::default()),
            vec![Directive::SpawnWave {
                wave: 1,
                fighters: 4
            }]
        );
        assert!(op.update(10.0, Observation::default()).is_empty());
        assert_eq!(op.outcome(), None);
        assert!(!op.wave_spawned(2));
        for wave in 1..=3 {
            assert!(op.wave_spawned(wave));
            op.update(
                0.0,
                Observation {
                    enemies_alive: 1,
                    ..Default::default()
                },
            );
            let directives = op.update(0.0, Observation::default());
            if wave == 3 {
                assert_eq!(directives, vec![Directive::Completed(success())]);
            } else {
                assert!(directives.is_empty());
                assert!(op.update(4.0, Observation::default()).is_empty());
                assert_eq!(
                    op.update(4.0, Observation::default()),
                    vec![Directive::SpawnWave {
                        wave: wave + 1,
                        fighters: 2 + (wave as usize + 1) * 2
                    }]
                );
                assert!(op.update(0.0, Observation::default()).is_empty());
                assert_eq!(op.outcome(), None);
            }
        }
        assert_eq!(op.wave(), 3);
        assert!(!op.wave_spawned(4));
        assert!(op.update(20.0, Observation::default()).is_empty());
    }
    #[test]
    fn defense_base_damage_breaches_and_all_human_losses_are_failures() {
        for (observation, reason) in [
            (
                Observation {
                    base_health: 0.0,
                    ..Default::default()
                },
                Reason::BaseDestroyed,
            ),
            (
                Observation {
                    breaches: 3,
                    ..Default::default()
                },
                Reason::TooManyBreaches,
            ),
        ] {
            let mut op = Operation::new(GameMode::Defense, 60.0);
            let result = op.update(1.0, observation);
            assert_eq!(result, vec![Directive::Completed(Outcome::Defeat(reason))]);
        }
        let mut op = Operation::new(GameMode::Defense, 60.0);
        op.update(
            0.0,
            Observation {
                breaches: 2,
                ..Default::default()
            },
        );
        op.update(
            0.0,
            Observation {
                breaches: 0,
                ..Default::default()
            },
        );
        assert_eq!(op.observation.breaches, 2);
        op.fail(Reason::AllPilotsLost);
        assert_eq!(op.outcome(), Some(Outcome::Defeat(Reason::AllPilotsLost)));
    }
    #[test]
    fn dogfight_scores_unique_lives_and_finishes_at_five_kills() {
        let mut op = Operation::new(GameMode::Dogfight, 600.0);
        let killer = actor(1, 0, 0);
        for life in 0..5 {
            assert!(op.record_kill(killer, actor(2, 1, life)));
            assert!(!op.record_kill(killer, actor(2, 1, life)));
            if life < 4 {
                assert_eq!(op.outcome(), None);
            }
        }
        assert_eq!(op.scores()[&1].kills, 5);
        assert_eq!(op.scores()[&2].deaths, 5);
        let expected = Outcome::Victory {
            actor: Some(1),
            team: None,
        };
        assert_eq!(
            op.update(0.0, Observation::default()),
            vec![Directive::Completed(expected)]
        );
        assert!(op.update(0.0, Observation::default()).is_empty());
    }
    #[test]
    fn replayed_deaths_cannot_insert_phantom_killers() {
        let mut op = Operation::new(GameMode::Dogfight, 60.0);
        assert!(op.record_death(actor(2, 1, 0)));
        assert!(!op.record_kill(actor(999, 0, 0), actor(2, 1, 0)));
        assert!(!op.scores().contains_key(&999));
    }
    #[test]
    fn teams_count_enemy_kills_but_never_suicides_or_friendly_fire() {
        let mut op = Operation::new(GameMode::Teams, 600.0);
        assert!(op.record_kill(actor(1, 0, 0), actor(1, 0, 0)));
        assert!(op.record_kill(actor(1, 0, 1), actor(3, 0, 0)));
        assert_eq!(op.team_scores()[&0].kills, 0);
        for life in 0..10 {
            assert!(op.record_kill(actor(1, 0, 1), actor(2, 1, life)));
        }
        assert_eq!(op.team_scores()[&0].kills, 10);
        assert_eq!(
            op.outcome(),
            Some(Outcome::Victory {
                team: Some(0),
                actor: None
            })
        );
    }
    #[test]
    fn uncredited_deaths_and_old_life_projectiles_do_not_duplicate_scores() {
        let mut op = Operation::new(GameMode::Dogfight, 60.0);
        assert!(op.record_death(actor(1, 0, 0)));
        assert!(op.register_actor(actor(1, 0, 1)));
        assert!(op.record_kill(actor(1, 0, 0), actor(2, 1, 0)));
        assert_eq!(op.scores()[&1].kills, 1);
        assert_eq!(op.scores()[&1].deaths, 1);
        assert!(!op.record_kill(actor(2, 1, 0), actor(1, 0, 0)));
        assert!(op.record_death(actor(1, 0, 1)));
        assert_eq!(op.scores()[&1].deaths, 2);
    }
    #[test]
    fn timeouts_select_pvp_leaders_or_draw_and_fail_unfinished_coop() {
        let mut op = Operation::new(GameMode::Dogfight, 1.0);
        op.record_kill(actor(1, 0, 0), actor(2, 1, 0));
        op.update(1.0, Observation::default());
        assert_eq!(
            op.outcome(),
            Some(Outcome::Victory {
                team: None,
                actor: Some(1)
            })
        );
        let mut op = Operation::new(GameMode::Dogfight, 1.0);
        op.record_kill(actor(1, 0, 0), actor(2, 1, 0));
        op.record_kill(actor(2, 1, 0), actor(1, 0, 0));
        op.update(1.0, Observation::default());
        assert_eq!(op.outcome(), Some(Outcome::Draw));
        let mut op = Operation::new(GameMode::Teams, 1.0);
        op.record_kill(actor(1, 0, 0), actor(2, 1, 0));
        op.update(1.0, Observation::default());
        assert_eq!(
            op.outcome(),
            Some(Outcome::Victory {
                team: Some(0),
                actor: None
            })
        );
        let mut op = Operation::new(GameMode::Teams, 1.0);
        op.update(1.0, Observation::default());
        assert_eq!(op.outcome(), Some(Outcome::Draw));
        for mode in [
            GameMode::Combat,
            GameMode::Strike,
            GameMode::Escort,
            GameMode::Defense,
        ] {
            let mut op = Operation::new(mode, 1.0);
            op.update(1.0, Observation::default());
            assert_eq!(op.outcome(), Some(Outcome::Defeat(Reason::TimeExpired)));
        }
    }
    #[test]
    fn completion_on_deadline_is_terminal_and_failure_wins_over_arrival() {
        let mut op = Operation::new(GameMode::Combat, 1.0);
        op.wave_spawned(1);
        op.update(1.0, Observation::default());
        assert_eq!(op.outcome(), Some(success()));
        op.fail(Reason::AllPilotsLost);
        assert_eq!(op.outcome(), Some(success()));
        let mut op = Operation::new(GameMode::Escort, 1.0);
        op.wave_spawned(1);
        op.update(
            1.0,
            Observation {
                escort_alive: false,
                escort_progress: 1.0,
                ..Default::default()
            },
        );
        assert_eq!(op.outcome(), Some(Outcome::Defeat(Reason::EscortLost)));
    }
    #[test]
    fn malformed_timing_progress_and_actor_churn_do_not_panic_or_grow_state() {
        let mut op = Operation::new(GameMode::Dogfight, f64::NAN);
        op.update(f64::INFINITY, Observation::default());
        op.update(-1.0, Observation::default());
        assert_eq!(op.remaining(), 900.0);
        for id in 0..MAX_ACTORS as u64 {
            assert!(op.register_actor(actor(id, 0, 0)));
        }
        assert!(!op.register_actor(actor(999, 0, 0)));
        assert_eq!(op.scores().len(), MAX_ACTORS);
        assert!(!op.register_actor(actor(0, 1, 0)));
        let mut op = Operation::new(GameMode::Escort, 60.0);
        op.wave_spawned(1);
        op.update(
            f64::NAN,
            Observation {
                escort_alive: true,
                escort_progress: f32::INFINITY,
                ..Default::default()
            },
        );
        assert_eq!(op.outcome(), None);
    }
    #[test]
    fn life_serials_wrap_and_summaries_are_bounded_ascii_for_every_mode() {
        let mut op = Operation::new(GameMode::Dogfight, 60.0);
        assert!(op.record_death(actor(u64::MAX, 0, u32::MAX)));
        assert!(op.record_death(actor(u64::MAX, 0, 0)));
        assert!(!op.record_death(actor(u64::MAX, 0, u32::MAX)));
        for mode in [
            GameMode::Dogfight,
            GameMode::Teams,
            GameMode::Combat,
            GameMode::Strike,
            GameMode::Escort,
            GameMode::Defense,
        ] {
            let mut op = Operation::new(mode, 7200.0);
            op.update(
                0.0,
                Observation {
                    enemies_alive: usize::MAX,
                    primary_targets_remaining: usize::MAX,
                    escort_alive: true,
                    escort_progress: 0.75,
                    ..Default::default()
                },
            );
            let summary = op.summary();
            assert!(summary.is_ascii() && summary.len() <= 160);
            op.fail(Reason::AllPilotsLost);
            assert!(op.summary().is_ascii() && op.summary().len() <= 160);
        }
    }
}
