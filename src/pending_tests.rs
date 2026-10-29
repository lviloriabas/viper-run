//! Regression coverage for the pending controls, loadout and escape work.
use crate::{audio, combat::*, eject, math::*, settings::Settings, sim::*, weapons::*};

#[test]
fn clean_loadouts_and_empty_guns_survive_airframe_changes() {
    for kind in Type::ALL {
        let mut cfg=Config::for_jet(MissionKind::Free,kind);
        for _ in 0..16 { if cfg.tips_empty {break;} cfg.cycle(0); }
        assert!(cfg.tips_empty,"{kind:?} tip pair never emptied");
        cfg.outer=None;cfg.outboard=None;cfg.inboard=None;cfg.gun_rounds=Some(0);
        let l=Loadout::configure(&cfg);
        assert!(l.stations.is_empty());assert_eq!(l.rounds,0);
        assert!(l.mass()>0.0);assert_eq!(l.drag(),0.0);
        cfg.set_jet(kind.next());
        assert!(cfg.tips_empty && Loadout::configure(&cfg).stations.is_empty());
        assert_eq!(Loadout::configure(&cfg).rounds,0);
    }
}

#[test]
fn ammunition_mass_distinguishes_unloaded_cartridges_and_retained_cases() {
    for kind in Type::ALL {
        let mut cfg=Config::for_jet(MissionKind::Free,kind);
        let full=Loadout::configure(&cfg);
        cfg.gun_rounds=Some(0);let empty=Loadout::configure(&cfg);
        assert!((full.mass()-empty.mass()-full.gun.rounds as f64*full.gun.cartridge_mass).abs()<1e-8);
        cfg.gun_rounds=Some(full.gun.rounds/2);let mut partial=Loadout::configure(&cfg);
        let before=partial.mass();let count=partial.rounds;
        partial.rounds=0;
        assert!((before-partial.mass()-count as f64*full.gun.expended).abs()<1e-8);
        assert!(partial.mass()>empty.mass(),"spent cases remain in the jet");
        assert_eq!(partial.initial_rounds,count);
        cfg.gun_rounds=None;
        for _ in 0..5 {cfg.cycle_ammo();}
        assert_eq!(Loadout::configure(&cfg).rounds,full.gun.rounds);
    }
}

#[test]
fn controller_gain_is_symmetric_monotonic_and_keeps_full_authority() {
    for gain in [0.5,1.0,2.0] {
        let s=Settings{pad_sensitivity:gain,..Settings::default()};
        assert_eq!(s.flight_stick((1.0,-1.0),0.0),(1.0,-1.0));
        assert_eq!(s.flight_stick((0.0,0.0),1.0),(0.0,0.0));
        let mut last=0.0;
        for k in 0..101 {
            let x=k as f64/100.0;let(y,negative)=s.flight_stick((x,-x),0.0);
            assert!(y>=last && y<=1.0);assert!((y+negative).abs()<1e-12);last=y;
        }
    }
}

#[test]
fn every_airframe_retains_protection_at_low_speed_and_high_altitude() {
    for kind in Type::ALL {for (alt,speed) in [(3500.0,105.0),(10000.0,220.0)] {
        let mut a=spawn_type(kind,v3(0.0,alt,0.0),0.0,speed);
        for _ in 0..(8.0/FIXED_DT) as usize {
            a.update(FIXED_DT,&Input{pitch_axis:1.0,roll_axis:0.15,yaw_axis:0.15,afterburner:true,..Input::default()});
            assert!(a.pos.is_finite() && !a.crashed && !a.lost.any(),"{kind:?} at {alt}");
        }
        assert!(a.alpha_deg<kind.alpha_limits().1+2.0,"{kind:?}: {} deg",a.alpha_deg);
        assert!(a.max_g<kind.g_limit(a.mass,a.mach)+1.5,"{kind:?}: {} g",a.max_g);
    }}
    assert!(Type::F18.alpha_limits().1>35.0);
    assert!(Type::F18E.g_limit(19500.0,0.5)>7.0);
}

#[test]
fn unguided_missiles_fall_and_continue_below_mach_one_until_impact_or_timeout() {
    for spec in [&AIM9M,&AIM9X,&R60M,&R73,&R27T,&AIM7M,&R27R,&AIM120C,&AIM120D,&R77] {
        let mut m=Missile::launch(spec,v3(0.0,9000.0,0.0),v3(0.0,0.0,-250.0),Who::Player,None);
        m.age=spec.drop+spec.motor.iter().map(|&(_,t)|t).sum::<f64>()+1.0;
        m.mass=spec.mass-spec.propellant;
        for _ in 0..120 {assert_eq!(m.step(FIXED_DT,None,&|_,_|0.0),Fate::Flying);}
        assert!(m.vel.y < -8.0 && m.pos.y<9000.0,"{} did not fall",spec.name);
        let mut terminal=false;
        for _ in 0..(spec.life/FIXED_DT) as usize {
            match m.step(FIXED_DT,None,&|_,_|0.0) {
                Fate::Ground(_)|Fate::Spent(_)=>{terminal=true;break;},
                Fate::Flying=>{},Fate::Burst(_,_)=>panic!("no target for proximity burst"),
            }
        }
        assert!(terminal,"{} never terminated",spec.name);
    }
}

#[test]
fn ejection_seats_stabilise_on_drogues_and_hold_the_main_at_altitude() {
    for kind in Type::ALL {
        let mut a=Aircraft::new(&Mission::new());a.set_type(kind);
        a.pos=v3(0.0,9000.0,0.0);a.vel=v3(0.0,0.0,-260.0);a.airspeed_kt=400.0;
        let mut e=eject::Ejection::new(&a);
        for _ in 0..(6.0/FIXED_DT) as usize {e.step(FIXED_DT,&a);}
        assert_eq!(e.phase,eject::Phase::Seat,"{kind:?} opened above hold altitude");
        assert!(e.drogue()>0.99);assert_eq!(e.chute,0.0);
        assert!(e.pilot.spin.len()<0.35,"{kind:?} spin {}",e.pilot.spin.len());
        assert!(e.pilot.pos.is_finite() && e.g_load.is_finite());
        e.pilot.pos.y=1000.0;e.step(FIXED_DT,&a);
        assert_eq!(e.phase,eject::Phase::Chute);
        assert_eq!(e.drogue(),0.0);
    }
}

#[test]
fn growler_sequences_two_occupied_seats_rear_first() {
    let mut a=Aircraft::new(&Mission::new());a.set_type(Type::EA18G);
    a.pos=v3(0.0,8000.0,0.0);a.vel=v3(0.0,0.0,-230.0);
    let mut e=eject::Ejection::new(&a);
    assert!(e.rear.is_some());
    for _ in 0..60 {e.step(FIXED_DT,&a);}
    assert_eq!(e.phase,eject::Phase::Canopy);
    assert_eq!(e.rear.as_ref().unwrap().phase,eject::Phase::Seat);
    for _ in 0..90 {e.step(FIXED_DT,&a);}
    assert_eq!(e.phase,eject::Phase::Seat);
    assert!((e.pilot.pos-e.rear.as_ref().unwrap().pilot.pos).len()>1.0);
}

#[test]
fn engine_distance_and_damage_masks_have_no_effect_on_an_intact_cockpit() {
    assert_eq!(audio::distance_gain(0.0),1.0);
    assert!(audio::distance_gain(500.0)<0.03 && audio::distance_gain(2000.0)<0.002);
    let mut a=Aircraft::new(&Mission::new());assert_eq!(crate::world::damage_of(&a),[0.0;4]);
    a.battle.wing=0.8;a.battle.side=-1.0;a.battle.engine=0.5;a.battle.fire=true;
    let d=crate::world::damage_of(&a);assert!(d[0]>d[1] && d[2]>=0.8 && d[3]==0.0);
}

#[test]
fn structural_fatigue_never_turns_into_weapon_impact_or_fire_masks() {
    let mut a=Aircraft::new(&Mission::new());
    a.structure.damage=0.85;
    assert_eq!(crate::world::damage_of(&a),[0.0;4]);
    a.battle.wing=0.7;a.battle.side=1.0;
    let weapon=crate::world::damage_of(&a);
    a.structure.damage=0.0;
    assert_eq!(crate::world::damage_of(&a),weapon);
}
