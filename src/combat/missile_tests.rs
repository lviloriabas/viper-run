use super::*;
use crate::sim::FIXED_DT;

const MISSILES: [Store; 10] = [Store::Aim9, Store::Aim9x, Store::R60m, Store::R73,
    Store::R27t, Store::Aim7, Store::R27r, Store::Aim120, Store::Aim120d, Store::R77];

fn fight(kind: Type, store: Store) -> (Combat, Aircraft) {
    terrain::init();
    let mut c = Combat::new(MissionKind::Dogfight, &mut Mission::new());
    let mut cfg = Config::for_jet(MissionKind::Dogfight, kind);
    cfg.tips_empty = true;
    cfg.outer = None;
    cfg.outboard = None;
    cfg.inboard = None;
    let pair = (0..4).find(|&p| pylon_choices(kind, p).contains(&Some(store))).unwrap();
    match pair {
        0 => { cfg.tips_empty = false; cfg.tip = store; }
        1 => cfg.outer = Some(store),
        2 => cfg.outboard = Some(store),
        _ => cfg.inboard = Some(store),
    }
    c.equip(&cfg);
    c.select(store.mode());
    c.bandits.truncate(1);
    c.reserve.clear();
    let a = spawn_type(kind, v3(0.0, 6000.0, 0.0), 0.0, 230.0);
    c.bandits[0].jet = spawn(a.pos - V3::Z * 5000.0, 0.0, 200.0);
    (c, a)
}

#[test]
fn all_six_airframes_offer_all_three_fox_families_on_selectable_stations() {
    for kind in Type::ALL {
        for (family, seeker) in [(Family::SemiActive, Seeker::SemiActive),
            (Family::ShortIr, Seeker::Infrared), (Family::ActiveRadar, Seeker::Active)] {
            let stores = arsenal(kind, family);
            assert!(!stores.is_empty(), "{kind:?} has no {family:?}");
            for store in stores {
                let (c, _) = fight(kind, store);
                assert!(c.loadout.count(store) >= 2);
                assert_eq!(c.station().unwrap(), c.station_for(store.mode()).unwrap());
                assert_eq!(store.missile().unwrap().seeker, seeker);
            }
        }
    }
}

#[test]
fn growler_receivers_keep_the_wingtips_clear_while_all_fox_types_remain_underwing() {
    for mission in MissionKind::ALL {
        let mut cfg = Config::for_jet(mission, Type::EA18G);
        assert!(cfg.tips_empty);
        for _ in 0..5 {
            cfg.cycle(0);
            assert!(cfg.tips_empty);
            assert!(Loadout::configure(&cfg).stations.iter().all(|s| !s.tip));
        }
        cfg.tips_empty = false;
        assert!(Loadout::configure(&cfg).stations.iter().all(|s| !s.tip), "stale configs cannot populate fixed receivers");
    }
    let cfg = Config::for_jet(MissionKind::Dogfight, Type::EA18G);
    let loadout = Loadout::configure(&cfg);
    for store in [Store::Aim7, Store::Aim9, Store::Aim120] {
        assert_eq!(loadout.count(store), 2);
    }
    let mut cfg = Config::for_jet(MissionKind::Dogfight, Type::F16);
    cfg.set_jet(Type::EA18G);
    assert!(cfg.tips_empty && Loadout::configure(&cfg).stations.iter().all(|s| !s.tip));
}

#[test]
fn every_available_missile_obeys_its_prelaunch_lock_rules_on_every_airframe() {
    for kind in Type::ALL {
        for store in MISSILES.into_iter().filter(|s| (0..4).any(|p| pylon_choices(kind, p).contains(&Some(*s)))) {
            let spec = store.missile().unwrap();
            let (mut c, a) = fight(kind, store);
            c.launch(&a, store.mode());
            if spec.semi_active() {
                assert!(c.missiles.is_empty(), "{kind:?} {store:?} launched without illumination");
                assert_eq!(c.loadout.count(store), 2);
            } else {
                assert_eq!(c.missiles.len(), 1);
                assert_eq!(c.missiles[0].target, None);
            }
            let (mut c, a) = fight(kind, store);
            c.lock = Some(Who::Bandit(0));
            c.launch(&a, store.mode());
            assert_eq!(c.missiles.len(), 1);
            assert_eq!(c.missiles[0].target.is_some(), !spec.infrared() || spec.loal,
                "radar cue is not an IR seeker lock: {kind:?} {store:?}");
            if spec.infrared() {
                let (mut c, a) = fight(kind, store);
                c.seeker = Some(Who::Bandit(0));
                c.launch(&a, store.mode());
                c.step_missiles(FIXED_DT, &a);
                assert!(c.missiles[0].tracking && !c.missiles[0].supported,
                    "{kind:?} {store:?} must home independently of radar");
            }
        }
    }
}

#[test]
fn every_radar_missile_requires_radar_rather_than_irst_for_support() {
    for kind in [Type::Mig29, Type::Su27] {
        for store in [Store::R27r, Store::R77] {
            let (mut c, a) = fight(kind, store);
            c.lock = Some(Who::Bandit(0));
            c.irst = true;
            c.launch(&a, store.mode());
            if store == Store::R27r { assert!(c.missiles.is_empty()); }
            else { assert_eq!(c.missiles[0].target, None); }
        }
    }
}

#[test]
fn dead_targets_clear_every_missile_family_before_respawn() {
    for store in MISSILES {
        let kind = if matches!(store, Store::R60m|Store::R73|Store::R27t|Store::R27r|Store::R77) { Type::Mig29 } else { Type::F16 };
        let (mut c, a) = fight(kind, store);
        c.fire_missile(Missile::launch(store.missile().unwrap(), a.pos, a.vel,
            Who::Player, Some(Who::Bandit(0))));
        c.lock = Some(Who::Bandit(0));
        c.seeker = c.lock;
        c.acquiring = c.lock;
        c.kill_bandit(0);
        assert!(c.lock.is_none() && c.seeker.is_none() && c.acquiring.is_none());
        assert!(c.missiles[0].target.is_none() && !c.missiles[0].tracking, "{store:?}");
        c.bandits[0].alive = true;
        c.bandits[0].hull = Hull::new(c.bandits[0].jet.kind);
        c.step_missiles(FIXED_DT, &a);
        assert!(c.missiles[0].target.is_none() && !c.missiles[0].tracking,
            "{store:?} inherited a respawn without a fresh acquisition");
    }
}

#[test]
fn crashed_ejected_and_broken_airframes_are_invalid_even_before_death_bookkeeping() {
    for store in MISSILES {
        for state in 0..5 {
            let (mut c, mut a) = fight(Type::F16, Store::Aim9);
            let p = c.bandits[0].jet.pos;
            c.fire_missile(Missile::launch(store.missile().unwrap(), p, V3::Z * 250.0,
                Who::Bandit(0), Some(Who::Player)));
            match state {
                0 => a.crashed = true,
                1 => a.ejected = true,
                2 => a.lost.wing[0] = true,
                3 => c.hull.dead = Some("TEST LOSS"),
                _ => c.player_dead = true,
            }
            c.step_missiles(FIXED_DT, &a);
            assert!(c.missiles[0].target.is_none() && !c.missiles[0].tracking, "{store:?} player state {state}");
            let (mut c, a) = fight(Type::F16, Store::Aim9);
            c.fire_missile(Missile::launch(store.missile().unwrap(), a.pos, a.vel,
                Who::Player, Some(Who::Bandit(0))));
            match state {
                0 => c.bandits[0].jet.crashed = true,
                1 => c.bandits[0].jet.ejected = true,
                2 => c.bandits[0].jet.lost.tail = true,
                3 => c.bandits[0].hull.dead = Some("TEST LOSS"),
                _ => c.bandits[0].alive = false,
            }
            c.lock = Some(Who::Bandit(0));
            c.acquiring = c.lock;
            c.sensors(FIXED_DT, &a);
            assert!(c.lock.is_none() && c.acquiring.is_none());
            assert!(!c.radar_contact(&a, 0) && c.candidates(&a, false).is_empty());
            c.step_missiles(FIXED_DT, &a);
            assert!(c.missiles[0].target.is_none() && !c.missiles[0].tracking, "{store:?} bandit state {state}");
        }
    }
}

#[test]
fn replacing_network_airframes_cannot_transfer_existing_missile_tracks() {
    for store in MISSILES {
        let spec = store.missile().unwrap();
        let (mut c, mut a) = fight(Type::F16, Store::Aim9);
        c.fire_missile(Missile::launch(spec, a.pos, a.vel, Who::Player, Some(Who::Bandit(0))));
        let replacement = c.bandits[0].jet.clone();
        c.network_reset_remote(0, replacement, MissionKind::Dogfight);
        assert!(!c.missiles[0].tracking && c.missiles[0].target.is_none());
        c.fire_missile(Missile::launch(spec, c.bandits[0].jet.pos, V3::Z * 250.0,
            Who::Bandit(0), Some(Who::Player)));
        let replacement = a.clone();
        c.network_reset_player(&mut a, replacement, MissionKind::Dogfight);
        assert!(!c.missiles[1].tracking && c.missiles[1].target.is_none());
        c.fire_missile(Missile::launch(spec, a.pos, a.vel, Who::Player, Some(Who::Bandit(0))));
        c.network_clear_ai();
        assert!(!c.missiles[2].tracking && c.missiles[2].target.is_none());
    }
}

#[test]
fn terminal_active_and_ir_missiles_survive_launcher_loss_but_fox_one_does_not() {
    for store in MISSILES {
        let (mut c, a) = fight(Type::F16, Store::Aim9);
        let spec = store.missile().unwrap();
        c.fire_missile(Missile::launch(spec, c.bandits[0].jet.pos, V3::Z * 250.0,
            Who::Bandit(0), Some(Who::Player)));
        c.bandits[0].alive = false;
        c.step_missiles(FIXED_DT, &a);
        assert_eq!(c.missiles[0].tracking, !spec.semi_active(), "{store:?}");
        assert!(!c.missiles[0].supported);
    }
}
