//! Distinct E and G exteriors adapted from Petar Jedvaj/F-GTUX's GPL model.
//! Geometry, source permission, datum and references live in assets/models.
use crate::{articulation::{Drive,Hinge,Surface},combat::{Station,Store},hornet::Airframe,
    math::*,mesh::{Frame,Mesh,Rgb},sim::Type};

const SX:f64=13.7/(2.0*6.58995);
const SY:f64=4.88/(5.13838+0.01097);
const SZ:f64=18.3/(9.18164+9.08501);
pub const fn source_point(p:V3)->V3 {v3(-p.z*SX,(p.y-2.20)*SY,p.x*SZ+0.15)}
// Fit the game pilot/seat to the source envelope. The upstream placeholder
// cockpit eye is too high for the reconstructed HUD and windshield aperture.
pub const EYE:V3=source_point(v3(-3.70,2.85,0.0));
pub const REAR_EYE:V3=source_point(v3(-2.32,3.00,0.0));
pub const NOZZLES:[V3;2]=[source_point(v3(7.555,2.205,0.4785)),source_point(v3(7.555,2.205,-0.4785))];
pub const GEAR_PIVOTS:[(V3,V3,f64);3]=[
    (source_point(v3(-2.297,1.343,0.0)),V3::X,-95.0*DEG),
    (source_point(v3(1.55,1.34,0.67)),V3::Z,-80.0*DEG),
    (source_point(v3(1.55,1.34,-0.67)),V3::Z,80.0*DEG),
];
pub fn bow_point(phi:f64)->V3 {
    let y=2.49342+0.62536*phi.sin();
    source_point(v3(-4.66163-(y-2.49342)*0.15,y,-0.40248*phi.cos()))
}
pub fn canopy_point(kind:Type,z:f64,phi:f64)->V3 {
    let(w,sill,crown)=crate::flightgear::canopy_section(kind,z);
    v3(w*phi.cos(),sill+(crown-sill)*phi.sin().max(0.0),z)
}
pub fn build(kind:Type)->Airframe {
    assert!(matches!(kind,Type::F18E|Type::EA18G));
    let mut a=crate::flightgear::build(kind);
    a.body.v.extend(crate::flightgear::part(kind,"frame").v);
    a.body.v.extend(crate::hornet::exterior_tub(kind).v);
    a.body.round_shading(|p|p.z< -6.9 && p.x.abs()<0.6);
    for (i,side) in [-1.0,1.0].into_iter().enumerate() {
        let axis=|a:V3,b:V3|{let a=source_point(a);let b=source_point(b);(a,(b-a).norm()*side)};
        for (name,drive,ends) in [
            ("slat_inner",Drive::Slat,[v3(0.953,2.211,-side*1.963),v3(2.084,2.101,-side*4.736)]),
            ("slat_outer",Drive::Slat,[v3(2.084,2.101,-side*4.736),v3(2.744,2.056,-side*6.331)]),
            ("aileron",Drive::Aileron,[v3(3.374,2.148,-side*4.519),v3(3.612,2.075,-side*6.394)]),
        ] {
            let(pivot,axis)=axis(ends[0],ends[1]);
            a.controls.push(Surface{mesh:crate::flightgear::part(kind,&format!("{name}{i}")),hinge:Hinge{pivot,axis,drive,side}});
        }
    }
    if kind==Type::F18E {
        // Cover the F/G's removed rear station with the E's raised spine.
        let mut point=|t:f64,phi:f64|source_point(v3(mix(-2.404,-1.08,t),2.519+0.686*phi.sin(),-mix(0.490,0.46,t)*phi.cos()));
        loft_roof(&mut a.body,&mut point,Rgb(0.58,0.62,0.65));
    }
    for nozzle in NOZZLES {
        // Physical dark cavity, deliberately without any albedo UV.
        a.body.lathe(nozzle,V3::Z,&[(0.0,0.405),(-0.18,0.40),(-0.55,0.32),(-0.64,0.0)],24,Rgb(0.055,0.065,0.070));
    }
    for side in [-1.0,1.0] {for x in [2.30,3.50,4.15] {
        let st=Station{store:Store::Aim120,at:v3(side*x,0.0,0.0),loaded:true,tip:false};
        let q=store_rotation(&st);let ax=q.axes();
        for z in [-0.28,0.28] {
            let f=Frame{o:rail(&st)+q.rotate(V3::Z*z)-V3::Y*0.025,x:ax.right,y:ax.up,z:ax.back};
            a.body.block(&f,v3(0.018,0.027,0.035),Rgb(0.27,0.30,0.32));
        }
    }}
    a
}
fn loft_roof(m:&mut Mesh,p:&mut impl FnMut(f64,f64)->V3,c:Rgb) {
    for j in 0..8 {for i in 0..20 {
        let(t,u)=(j as f64/8.0,i as f64*PI/20.0);
        m.quad(p(t,u),p(t+0.125,u),p(t+0.125,u+PI/20.0),p(t,u+PI/20.0),c);
    }}
}
pub fn stab_pivot(side:f64)->V3 {source_point(v3(6.71,2.10,-side*0.90))}
pub fn flap_hinge(side:f64)->(V3,V3) {
    let p=source_point(v3(2.931,2.403,-side*1.219));
    let q=source_point(v3(3.374,2.148,-side*4.519));
    (p,(q-p).norm()*side)
}
pub fn rudder_hinge(side:f64)->(V3,V3) {
    let p=source_point(v3(5.560,2.567,-side*0.966));
    let q=source_point(v3(6.194,4.438,-side*1.641));
    (p,(q-p).norm())
}
/// Store centre and rail tangent from the separate source pylons. Source
/// weapon meshes are excluded; the inventory owns every rendered store.
fn rail(st:&Station)->V3 {
    rail_at(st.at,st.tip)
}
fn rail_at(at:V3,tip:bool)->V3 {
    let x=at.x.abs();let side=at.x.signum();
    if tip {return source_point(v3(2.30,2.076,-side*6.58995));}
    let at=if x>3.8 {v3(1.90,1.417,-side*4.288)}
        else if x>2.9 {v3(1.65,1.433,-side*3.314)}
        else {v3(1.18,1.473,-side*2.384)};
    source_point(at)
}
pub fn store_shift(at:V3)->V3 {rail_at(at,at.x.abs()>4.6)-at}
pub fn store_rotation(st:&Station)->Quat {
    // Real wing stations are toed out for store separation. Tip rails stay
    // parallel to the wingtip; the small incidence follows their source rail.
    let yaw=if st.tip {0.0}else{-st.at.x.signum()*3.5*DEG};
    Quat::axis_angle(V3::Y,yaw).mul(Quat::axis_angle(V3::X,-2.0*DEG))
}
pub fn station_position(st:&Station)->V3 {
    let radius=st.store.missile().map_or(0.14,|s|s.diameter*0.5);
    let centre=if st.tip {rail(st)+V3::X*(st.at.x.signum()*(radius+0.040))}else{rail(st)-V3::Y*(radius+0.045)};
    centre+store_rotation(st).rotate(-V3::Z*st.store.missile().map_or(0.0,|s|s.length*0.5))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{Config,Loadout,MissionKind};
    #[test]
    fn variants_have_distinct_canopies_correct_span_and_compact_movable_parts() {
        let e=build(Type::F18E);let g=build(Type::EA18G);
        let aft=|m:&Mesh|m.v.chunks(crate::mesh::STRIDE).map(|v|v[2]).fold(f32::NEG_INFINITY,f32::max);
        assert!(aft(&g.canopy)-aft(&e.canopy)>1.2,"single-seat roof still matches the tandem canopy");
        assert!(g.body.count()>e.body.count(),"Growler receiver geometry is missing");
        for a in [e,g] {
            assert_eq!(a.brake.count(),0,"Super Hornet has no dorsal brake");
            assert_eq!(a.controls.len(),6);
            let total=a.body.count()+a.canopy.count()+a.stabs.iter().map(Mesh::count).sum::<usize>()
                +a.flaps.iter().map(Mesh::count).sum::<usize>()+a.rudders.iter().map(Mesh::count).sum::<usize>()
                +a.gear.iter().map(Mesh::count).sum::<usize>()+a.controls.iter().map(|s|s.mesh.count()).sum::<usize>();
            assert!(total/3<30000,"exterior exceeds compact budget: {}",total/3);
            let span=a.body.v.chunks(crate::mesh::STRIDE).map(|v|v[0].abs()).fold(0.0,f32::max)*2.0;
            assert!((13.65..14.50).contains(&span),"wrong full-size Super Hornet span: {span}");
            for (i,gear) in a.gear.iter().enumerate() {
                let bottom=gear.v.chunks(crate::mesh::STRIDE).map(|v|v[1] as f64).fold(f64::INFINITY,f64::min);
                assert!((bottom-Type::F18E.wheels()[i].local.y).abs()<0.015,"contact does not match tyre {i}: {bottom}");
            }
        }
    }
    #[test]
    fn mounted_missile_centres_follow_their_own_pylon_and_rail_orientation() {
        for kind in [Type::F18E,Type::EA18G] {
            let cfg=Config::for_jet(MissionKind::Dogfight,kind);
            for st in Loadout::configure(&cfg).stations {
                let Some(spec)=st.store.missile()else{continue};
                let centre=st.position(kind)+st.mount_rotation(kind).rotate(V3::Z*(spec.length*0.5));
                let top=centre+if st.tip{-V3::X*(st.at.x.signum()*spec.diameter*0.5)}else{V3::Y*(spec.diameter*0.5)};
                let contact=rail(&st)+if st.tip{V3::X*(st.at.x.signum()*0.040)}else{-V3::Y*0.045};
                assert!((top-contact).len()<1e-8,"missile origin disagrees with its rail");
                assert!((st.mount_rotation(kind).rotate(-V3::Z).len()-1.0).abs()<1e-9);
                if kind==Type::EA18G{assert!(!st.tip,"EW receiver occupied by a missile");}
            }
        }
    }
}
