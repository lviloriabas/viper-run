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
    let mut halves=[Mesh::new(),Mesh::new()];let mut segments:Vec<(V3,V3)>=Vec::new();
    for tri in mesh.v.chunks_exact(STRIDE*3) {
        let vs:Vec<[f32;STRIDE]>=tri.chunks_exact(STRIDE).map(|v|v.try_into().unwrap()).collect();
        let pos=|v:&[f32;STRIDE]|v3(v[0] as f64,v[1] as f64,v[2] as f64);
        let ds:Vec<_>=vs.iter().map(|v|pos(v).dot(n)-d).collect();
        let mut cut:Vec<V3>=Vec::new();
        for i in 0..3 {
            let (a,b)=(pos(&vs[i]),pos(&vs[(i+1)%3]));
            let (da,db)=(ds[i],ds[(i+1)%3]);
            let p=if da.abs()<1e-8 {Some(a)} else if da*db < -1e-16 {Some(a.lerp(b,da/(da-db)))} else {None};
            if let Some(p)=p {if !cut.iter().any(|q|(*q-p).len()<1e-5){cut.push(p);}}
        }
        if cut.len()==2 && ds.iter().any(|v|v.abs()>1e-8) {
            let (a,b)=(cut[0],cut[1]);
            if !segments.iter().any(|(c,d)| ((a-*c).len()<1e-5&&(b-*d).len()<1e-5)||((a-*d).len()<1e-5&&(b-*c).len()<1e-5)) {segments.push((a,b));}
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
    // Each connected cut loop gets its own cap. Angular sorting all cut
    // points joined unrelated wings, nacelles and concave rims with long bars.
    while let Some((a,b))=segments.pop() {
        let mut rim=vec![a,b];
        while (rim[0]-*rim.last().unwrap()).len()>1e-5 {
            let end=*rim.last().unwrap();
            let Some(i)=segments.iter().position(|(p,q)|(*p-end).len()<1e-5||(*q-end).len()<1e-5) else {break};
            let(p,q)=segments.swap_remove(i);rim.push(if(p-end).len()<1e-5{q}else{p});
        }
        if rim.len()<4 || (rim[0]-*rim.last().unwrap()).len()>1e-5 {continue;}
        rim.pop();
        cap(&mut halves,&mut rim,n.norm());
    }
    let [a,b]=halves;(a,b)
}

fn cap(halves:&mut [Mesh;2],rim:&mut Vec<V3>,n:V3) {
    let area=(0..rim.len()).map(|i|rim[i].cross(rim[(i+1)%rim.len()]).dot(n)).sum::<f64>();
    if area<0.0{rim.reverse();}
    while rim.len()>3 {
        let len=rim.len();
        let Some(i)=(0..len).find(|&i| {
            let(a,b,c)=(rim[(i+len-1)%len],rim[i],rim[(i+1)%len]);
            if(b-a).cross(c-b).dot(n)<=1e-8{return false;}
            !(0..len).filter(|&j|j!=i&&j!=(i+len-1)%len&&j!=(i+1)%len).any(|j| {
                let p=rim[j];
                (b-a).cross(p-a).dot(n)>-1e-10 && (c-b).cross(p-b).dot(n)>-1e-10 && (a-c).cross(p-c).dot(n)>-1e-10
            })
        }) else {
            // Collinear samples are common in triangulated source skins.
            if let Some(i)=(0..len).find(|&i|(rim[i]-rim[(i+len-1)%len]).cross(rim[(i+1)%len]-rim[i]).len()<1e-8){rim.remove(i);continue;}
            return;
        };
        let(a,b,c)=(rim[(i+len-1)%len],rim[i],rim[(i+1)%len]);
        halves[0].tri(a,b,c,Rgb(0.30,0.33,0.34));halves[1].tri(c,b,a,Rgb(0.30,0.33,0.34));rim.remove(i);
    }
    if rim.len()==3 && (rim[1]-rim[0]).cross(rim[2]-rim[0]).len()>1e-8 {halves[0].tri(rim[0],rim[1],rim[2],Rgb(0.30,0.33,0.34));halves[1].tri(rim[2],rim[1],rim[0],Rgb(0.30,0.33,0.34));}
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
    #[test]
    fn cut_caps_never_bridge_disconnected_skins() {
        let mut m=Mesh::new();
        for x in [-3.0,3.0] {m.block(&Frame{o:v3(x,0.0,0.0),x:V3::X,y:V3::Y,z:V3::Z},v3(1.0,0.2,1.0),Rgb(0.6,0.6,0.6));}
        let(a,b)=split(m,V3::Z,0.0);
        for m in [a,b] {for tri in m.v.chunks_exact(STRIDE*3) {
            let lo=tri.chunks(STRIDE).map(|v|v[0]).fold(f32::INFINITY,f32::min);
            let hi=tri.chunks(STRIDE).map(|v|v[0]).fold(f32::NEG_INFINITY,f32::max);
            assert!(hi-lo<2.01,"cap bridges separate components");
        }}
    }
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
                Type::F18|Type::F18E|Type::EA18G=>crate::hornet::build_for(kind).controls,
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
                for flaps in [0.0,0.5,1.0] {for command in [-1.0,-0.5,0.0,0.5,1.0] {
                    let mut probe=Aircraft::new(&crate::sim::Mission::new());probe.kind=kind;
                    probe.flaps=flaps;probe.alpha=flaps*20.0*DEG;
                    probe.surf_roll=command;probe.surf_yaw= -command;
                    let angle=s.hinge.angle(&probe);assert!(angle.is_finite());
                    let rotation=Quat::axis_angle(axis,angle);
                    for v in s.mesh.v.chunks(STRIDE) {
                        let local=v3(v[0] as f64,v[1] as f64,v[2] as f64)-p;
                        let moved=rotation.rotate(local);
                        assert!((moved.len()-local.len()).abs()<1e-8,"non-rigid control travel {kind:?}");
                        assert!((moved.dot(axis)-local.dot(axis)).abs()<1e-8,"hinge drift {kind:?}");
                    }
                }}
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
