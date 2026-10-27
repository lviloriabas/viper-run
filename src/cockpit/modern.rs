//! Reference-led single-seat F/A-18C with UFCD and baseline Su-27SK cockpits.
//! Geometry is in the shared design-eye space. All instrument faces use
//! their board's orthonormal frame; consoles clear the common pilot envelope.
use super::*;
use crate::sim::Type;
const DARK: Rgb = Rgb(0.075, 0.085, 0.09);
const IFEI: R = r(0.0, 1872.0, 256.0, 176.0);
const UFC: R = r(384.0, 1872.0, 160.0, 176.0);
const HUD_CONTROL: R = r(704.0, 1872.0, 256.0, 96.0);
const FLANKER_WEAPONS: R = r(1024.0, 1840.0, 448.0, 64.0);
const FLANKER_SYSTEMS: R = r(1472.0, 1840.0, 192.0, 64.0);
const FLANKER_WARN: R = r(1024.0, 640.0, 96.0, 320.0);

/// Native board tilt and eye position, shared by bezels and their live faces.
fn instrument(kind:Type,x:f64,y:f64)->Frame {
    let (o,tilt)=if kind==Type::Su27 {
        (crate::flightgear::cockpit_point(kind,v3(4.64,0.92,0.0)),20.5*DEG)
    } else {(v3(0.0,0.64,-4.10),8.5*DEG)};
    let f=Frame{o,x:V3::X,y:v3(0.0,tilt.cos(),-tilt.sin()),z:v3(0.0,tilt.sin(),tilt.cos())};
    f.at(x,(y-o.y)/tilt.cos(),0.0)
}
#[derive(Clone, Copy)]
enum Shape {
    Dial,
    Display(bool),
    Label,
}
#[derive(Clone, Copy)]
struct Instrument {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    rect: R,
    shape: Shape,
}
fn dial(x: f64, y: f64, radius: f64, rect: R) -> Instrument {
    Instrument {
        x,
        y,
        w: radius * 2.0,
        h: radius * 2.0,
        rect,
        shape: Shape::Dial,
    }
}
fn display_face(x: f64, y: f64, w: f64, h: f64, rect: R, buttons: bool) -> Instrument {
    Instrument {
        x,
        y,
        w,
        h,
        rect,
        shape: Shape::Display(buttons),
    }
}
fn label(x: f64, y: f64, w: f64, h: f64, rect: R) -> Instrument {
    Instrument {
        x,
        y,
        w,
        h,
        rect,
        shape: Shape::Label,
    }
}
/// Fit the complete face without stretching or cropping its pixel grid.
fn face_size(item: &Instrument) -> (f64, f64) {
    let margin = match item.shape {
        Shape::Display(true) => 0.69,
        Shape::Display(false) => 0.91,
        _ => 1.0,
    };
    let scale = (item.w * margin / item.rect.w as f64).min(item.h * margin / item.rect.h as f64);
    (item.rect.w as f64 * scale, item.rect.h as f64 * scale)
}

// Dense groups and proportions reconstructed from the supplied cockpit photos.
fn layout(kind: Type) -> Vec<Instrument> {
    if kind == Type::Su27 {
        let sc=14.7/15.25;
        let y=|z:f64|crate::flightgear::cockpit_point(kind,v3(4.64,0.920+z*(20.0*DEG).cos(),0.0)).y;
        vec![
            dial(-0.085*sc,y(0.080),0.040*sc,mig::R_ADI),
            dial(-0.085*sc,y(-0.050),0.040*sc,mig::R_HSI),
            dial(-0.194*sc,y(0.082),0.032*sc,mig::R_ASI),
            dial(-0.198*sc,y(0.000),0.031*sc,mig::R_ALT),
            dial(-0.281*sc,y(0.084),0.032*sc,mig::R_UUAP),
            dial(-0.281*sc,y(0.000),0.030*sc,mig::R_RADALT),
            dial(0.018*sc,y(0.075),0.032*sc,mig::R_VVI),
            dial(0.110*sc,y(0.060),0.034*sc,mig::R_RPM),
            dial(0.082*sc,y(-0.020),0.025*sc,mig::R_EGT),
            dial(0.148*sc,y(-0.020),0.025*sc,mig::R_EGT_R),
            dial(-0.275*sc,y(-0.090),0.028*sc,mig::R_CLOCK),
            display_face(0.250*sc,0.935,0.142,0.142,mig::R_TV,false),
            dial(0.370*sc,y(-0.110),0.039*sc,mig::R_SPO),
            label(0.220*sc,y(0.0000),0.044,0.098,mig::R_FUEL),
            label(0.300*sc,y(0.010),0.055,0.100,mig::R_EKRAN),
            label(-0.345*sc,y(-0.090),0.050,0.054,mig::R_GEAR),
            label(0.0,1.110,0.190,0.030,FLANKER_WEAPONS),
            label(-0.370*sc,y(-0.030),0.046,0.035,FLANKER_SYSTEMS),
            label(0.420*sc,y(-0.006),0.021,0.095,FLANKER_WARN),
        ]
    } else {
        // The first supplied photo has the upgraded UFCD, not a keypad UFC.
        let mut v=vec![
            display_face(-0.250, 0.885, 0.245, 0.245, R_MFD_L, true),
            display_face(0.250, 0.885, 0.245, 0.245, R_MFD_R, true),
            display_face(0.0, 1.005, 0.215, 0.240, UFC, true),
            label(0.0, 0.812, 0.210, 0.080, HUD_CONTROL),
            display_face(0.0, 0.590, 0.225, 0.225, R_HSI, true),
            label(-0.265, 0.595, 0.215, 0.150, IFEI),
            dial(0.200, 0.650, 0.041, R_DIALS[2]),
            dial(0.335, 0.650, 0.041, R_DIALS[0]),
            dial(0.180, 0.515, 0.031, R_DIALS[1]),
            dial(0.255, 0.515, 0.031, R_DIALS[3]),
            dial(0.330, 0.515, 0.031, R_VVI),
            dial(-0.432, 0.690, 0.029, R_RWR),
            label(-0.432, 0.880, 0.040, 0.170, R_ARM),
            label(-0.420, 1.035, 0.034, 0.075, R_AOA),
            label(-0.270, 1.065, 0.150, 0.050, R_CAUTION),
            label(0.270, 1.065, 0.150, 0.050, R_CAUTION),
        ];
        for i in &mut v {i.x*=0.82;i.w*=0.82;i.h*=0.82;}
        v
    }
}
/// Physical aperture used both by the mesh and HUD clipping.
pub(super) fn hud_glass(kind:Type)->[V3;4] {
    if kind==Type::Su27 {
        let offset=crate::airframe::EYE-kind.eye();
        [v3(-0.073519279,1.272033713,-6.303307884),v3(0.092537705,1.272030734,-6.303317319),
         v3(0.092711213,1.494010153,-6.192297851),v3(-0.073683148,1.494010153,-6.192297851)].map(|p|p+offset)
    }else{
        [v3(-0.123,1.180,-4.09),v3(0.123,1.180,-4.09),v3(0.132,1.445,-4.14),v3(-0.132,1.445,-4.14)]
    }
}
// Projector controls are on the front of its housing, above the main board.
fn mount(kind: Type, item: &Instrument) -> Frame {
    let mut f = instrument(kind, item.x, item.y);
    if kind==Type::Su27 && item.rect.x==FLANKER_WEAPONS.x && item.rect.y==FLANKER_WEAPONS.y {
        f.o.z=-3.55;
    }
    if kind==Type::Su27 && item.rect.x==mig::R_TV.x && item.rect.y==mig::R_TV.y {
        // Raised monitor case sits aft of the coaming and HUD's side wall.
        f.o.z+=0.11;
    }
    if kind == Type::F18 && ((item.rect.x == UFC.x && item.rect.y == UFC.y) || (item.rect.x == HUD_CONTROL.x && item.rect.y == HUD_CONTROL.y)) {
        f.o.z += 0.17;
    }
    if kind == Type::Su27 && item.rect.x == HUD_CONTROL.x && item.rect.y == HUD_CONTROL.y {
        f.o.z += 0.17;
    }
    f
}
// The console atlas occupies the F-16 console area only for these aircraft;
// the active type draws its own legends after the common atlas pass.
fn console_rect(side: f64, row: usize) -> R {
    let i = row + if side > 0.0 { 6 } else { 0 };
    r(
        1024.0 + (i % 4) as f32 * 256.0,
        (i / 4) as f32 * 192.0,
        if side < 0.0 && (1..=3).contains(&row) {
            128.0
        } else {
            256.0
        },
        192.0,
    )
}
fn console_labels(russian: bool, side: f64, row: usize) -> [&'static str; 9] {
    if russian {
        if side < 0.0 {
            [
                [
                    "SHASSI / ZAKRYLKI",
                    "SHASSI",
                    "ZAKR",
                    "NOS",
                    "TORM",
                    "VYP",
                    "UBR",
                    "AVAR",
                    "TEST",
                ],
                [
                    "DVIGATELI",
                    "LEV",
                    "PRAV",
                    "ZAP",
                    "STOP",
                    "TOPL",
                    "NASOS",
                    "REZH",
                    "OTKL",
                ],
                [
                    "SAU", "KREN", "TANG", "VYS", "KURS", "STAB", "MARSH", "POS", "TEST",
                ],
                [
                    "VOORUZHENIE",
                    "PUSK",
                    "SBROS",
                    "SER",
                    "PUSH",
                    "BORT",
                    "ZALP",
                    "AVAR",
                    "OTKL",
                ],
                [
                    "RADIO", "VHF", "UHF", "VOL", "KAN", "RSBN", "ARK", "SAU", "SVYAZ",
                ],
                [
                    "TOPLIVO", "LEV", "PRAV", "NASOS", "BAKI", "PODV", "OSN", "AVAR", "TEST",
                ],
            ][row]
        } else {
            [
                [
                    "ELEKTRO", "AKK", "GEN L", "GEN R", "AC", "SET", "PIT", "AVAR", "TEST",
                ],
                [
                    "KONDITSIONER",
                    "TEMP",
                    "DAVL",
                    "KAB",
                    "VENT",
                    "OBDUV",
                    "OTOP",
                    "STEK",
                    "TEST",
                ],
                [
                    "NAVIGATSIYA",
                    "RSBN",
                    "ARK",
                    "KURS",
                    "REZH",
                    "KAN",
                    "MARSH",
                    "POS",
                    "TEST",
                ],
                [
                    "OSVESHCHENIE",
                    "PRIB",
                    "PULT",
                    "FARA",
                    "BORT",
                    "DEN",
                    "NOCH",
                    "YARK",
                    "TEST",
                ],
                [
                    "KISLOROD", "PIT", "DAVL", "REZH", "AVAR", "SMES", "OBDUV", "MASKA", "TEST",
                ],
                [
                    "AVTOMATY", "RLS", "OEPS", "SAU", "SVYAZ", "PIT", "PRIB", "NAV", "REZ",
                ],
            ][row]
        }
    } else if side < 0.0 {
        [
            [
                "FLIGHT CONTROL",
                "FLAPS",
                "TRIM",
                "FCS",
                "T/O",
                "LAUNCH",
                "BAR",
                "FOLD",
                "RESET",
            ],
            [
                "ENGINES", "L ENG", "R ENG", "START", "CRANK", "FUEL", "BOOST", "IGN", "STOP",
            ],
            [
                "THROTTLE", "IDLE", "MIL", "AB", "CUT", "L ENG", "R ENG", "FUEL", "TEST",
            ],
            [
                "COMMUNICATION",
                "COMM1",
                "COMM2",
                "VOL",
                "ICS",
                "IFF",
                "TCN",
                "ILS",
                "TEST",
            ],
            [
                "FUEL", "PROBE", "DUMP", "EXT", "BINGO", "WING", "CTR", "FEED", "TEST",
            ],
            [
                "EXTERIOR LIGHTS",
                "POS",
                "FORM",
                "STROBE",
                "TAXI",
                "NAV",
                "BRT",
                "DIM",
                "TEST",
            ],
        ][row]
    } else {
        [
            [
                "ELECTRICAL",
                "BATT",
                "L GEN",
                "R GEN",
                "AC",
                "RESET",
                "DC",
                "EXT",
                "TEST",
            ],
            [
                "ENVIRONMENT",
                "TEMP",
                "CABIN",
                "BLEED",
                "DEFOG",
                "RAM",
                "ANTI-I",
                "VENT",
                "TEST",
            ],
            [
                "NAVIGATION",
                "INS",
                "RADAR",
                "ALT",
                "MAP",
                "ALIGN",
                "NAV",
                "ATT",
                "TEST",
            ],
            [
                "INTERIOR LIGHTS",
                "INST",
                "CONSL",
                "FLOOD",
                "CHART",
                "DAY",
                "NIGHT",
                "BRT",
                "TEST",
            ],
            [
                "OXYGEN", "SUPPLY", "FLOW", "MASK", "EMERG", "NORMAL", "100%", "PRESS", "TEST",
            ],
            [
                "CIRCUIT BREAKERS",
                "AVION",
                "COMM",
                "NAV",
                "HUD",
                "DDI L",
                "DDI R",
                "FCS",
                "AUX",
            ],
        ][row]
    }
}
fn draw_consoles(cv: &mut Canvas, russian: bool) {
    for side in [-1.0, 1.0] {
        for row in 0..6 {
            let r = console_rect(side, row);
            let names = console_labels(russian, side, row);
            let ink = rgba(0xc9, 0xd1, 0xcb, 1.0);
            cv.rect(
                r.x,
                r.y,
                r.w,
                r.h,
                if russian {
                    rgba(0x28, 0x39, 0x3c, 1.0)
                } else {
                    rgba(0x20, 0x24, 0x25, 1.0)
                },
            );
            cv.text(
                names[0],
                r.x + r.w * 0.5,
                r.y + 16.0,
                1.0,
                ink,
                Align::Center,
            );
            cv.line(
                r.x + 10.0,
                r.y + 29.0,
                r.x + r.w - 10.0,
                r.y + 29.0,
                1.0,
                DIM,
            );
            let cols = if r.w < 200.0 { 2 } else { 4 };
            for k in 0..2 {
                for j in 0..cols {
                    let x = r.x
                        + if cols == 2 {
                            28.8 + j as f32 * 70.4
                        } else {
                            34.0 + j as f32 * 61.4
                        };
                    let y = r.y + if k == 0 { 64.4 } else { 140.7 };
                    cv.text(
                        names[1 + k * cols + j],
                        x,
                        y - 20.0,
                        1.0,
                        ink,
                        Align::Center,
                    );
                    cv.circle(x, y, 9.0, 1.0, DIM);
                    cv.text(
                        if k == 0 { "ON" } else { "OFF" },
                        x,
                        y + 22.0,
                        1.0,
                        ink,
                        Align::Center,
                    );
                }
            }
        }
    }
}
pub fn build(kind:Type)->Mesh {
    let russian=kind==Type::Su27;
    let panel=if russian{Rgb(0.32,0.53,0.56)}else{Rgb(0.19,0.21,0.22)};
    let wall=panel.scale(0.65);
    let plain=|o|Frame{o,x:V3::X,y:V3::Y,z:V3::Z};
    let mut m=crate::flightgear::cockpit_mesh(kind,"cockpit");
    m.v.extend(crate::flightgear::cockpit_mesh(kind,"frame").v);
    let offset=crate::airframe::EYE-kind.eye();
    let native=|z:f64,phi:f64|crate::hornet::canopy_point(russian,z,phi)+offset;
    let (front,aft)=crate::flightgear::canopy_range(kind);
    // Continuous walls from the original canopy sills to the floor.
    let (front,aft)=(front+0.008,aft-0.008);
    let floor=0.12;
    let outer=if russian{0.68}else{0.43};
    let mut widest:f64=outer;
    for k in 0..32 {
        let z0=mix(front,aft,k as f64/32.0);let z1=mix(front,aft,(k+1) as f64/32.0);
        for phi in [0.0,PI] {
            let a=native(z0,phi);let b=native(z1,phi);
            widest=widest.max(a.x.abs()).max(b.x.abs());
            let side=phi.cos();
            let c=v3(side*outer,0.55,a.z);let d=v3(side*outer,0.55,b.z);
            m.quad(a,b,d,c,wall);
            m.quad(c,d,v3(d.x,floor,d.z),v3(c.x,floor,c.z),wall);
        }
    }
    m.block(&plain(v3(0.0,floor-0.015,(front+aft)*0.5+offset.z)),v3(widest+0.02,0.015,(aft-front)*0.5+0.02),wall);
    for z in [front,aft] {
        let a=native(z,0.0);let b=native(z,PI);
        let c=v3(-outer,0.55,b.z);let d=v3(outer,0.55,a.z);
        m.quad(a,b,c,d,wall);
        m.quad(d,c,v3(-outer,floor,b.z),v3(outer,floor,a.z),wall);
    }
    if !russian {
        let top=|x:f64|if x.abs()<0.12{1.15}else if x.abs()<0.30{1.11}else{mix(1.11,0.94,(x.abs()-0.30)/0.09)};
        let xs=[-0.39,-0.30,-0.12,0.0,0.12,0.30,0.39];
        for pair in xs.windows(2) {
            let (a,b)=(pair[0],pair[1]);
            let (p,q)=(instrument(kind,a,top(a)).o,instrument(kind,b,top(b)).o);
            m.quad(instrument(kind,a,0.40).o-V3::Z*0.025,instrument(kind,b,0.40).o-V3::Z*0.025,q-V3::Z*0.025,p-V3::Z*0.025,panel);
            m.quad(p,q,q-v3(0.0,0.055,0.30),p-v3(0.0,0.055,0.30),DARK);
            m.capsule(p,q,0.012,0.012,DARK,6);
        }
        for side in [-1.0,1.0] {
            let a=instrument(kind,side*0.39,0.40).o;let b=instrument(kind,side*0.39,0.94).o;
            let sill=native(-3.55,if side>0.0{0.0}else{PI});
            m.quad(a,b,sill,v3(sill.x,0.40,sill.z),wall);
            let c=|x:f64,z:f64|v3(side*x,0.62+(x-0.262)*0.24,z);
            m.quad(c(0.262,-3.96),c(0.395,-3.96),c(0.395,-2.74),c(0.262,-2.74),panel);
            m.quad(c(0.262,-3.96),c(0.262,-2.74),v3(side*0.262,floor,-2.74),v3(side*0.262,floor,-3.96),wall);
            for row in 0..6 {
                let z=-3.84+row as f64*0.183;
                let f=Frame{o:c(0.337,z)+V3::Y*0.006,x:v3(1.0,side*0.24,0.0).norm(),y:-V3::Z,z:v3(-side*0.24,1.0,0.0).norm()};
                m.block(&f,v3(0.060,0.077,0.004),DARK);
                let rect=console_rect(side,row);let h=0.118*rect.h as f64/rect.w as f64;
                screen(&mut m,&f.at(0.0,0.0,0.0045),0.118,h,rect);
                for j in 0..3 {
                    let at=f.p(-0.040+j as f64*0.04,-0.025,0.009);
                    if (row+j)%3==0 {m.lathe(at,f.z,&[(0.0,0.007),(0.014,0.007)],8,DARK);}
                    else {m.capsule(at,at+f.z*0.02+f.y*0.007,0.0025,0.002,METALC,6);}
                }
                for x in [-0.052,0.052]{m.block(&f.at(x,0.066,0.006),v3(0.002,0.002,0.001),METALC);}
            }
        }
    }
    for item in layout(kind) {
        let f=mount(kind,&item);let(w,h)=face_size(&item);
        match item.shape {
            Shape::Dial=>gauge_mount(&mut m,&f,item.w*0.5,item.rect,0.008),
            Shape::Display(buttons)=>display(&mut m,&f,item.w,item.h,w,h,item.rect,buttons),
            Shape::Label=>plate(&mut m,&f,w,h,item.rect),
        }
    }
    if russian {
        let item=layout(kind).into_iter().find(|i|i.rect.x==mig::R_TV.x && i.rect.y==mig::R_TV.y).unwrap();
        let f=mount(kind,&item);
        for side in [-1.0,1.0] {
            m.block(&f.at(side*(item.w*0.5+0.006),0.0,-0.085),v3(0.006,item.h*0.5+0.012,0.085),DARK);
        }
        m.block(&f.at(0.0,item.h*0.5+0.006,-0.085),v3(item.w*0.5+0.012,0.006,0.085),DARK);
    }
    if !russian {
        let glass=hud_glass(kind);let foot=(glass[0]+glass[1])*0.5;
        m.block(&plain(foot-v3(0.0,0.066,0.075)),v3(0.110,0.080,0.140),DARK);
        for (a,b) in [(glass[0],glass[3]),(glass[1],glass[2])] {m.capsule(a,b,0.008,0.008,DARK,8);}
    }
    // Mirrors are attached to the source windscreen bow.
    let bow=crate::hornet::canopy_bow(russian);
    let angles=if russian{vec![0.88,PI/2.0,PI-0.88]}else{vec![0.74,PI-0.74]};
    for (i,phi) in angles.into_iter().enumerate() {
        let at=native(bow,phi);let tangent=v3(phi.sin(),-phi.cos(),0.0);let radial=v3(phi.cos(),phi.sin(),0.0);
        let (w,h)=if russian{(0.086,0.035)}else{(0.136,0.060)};
        m.capsule(at,at-radial*0.04+V3::Z*0.04,0.008,0.008,DARK,6);
        curved_mirror(&mut m,w,h,&|x,y,d|at+tangent*x+radial*(y-0.045)+V3::Z*(0.045+d),DARK,
            if russian{-0.01}else if i==0{-0.42}else{0.42});
    }
    let back=native(aft,PI/2.0);
    m.block(&plain(v3(0.0,0.60,back.z+0.02)),v3(widest,0.48,0.025),wall);
    pilot::build_static_mig(&mut m);
    m
}
/// Main levers and selector respond to the same commands as the flight model.
pub(super) fn moving_controls(m: &mut Mesh, kind: Type, h: &Hands) {
    let f = instrument(kind, if kind==Type::F18{-0.340}else{-0.405}, if kind==Type::F18{0.555}else{0.655});
    m.block(&f.at(0.0, 0.0, 0.012), v3(0.018, 0.048, 0.012), DARK);
    let base = f.p(0.0, mix(0.030, -0.030, h.gear), 0.024);
    let tip = base + f.z * 0.065;
    m.capsule(base, tip, 0.005, 0.005, METALC, 8);
    m.lathe(
        tip - f.z * 0.008,
        f.z,
        &[(0.0, 0.022), (0.016, 0.022), (0.016, 0.0)],
        12,
        Rgb(0.85, 0.86, 0.80),
    );
    let lamp = if h.gear_warn {
        Rgb(1.2, 0.18, 0.08)
    } else {
        Rgb(0.12, 0.06, 0.04)
    };
    m.ellipsoid(tip + f.z * 0.010, v3(0.007, 0.007, 0.004), lamp, 4, 8);
    let switch = |m: &mut Mesh, at: V3, value: f64, col: Rgb| {
        let to = at + v3(0.0, 0.024, mix(-0.010, 0.010, value));
        m.capsule(at, to, 0.0027, 0.0027, METALC, 6);
        m.ellipsoid(to, v3(0.005, 0.005, 0.008), col, 4, 8);
    };
    switch(m, v3(if kind==Type::F18{-0.337}else{-0.447}, 0.650, -3.805), h.flaps, DARK);
    // Speedbrake rocker on the outboard side of the moving throttle grip.
    let throttle = v3(-0.355, 0.747, -3.40 - h.throttle * 0.22);
    switch(m, throttle, h.airbrake, Rgb(0.50, 0.51, 0.48));
    if kind == Type::Su27 {
        switch(
            m,
            v3(0.445, 0.650, -3.805),
            if h.irst { 1.0 } else { 0.0 },
            DARK,
        );
    }
    let f = Frame {
        o: v3(if kind==Type::F18{0.337}else{0.445}, 0.653, -3.485),
        x: V3::X,
        y: -V3::Z,
        z: V3::Y,
    };
    m.lathe(f.o, f.z, &[(0.0, 0.012), (0.012, 0.011)], 10, DARK);
    let angle = h.weapon_mode as f64 * 45.0 * DEG;
    m.capsule(
        f.p(0.0, 0.0, 0.013),
        f.p(angle.sin() * 0.009, angle.cos() * 0.009, 0.013),
        0.0012,
        0.0012,
        Rgb(0.85, 0.85, 0.77),
        4,
    );
}

pub fn hornet_instruments(
    cv: &mut Canvas,
    a: &Aircraft,
    ad: &AirData,
    m: &Mission,
    time: f64,
    c: &Combat,
) {
    draw_consoles(cv, false);
    mfd(cv, R_HSI, Page::Hsd, a, ad, m, time, c);
    let r = UFC;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x02, 0x0d, 0x08, 1.0));
    cv.text(
        "COM 1     COM 2",
        r.x + r.w * 0.5,
        r.y + 18.0,
        1.0,
        GREEN,
        Align::Center,
    );
    cv.text(
        "305.000   127.500",
        r.x + r.w * 0.5,
        r.y + 39.0,
        1.2,
        GREEN,
        Align::Center,
    );
    for (i, name) in ["IFF", "TCN", "ILS", "D/L"].into_iter().enumerate() {
        cv.text(
            name,
            r.x + 12.0,
            r.y + 65.0 + i as f32 * 22.0,
            1.0,
            GREEN,
            Align::Left,
        );
    }
    for (i, name) in ["A/P", "WPT", "TIME", "MENU"].into_iter().enumerate() {
        cv.text(
            name,
            r.x + r.w - 12.0,
            r.y + 65.0 + i as f32 * 22.0,
            1.0,
            GREEN,
            Align::Right,
        );
    }
    cv.text(
        &format!("HDG {:03.0}", a.heading),
        r.x + r.w * 0.5,
        r.y + 92.0,
        1.2,
        GREEN,
        Align::Center,
    );
    cv.text(
        c.mode.label(),
        r.x + r.w * 0.5,
        r.y + 119.0,
        1.0,
        GREEN,
        Align::Center,
    );
    cv.stroke_rect(r.x + 49.0, r.y + 145.0, r.w - 98.0, 24.0, 1.0, GREEN);
    let r = HUD_CONTROL;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x24, 0x2a, 0x2a, 1.0));
    for (i, name) in ["HUD", "BRT", "DAY", "ATT", "ALT"].into_iter().enumerate() {
        let x = r.x + 28.0 + i as f32 * 50.0;
        cv.text(name, x, r.y + 13.0, 1.0, WHITE, Align::Center);
        cv.disc(x, r.y + 49.0, 13.0, rgba(0x09, 0x0b, 0x0b, 1.0));
        cv.line(x, r.y + 49.0, x + 4.0, r.y + 38.0, 2.0, WHITE);
    }
    let r = IFEI;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x13, 0x1b, 0x18, 1.0));
    for (i, (name, value)) in [
        ("RPM", format!("{:.0}", a.rpm)),
        ("TEMP", format!("{:.0}", a.ftit)),
        ("FF", format!("{:.1}", a.fuel_flow * 7936.64 * 0.5 / 1000.0)),
        ("NOZ", format!("{:.0}", a.nozzle)),
        ("OIL", format!("{:.0}", a.oil_psi)),
    ]
    .into_iter()
    .enumerate()
    {
        let y = r.y + 17.0 + i as f32 * 26.0;
        cv.text(name, r.x + 80.0, y, 1.0, DIM, Align::Center);
        for x in [r.x + 27.0, r.x + 130.0] {
            cv.text(&value, x, y, 1.0, GREEN, Align::Center);
        }
    }
    cv.text(
        "L   ENGINE   R",
        r.x + 80.0,
        r.y + 148.0,
        1.0,
        DIM,
        Align::Center,
    );
    cv.text("FUEL", r.x + 211.0, r.y + 20.0, 1.1, DIM, Align::Center);
    cv.text(
        &format!("{:.0}", a.fuel * 2.20462),
        r.x + 211.0,
        r.y + 48.0,
        1.6,
        GREEN,
        Align::Center,
    );
    cv.text("BINGO", r.x + 211.0, r.y + 91.0, 1.0, DIM, Align::Center);
    cv.text("2000", r.x + 211.0, r.y + 116.0, 1.4, GREEN, Align::Center);
}
pub fn flanker_instruments(cv: &mut Canvas, a: &Aircraft, c: &Combat) {
    draw_consoles(cv, true);
    let r = FLANKER_WEAPONS;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x24, 0x36, 0x39, 1.0));
    let weapon = symbology::weapon_line(c);
    cv.text(
        &weapon,
        r.x + r.w * 0.5,
        r.y + 19.0,
        1.0,
        GREEN,
        Align::Center,
    );
    cv.text(
        if c.irst {
            "OEPS-27 / IRST"
        } else {
            "N001 / RLS"
        },
        r.x + 12.0,
        r.y + 46.0,
        1.0,
        WHITE,
        Align::Left,
    );
    cv.text(
        if c.lock.is_some() { "ZAKHVAT" } else { "POISK" },
        r.x + r.w - 12.0,
        r.y + 46.0,
        1.0,
        GREEN,
        Align::Right,
    );
    let r = FLANKER_SYSTEMS;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x32, 0x48, 0x4b, 1.0));
    for (i, (name, on)) in [
        ("SHASSI", a.gear > 0.98),
        ("ZAKR", a.flaps > 0.05),
        ("TORM", a.brake > 0.05),
    ]
    .into_iter()
    .enumerate()
    {
        let x = r.x + 32.0 + i as f32 * 64.0;
        cv.text(name, x, r.y + 15.0, 1.0, WHITE, Align::Center);
        cv.disc(
            x,
            r.y + 42.0,
            7.0,
            if on {
                GREEN
            } else {
                rgba(0x12, 0x24, 0x1b, 1.0)
            },
        );
    }
    let r = FLANKER_WARN;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x11, 0x19, 0x1a, 1.0));
    for (i, (name, on)) in [
        ("POZHAR", a.battle.fire),
        ("MASLO", a.oil_psi < 20.0),
        ("TOPLIVO", a.fuel < 800.0),
        ("SHASSI", a.gear > 0.02 && a.gear < 0.98),
        ("TORMOZ", a.brake > 0.1),
        ("ALFA", a.alpha_deg > 22.0),
    ]
    .into_iter()
    .enumerate()
    {
        let height = r.h / 6.0;
        let y = r.y + i as f32 * height;
        cv.rect(
            r.x + 3.0,
            y + 2.0,
            r.w - 6.0,
            height - 4.0,
            if on {
                AMBER
            } else {
                rgba(0x16, 0x22, 0x1b, 1.0)
            },
        );
        cv.text(
            name,
            r.x + r.w * 0.5,
            y + height * 0.5,
            1.0,
            if on { rgba(0x12, 0x10, 0x08, 1.0) } else { DIM },
            Align::Center,
        );
    }
    let r = HUD_CONTROL;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x1b, 0x22, 0x22, 1.0));
    for (i, name) in ["СЕТКА", "ЯРК", "НОЧЬ", "ДЕНЬ"].into_iter().enumerate() {
        let x = r.x + 32.0 + i as f32 * 64.0;
        cv.text(name, x, r.y + 18.0, 1.0, WHITE, Align::Center);
        cv.circle(x, r.y + 56.0, 12.0, 2.0, DIM);
        cv.line(x, r.y + 56.0, x + 4.0, r.y + 45.0, 2.0, WHITE);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hornet_has_two_large_mirrors_while_flanker_keeps_three_small_panes() {
        for kind in [Type::F18, Type::Su27] {
            let m = build(kind);
            let glass: Vec<_> =
                m.v.chunks(crate::mesh::STRIDE)
                    .filter(|v| {
                        let x = v[9] as f64 * AW;
                        let y = (1.0 - v[10] as f64) * A;
                        x >= R_MIRROR.x as f64
                            && x <= (R_MIRROR.x + R_MIRROR.w) as f64
                            && y >= R_MIRROR.y as f64
                            && y <= (R_MIRROR.y + R_MIRROR.h) as f64
                    })
                    .collect();
            assert!(glass.len() > 100);
            assert!(glass.iter().any(|v| v[0] < -0.20));
            assert!(glass.iter().any(|v| v[0] > 0.20));
            assert_eq!(glass.iter().any(|v| v[0].abs() < 0.10), kind == Type::Su27);
            let width = |side: f32| {
                let mut lo = f32::INFINITY;
                let mut hi = f32::NEG_INFINITY;
                for v in glass.iter().filter(|v| side * v[0] > 0.20) {
                    lo = lo.min(v[0]);
                    hi = hi.max(v[0]);
                }
                hi - lo
            };
            for side in [-1.0, 1.0] {
                assert!(width(side) > if kind == Type::F18 { 0.09 } else { 0.04 });
            }
        }
    }

    #[test]
    fn moving_controls_follow_gear_flaps_brake_sensor_and_weapon_state() {
        for kind in [Type::F18, Type::Su27] {
            let initial = Hands::default();
            let mut base = Mesh::new();
            moving_controls(&mut base, kind, &initial);
            for changed in [
                Hands {
                    gear: 1.0,
                    ..initial
                },
                Hands {
                    flaps: 1.0,
                    ..initial
                },
                Hands {
                    airbrake: 1.0,
                    ..initial
                },
                Hands {
                    weapon_mode: 2,
                    ..initial
                },
            ] {
                let mut m = Mesh::new();
                moving_controls(&mut m, kind, &changed);
                assert_ne!(m.v, base.v, "unresponsive control {kind:?}");
                assert!(m.v.iter().all(|v| v.is_finite()));
            }
            let mut sensor = Mesh::new();
            moving_controls(
                &mut sensor,
                kind,
                &Hands {
                    irst: true,
                    ..initial
                },
            );
            assert_eq!(
                sensor.v == base.v,
                kind == Type::F18,
                "IRST selector on wrong aircraft"
            );
        }
    }

    #[test]
    fn active_instrument_and_console_atlas_regions_do_not_overlap() {
        for kind in [Type::F18, Type::Su27] {
            let mut rects: Vec<_> = layout(kind).iter().map(|i| i.rect).collect();
            rects.push(R_MIRROR);
            for side in [-1.0, 1.0] {
                for row in 0..6 {
                    rects.push(console_rect(side, row));
                }
            }
            for (i, a) in rects.iter().enumerate() {
                assert!(
                    a.x >= 0.0 && a.y >= 0.0 && a.x + a.w <= AW as f32 && a.y + a.h <= A as f32
                );
                for b in &rects[i + 1..] {
                    // Identical tiles can be reused (the Hornet caution eyebrows).
                    if a.x == b.x && a.y == b.y && a.w == b.w && a.h == b.h {
                        continue;
                    }
                    assert!(
                        a.x + a.w <= b.x
                            || b.x + b.w <= a.x
                            || a.y + a.h <= b.y
                            || b.y + b.h <= a.y,
                        "atlas faces overlap in {kind:?}: {},{} and {},{}",
                        a.x,
                        a.y,
                        b.x,
                        b.y
                    );
                }
            }
        }
    }

    #[test]
    fn instrument_images_keep_their_native_pixel_aspect() {
        for kind in [Type::F18, Type::Su27] {
            for item in layout(kind) {
                let (w, h) = face_size(&item);
                assert!(
                    (w / h - item.rect.w as f64 / item.rect.h as f64).abs() < 1e-9,
                    "stretched face {kind:?} at {},{}",
                    item.x,
                    item.y
                );
                assert!(w <= item.w + 1e-9 && h <= item.h + 1e-9);
            }
        }
    }

    #[test]
    fn fixed_readouts_fit_inside_the_model_specific_hud_glass() {
        for (kind, ys) in [
            (Type::F18, [-62.0, 0.0, 119.0]),
            (Type::Su27, [-84.0, 0.0, 81.0]),
        ] {
            let g = hud_glass(kind);
            let normal = (g[1] - g[0]).cross(g[3] - g[0]).norm();
            for y in ys {
                for x in [-79.0, 0.0, 79.0] {
                    let (x, y) = fixed_hud_coords(kind, x, y);
                    let ray = v3(x / 650.0, -y / 650.0, -1.0);
                    let eye = crate::airframe::EYE;
                    let p = eye + ray * ((g[0] - eye).dot(normal) / ray.dot(normal));
                    let t = (p.y - g[0].y) / (g[3].y - g[0].y);
                    let half = mix(g[1].x, g[2].x, t);
                    assert!(
                        t > 0.015 && t < 0.985 && p.x.abs() < half - 0.005,
                        "clipped HUD readout {kind:?}: {p:?}"
                    );
                }
            }
        }
    }
    #[test]
    fn instrument_faces_are_visible_and_do_not_overlap() {
        for kind in [Type::F18, Type::Su27] {
            let mut m = build(kind);
            let offset=crate::airframe::EYE-kind.eye();
            for t in crate::flightgear::textures(kind) {
                let mut v=t.mesh.v.clone();
                for p in v.chunks_mut(crate::mesh::STRIDE){p[0]+=offset.x as f32;p[1]+=offset.y as f32;p[2]+=offset.z as f32;}
                m.v.extend(v);
            }
            assert!(m.count() > 1000 && m.v.iter().all(|x| x.is_finite()));
            let items = layout(kind);
            let mut hidden=Vec::new();
            for (i, item) in items.iter().enumerate() {
                let f = mount(kind, &item);
                assert!(f.x.dot(f.y).abs() < 1e-8 && (f.z.len() - 1.0).abs() < 1e-8);
                let (w, h, depth) = match item.shape {
                    Shape::Dial => (item.w, item.h, 0.0135),
                    Shape::Display(_) => {
                        let (w, h) = face_size(item);
                        (w, h, 0.006)
                    }
                    Shape::Label => {
                        let (w, h) = face_size(item);
                        (w, h, 0.0085)
                    }
                };
                for x in [-0.7, 0.0, 0.7] {
                    for y in [-0.7, 0.0, 0.7] {
                        let target = f.p(x * w * 0.5, y * h * 0.5, depth);
                        let hit =
                            m.v.chunks(crate::mesh::STRIDE * 3)
                                .find(|tri| blocked(crate::airframe::EYE, target, tri));
                        if hit.is_some(){hidden.push((i,x,y,target,hit.map(|t|t[..6].to_vec())));}
                    }
                }
                for next in &items[i + 1..] {
                    let gap_x = (item.x - next.x).abs() - (item.w + next.w) * 0.5;
                    let gap_y = (item.y - next.y).abs() - (item.h + next.h) * 0.5;
                    assert!(
                        gap_x >= 0.008 || gap_y >= 0.008,
                        "{kind:?}: overlapping instruments at {},{} and {},{}",
                        item.x,
                        item.y,
                        next.x,
                        next.y
                    );
                }
            }
            assert!(hidden.is_empty(),"hidden faces in {kind:?}: {hidden:?}");
        }
    }
    #[test]
    fn cockpit_tub_is_opaque_below_the_sills_in_every_direction() {
        for kind in [Type::F18,Type::Su27] {
            let m=build(kind);
            for k in 0..72 {
                let a=k as f64*TAU/72.0;
                for radius in [0.65,0.85] {
                    let target=crate::airframe::EYE+v3(a.sin()*radius,-1.24,a.cos()*radius);
                    assert!(m.v.chunks(crate::mesh::STRIDE*3).any(|t|blocked(crate::airframe::EYE,target,t)),
                        "open tub in {kind:?}, angle {k}, target {target:?}");
                }
            }
        }
    }
    #[test]
    fn consoles_leave_the_pilot_and_throttle_envelopes_open() {
        for kind in [Type::F18, Type::Su27] {
            let mut pilot_mesh = Mesh::new();
            pilot::build_dynamic_center(
                &mut pilot_mesh,
                &Hands {
                    rudder: 1.0,
                    throttle: 1.0,
                    ..Default::default()
                },
                kind == Type::Su27,
            );
            let cockpit = build(kind);
            // Floor and console edges lie below the boot soles and outside the bucket.
            assert!(cockpit.v.iter().all(|v| v.is_finite()));
            for v in pilot_mesh.v.chunks(crate::mesh::STRIDE) {
                if v[1] < 0.60 && v[2] > -3.9 && v[2] < -3.0 {
                    assert!(
                        v[0].abs() < 0.262 + 0.012,
                        "pilot intersects console: {v:?}"
                    );
                    assert!(v[1] > 0.15, "boot goes through the cockpit floor");
                }
            }
        }
    }
}
