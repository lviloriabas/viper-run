//! FlightGear paint adapted to the procedural airframes. See MODEL-LIVERIES.md.
use crate::{math::*,mesh::{Mesh,STRIDE},sim::Type};

pub const WIDTH:usize=2048;
pub const HEIGHT:usize=1024;

pub fn pixels(kind:Type)->Vec<u8> {
    let bytes=match kind {
        Type::F16=>include_bytes!("../assets/models/f16-skin.qoi").as_slice(),
        Type::Mig29=>include_bytes!("../assets/models/mig29-skin.qoi").as_slice(),
        _=>unreachable!("procedural paint atlas"),
    };
    let(w,h,px)=crate::flightgear::image(bytes);
    assert_eq!((w as usize,h as usize),(WIDTH,HEIGHT));px
}

fn uv(p:V3,n:V3)->(f32,f32) {
    let u=((p.z+9.0)/17.7).clamp(0.001,0.999)*0.5;
    if n.y.abs()>=n.x.abs()*0.8 {
        (u as f32,((p.x+6.0)/12.0*0.5+if n.y<0.0{0.5}else{0.0}) as f32)
    }else{
        ((u+0.5) as f32,((3.6-p.y)/5.2*0.5+if n.x<0.0{0.5}else{0.0}) as f32)
    }
}

/// One projection per triangle keeps UVs on the parent skin when articulated.
/// Material selection leaves glass, decals, intake cavities and metal unpainted.
pub fn wrap_mig(m:&mut Mesh,painted:impl Fn(&[f32])->Option<f32>) {
    for tri in m.v.chunks_mut(STRIDE*3) {
        let p:Vec<_>=tri.chunks(STRIDE).map(|v|v3(v[0] as f64,v[1] as f64,v[2] as f64)).collect();
        let facing=tri.chunks(STRIDE).fold(V3::ZERO,|sum,v|sum+v3(v[3] as f64,v[4] as f64,v[5] as f64));
        let n=if facing.len()>1e-6{facing}else{(p[1]-p[0]).cross(p[2]-p[0])};
        for (v,p) in tri.chunks_mut(STRIDE).zip(p) {
            if let Some(brightness)=painted(v) {
                let(u,w)=uv(p,n);v[6..9].fill(brightness);v[9]=u;v[10]=w;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn skins_have_distinct_colours_and_detail_in_their_own_atlases() {
        let f=pixels(Type::F16);let m=pixels(Type::Mig29);
        assert_ne!(f,m);
        assert!(!m.chunks(4).any(|p|p[1]>170 && p[1] as f32>p[0] as f32*1.5 && p[1] as f32>p[2] as f32*1.5),"wheel-hub green leaked into airframe paint");
        for px in [&f,&m] {
            assert_eq!(px.len(),WIDTH*HEIGHT*4);
            assert!(px.chunks(4).all(|p|p[3]==255));
            let colours:std::collections::BTreeSet<_>=px.chunks(4).map(|p|[p[0],p[1],p[2]]).collect();
            assert!(colours.len()>24 && colours.len()<=96);
        }
    }
    #[test]
    fn projected_mig_paint_stays_on_its_view_and_keeps_special_materials() {
        let a=crate::mig29::build();let mut painted=0;let mut metal=0;
        for mesh in std::iter::once(&a.body).chain(a.stabs.iter()).chain(a.brakes.iter()).chain(a.controls.iter().map(|c|&c.mesh)) {
            for v in mesh.v.chunks(STRIDE) {
                if v[9]>=0.0 {
                    painted+=1;assert!((0.0..=1.0).contains(&v[9]) && (0.0..=1.0).contains(&v[10]));
                }else if v[2]>6.8 {metal+=1;}
            }
        }
        assert!(painted>5000);assert!(metal>100,"nozzle material must remain separate");
        assert!(a.canopy.v.chunks(STRIDE).all(|v|v[9]<0.0));
    }
}
