//! Control meshes own their skin, hinges and travel. The renderer and QA use
//! these same poses. Axes are right, up, aft in metres.
use crate::{math::*, mesh::{Mesh,Rgb,STRIDE},sim::Aircraft};
#[derive(Clone,Copy,Debug)]
pub enum Drive { Slat, Flap, Aileron, Rudder }
#[derive(Clone,Copy,Debug)]
pub struct Hinge { pub pivot:V3,pub axis:V3,pub drive:Drive,pub side:f64 }
pub struct Surface { pub mesh:Mesh,pub hinge:Hinge }
impl Hinge {
    pub fn angle(self,a:&Aircraft)->f64 {
        let f=a.flaps.clamp(0.0,1.0);
        match self.drive {
            Drive::Slat => -(f*20.0+(a.alpha*RAD-4.0).clamp(0.0,16.0)).clamp(0.0,30.0)*DEG,
            Drive::Flap => (f*a.kind.flap_angle()+a.flap_damage*8.0)*DEG,
            Drive::Aileron => (f*12.0-self.side*a.surf_roll*21.5).clamp(-25.0,25.0)*DEG,
            Drive::Rudder => a.surf_yaw.clamp(-1.0,1.0)*25.0*DEG,
        }
    }
    pub fn pose(self,a:&Aircraft)->M4 {crate::world::rotate_about(self.pivot,Quat::axis_angle(self.axis,self.angle(a)))}
}
/// Clip a triangle soup on a plane, interpolating the original UV and colour.
/// Both halves receive an unpainted hinge closure, never a duplicate outer skin.
pub fn split(mesh:Mesh,n:V3,d:f64)->(Mesh,Mesh) {
    let mut halves=[Mesh::new(),Mesh::new()];let mut rim:Vec<V3>=Vec::new();
    for tri in mesh.v.chunks_exact(STRIDE*3) {
        let vs:Vec<[f32;STRIDE]>=tri.chunks_exact(STRIDE).map(|v|v.try_into().unwrap()).collect();
        let pos=|v:&[f32;STRIDE]|v3(v[0] as f64,v[1] as f64,v[2] as f64);
        let ds:Vec<_>=vs.iter().map(|v|pos(v).dot(n)-d).collect();
        if ds.iter().any(|v|v.abs()<1e-8) && ds.iter().any(|v|v.abs()>1e-8) {
            for (v,dd) in vs.iter().zip(&ds) {
                let p=pos(v);
                if dd.abs()<1e-8&&!rim.iter().any(|q|(*q-p).len()<1e-5){rim.push(p);}
            }
        }
        for (k,sign) in [1.0,-1.0].into_iter().enumerate() {
            let mut poly=Vec::new();
            for i in 0..3 {
                let a=vs[i];let b=vs[(i+1)%3];let da=(pos(&a).dot(n)-d)*sign;let db=(pos(&b).dot(n)-d)*sign;
                if da<=1e-8 {poly.push(a);}
                if (da < -1e-8 && db > 1e-8)||(da > 1e-8 && db < -1e-8) {
                    let t=da/(da-db);let mut v=[0.0;STRIDE];
                    for j in 0..STRIDE {v[j]=(a[j] as f64+(b[j]-a[j]) as f64*t) as f32;}
                    let nn=v3(v[3] as f64,v[4] as f64,v[5] as f64).norm();v[3..6].copy_from_slice(&nn.f32());
                    let p=pos(&v);
                    if k==0 && !rim.iter().any(|q|(*q-p).len()<1e-5){rim.push(p);}
                    poly.push(v);
                }
            }
            for i in 1..poly.len().saturating_sub(1) {
                let a=pos(&poly[0]);let b=pos(&poly[i]);let c=pos(&poly[i+1]);
                if (b-a).cross(c-a).len()>1e-10 {
                    for v in [poly[0],poly[i],poly[i+1]] {halves[k].v.extend(v);}
                }
            }
        }
    }
    if rim.len()>=3 {
        let centre=rim.iter().fold(V3::ZERO,|a,b|a+*b)/rim.len() as f64;
        let x=(rim[0]-centre).norm();let y=n.norm().cross(x);
        rim.sort_by(|a,b|{let a=*a-centre;let b=*b-centre;a.dot(y).atan2(a.dot(x)).total_cmp(&b.dot(y).atan2(b.dot(x)))});
        halves[0].polygon(&rim,Rgb(0.30,0.33,0.34));
        rim.reverse();halves[1].polygon(&rim,Rgb(0.30,0.33,0.34));
    }
    let [a,b]=halves;(a,b)
}
/// The intersection of half spaces is the movable region. Everything outside
/// is retained once in the fixed mesh, including closed cut boundaries.
pub fn extract(mesh:Mesh,planes:&[(V3,f64)])->(Mesh,Mesh) {
    let mut fixed=Mesh::new();let mut moving=mesh;
    for &(n,d) in planes {let(a,b)=split(moving,n,d);moving=a;fixed.v.extend(b.v);}
    (fixed,moving)
}
#[cfg(test)]
mod tests {
    use super::*; use crate::mesh::Frame;
    #[test] fn clipped_skin_keeps_uv_and_has_no_outer_overlap() {
        let mut m=Mesh::new();m.block(&Frame{o:V3::ZERO,x:V3::X,y:V3::Y,z:V3::Z},v3(1.0,0.1,1.0),Rgb(1.0,1.0,1.0));
        for v in m.v.chunks_mut(STRIDE){v[9]=(v[0]+1.0)*0.5;v[10]=(v[2]+1.0)*0.5;}
        let(a,b)=split(m,V3::Z,0.3); assert!(a.count()>0&&b.count()>0);
        for (m,sign) in [(&a,1.0),(&b,-1.0)] {
            for v in m.v.chunks(STRIDE) {
                assert!((v[2] as f64-0.3)*sign<1e-6);
                if v[9]>=0.0 {assert!((v[9]-(v[0]+1.0)*0.5).abs()<1e-6);}
            }
        }
    }
    #[test] fn controls_have_stable_hinges_and_correct_travel() {
        use crate::sim::{Aircraft,Type};
        for kind in Type::ALL {
            let mut a=Aircraft::new(&crate::sim::Mission::new());a.kind=kind;a.flaps=1.0;a.surf_roll=1.0;a.surf_yaw=1.0;a.alpha=0.0;
            let surfaces=match kind {
                Type::F16=>crate::airframe::build().slats,
                Type::Mig29=>crate::mig29::build().controls,
                Type::F18|Type::F18E|Type::EA18G=>crate::hornet::build().controls,
                Type::Su27=>crate::su27::build().controls,
            };
            for s in surfaces {
                assert!(s.mesh.count()>=6,"empty control {kind:?}");
                assert!((s.hinge.axis.len()-1.0).abs()<1e-6);
                let q=Quat::axis_angle(s.hinge.axis,s.hinge.angle(&a));
                let p=s.hinge.pivot;let axis=s.hinge.axis;
                for u in [-1.0,0.0,1.0] {
                    let v=p+axis*u;
                    assert!((p+q.rotate(v-p)-v).len()<1e-9);
                }
                for v in s.mesh.v.chunks(STRIDE) {
                    assert!(v.iter().all(|x|x.is_finite()));
                    let n=v3(v[3] as f64,v[4] as f64,v[5] as f64);
                    assert!((n.len()-1.0).abs()<0.001);
                }
                match s.hinge.drive {
                    Drive::Slat=>assert!((q.rotate(-V3::Z)).y<0.0),
                    Drive::Flap=>assert!((q.rotate(V3::Z)).y<0.0),
                    Drive::Rudder=>assert!(q.rotate(V3::Z).x>0.0),
                    Drive::Aileron=>{a.flaps=0.0;assert!(s.hinge.angle(&a)*s.hinge.side<0.0);a.flaps=1.0;}
                }
            }
        }
    }

}
