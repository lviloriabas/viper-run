//! Reproducible Windows/OpenGL menu integration check, using the live input handlers.
//! Run with --ui-check --screen menu --shot menu-check.png --frames 200.
use crate::{App, State, Action, MissionKind, Gamepad, gamepad};
use miniquad::{EventHandler, KeyCode, KeyMods, MouseButton};

fn key(app: &mut App, k: KeyCode) { app.key_down_event(k, KeyMods::default(), false); }
fn select(app: &mut App, action: Action, method: usize) {
    let i = app.buttons.iter().position(|b| b.action == action).expect("menu action visible");
    let b = app.buttons[i];
    match method {
        0 => app.mouse_button_down_event(MouseButton::Left, b.x+b.w*0.5, b.y+b.h*0.5),
        1 => {
            app.selected = i;
            app.pad = Gamepad::new();
            app.pad.state.connected = true;
            app.pad.state.buttons = gamepad::A;
            app.pad_active = true;
            app.pad_buttons();
            assert!(app.pad_active, "controller context lost while activating A");
            app.pad = Gamepad::new();
        },
        _ => { app.selected = i; key(app, KeyCode::Enter); }
    }
}

pub(super) fn step(app: &mut App, frame: u32) -> bool {
    let i = frame as usize - 1;
    if i < MissionKind::ALL.len()*6 {
        let mode = MissionKind::ALL[i/6];
        let method = (i%6)/2;
        if i%2 == 0 {
            assert_eq!(app.state, State::Attract);
            select(app, Action::Pick(mode), method);
        } else {
            assert_eq!(app.state, State::Loadout, "mode must open its setup directly");
            assert_eq!(app.kind, mode);
            println!("PASS mode {:?} via {}", mode, ["click", "controller A", "Enter"][method]);
            key(app, KeyCode::Escape);
            assert_eq!(app.state, State::Attract);
        }
        return false;
    }
    match i-MissionKind::ALL.len()*6 {
        0 => key(app, KeyCode::Escape),
        1 => { assert!(app.controls_open); key(app, KeyCode::Escape); },
        2 => select(app, Action::Pick(MissionKind::Free), 2),
        3 => { assert_eq!(app.state, State::Loadout); select(app, Action::Launch, 0); },
        4 => { assert_eq!(app.state, State::Flying); key(app, KeyCode::Escape); },
        5 => {
            assert!(app.controls_open && app.state == State::Paused);
            app.settings.expo = 0.5; app.settings.sensitivity = 2.0;
            select(app, Action::SensUp, 1);
            assert!((app.settings.expo-0.6).abs() < 1e-8);
            assert_eq!(app.settings.sensitivity, 2.0);
            println!("PASS Escape opens configuration; controller A adjusts expo");
        },
        6 => { key(app, KeyCode::Escape); assert_eq!(app.state, State::Flying); },
        7 => key(app, KeyCode::P),
        8 => { assert_eq!(app.state, State::Frozen); key(app, KeyCode::Escape); },
        9 => { assert!(app.controls_open); key(app, KeyCode::Escape); },
        10 => { assert_eq!(app.state, State::Frozen); app.pause(); },
        11 => { assert_eq!(app.state, State::Paused); key(app, KeyCode::Escape); },
        12 => { assert!(app.controls_open); key(app, KeyCode::Escape); assert_eq!(app.state, State::Frozen); },
        13 => key(app, KeyCode::K),
        14 => select(app, Action::Menu, 0),
        _ => {
            assert!(!app.controls_open && app.state == State::Attract);
            println!("PASS configuration resumes flying/frozen states and Main Menu works");
            assert_eq!(app.performance.frame, frame as u64, "mirror passes counted as display frames");
            println!("PASS one performance sample per display frame, including mirror views");
            println!("PASS menu integration check complete");
            return true;
        }
    }
    false
}


// Hold the loopback peer only for the native UI integration check.
thread_local! {
    static FUTURE_HUB:std::cell::RefCell<Option<crate::net::Directory>>=const{std::cell::RefCell::new(None)};
    static FUTURE_HOST:std::cell::RefCell<Option<crate::net::Session>>=const{std::cell::RefCell::new(None)};
    static FUTURE_PEER:std::cell::RefCell<Option<crate::net::Session>>=const{std::cell::RefCell::new(None)};
    static FUTURE_DELAY:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};
    static FUTURE_JOINED:std::cell::RefCell<Option<std::time::Instant>>=const{std::cell::RefCell::new(None)};
}
pub(super) fn future_step(app:&mut App,frame:u32)->bool{
    FUTURE_HUB.with(|hub| { if let Some(hub)=hub.borrow_mut().as_mut() {
        hub.poll().unwrap();
        FUTURE_HOST.with(|host| { if let Some(host)=host.borrow_mut().as_mut() { host.poll().unwrap(); } });
        hub.poll().unwrap();
    }});

    fn bounds(app:&App){
        for b in &app.buttons{
            assert!(b.x>=0.0&&b.y>=0.0&&b.x+b.w<=app.win.0+0.5&&b.y+b.h<=app.win.1+0.5,"button {:?} outside {:?}",b.action,app.win);
        }
        for(i,a)in app.buttons.iter().enumerate(){for b in &app.buttons[i+1..]{
            let area=(a.x+a.w).min(b.x+b.w)-a.x.max(b.x);
            let height=(a.y+a.h).min(b.y+b.h)-a.y.max(b.y);
            assert!(area<=0.5||height<=0.5,"overlapping {:?} and {:?}",a.action,b.action);
        }}
    }
    let i=frame as usize-1-FUTURE_DELAY.with(|v|v.get());
    if i<60{
        let lesson=crate::tutorial::Lesson::ALL[(i/4)/3];
        let method=(i/4)%3;
        match i%4{
            0=>{assert_eq!(app.state,State::Attract);bounds(app);select(app,Action::School,method);}
            1=>{assert_eq!(app.state,State::School);bounds(app);select(app,Action::Lesson(lesson),method);}
            2=>{
                assert_eq!(app.state,State::Flying);assert_eq!(app.training.as_ref().unwrap().lesson,lesson);
                assert!(!app.mission.timed);assert_eq!(app.aircraft.as_ref().unwrap().kind,app.config.jet);
                assert_eq!(app.combat.sites.len(),if lesson==crate::tutorial::Lesson::Weapons{app.combat.sites.len()}else{0});
                key(app,KeyCode::Escape);
            }
            _=>{assert!(app.controls_open);select(app,Action::Menu,method);assert!(app.training.is_none());}
        }
        return false;
    }
    match i-60{
        0=>{select(app,Action::Online,0);}
        1=>{
            assert_eq!(app.state,State::Network);bounds(app);
            assert_eq!(app.multiplayer.voice.settings().mode,crate::voice::VoiceMode::Disabled);
            assert!(!app.multiplayer.voice.capturing());
            select(app,Action::NetEdit(crate::multiplayer::Field::Name),2);
            for _ in 0..5{key(app,KeyCode::Backspace);}
            for ch in "TestPilot".chars(){app.char_event(ch,KeyMods::default(),false);}
            key(app,KeyCode::Enter);
            assert_eq!(app.multiplayer.menu.name,"TestPilot");
            let probe=std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
            app.multiplayer.menu.port=probe.local_addr().unwrap().port().to_string();
            drop(probe);
            select(app,Action::NetTab(crate::multiplayer::Panel::Lan),0);
        }
        2=>select(app,Action::NetHost,0),
        3=>{
            assert!(app.multiplayer.session.as_ref().unwrap().host);bounds(app);
            assert!(!app.multiplayer.voice.capturing());
            let talk=app.settings.keys.get(crate::keymap::Bind::PushToTalk).iter().flatten().next().copied().unwrap_or(KeyCode::U);
            app.settings.keys.set(crate::keymap::Bind::PushToTalk,0,Some(talk));
            key(app,talk);assert!(app.settings.keys.held(crate::keymap::Bind::PushToTalk,&app.keys));
            app.key_up_event(talk,KeyMods::default());
            select(app,Action::NetLaunch,2);
            assert!(!app.multiplayer.session.as_ref().unwrap().rules.started);
        }
        4=>{
            let mut invite=crate::net::Invite::parse(&app.multiplayer.session.as_ref().unwrap().invitation()).unwrap();
            invite.endpoint.set_ip(std::net::Ipv4Addr::LOCALHOST.into());
            FUTURE_PEER.with(|p|*p.borrow_mut()=Some(crate::net::Session::join(invite,"Wing",1).unwrap()));
            FUTURE_JOINED.with(|t|*t.borrow_mut()=Some(std::time::Instant::now()));
        }
        5..=15=>{
            FUTURE_PEER.with(|p|{let mut peer=p.borrow_mut();let peer=peer.as_mut().unwrap();peer.poll().unwrap();peer.set_ready(true,1).unwrap();});
            app.network_poll();
        }
        16=>{
            FUTURE_PEER.with(|p|{let mut peer=p.borrow_mut();let peer=peer.as_mut().unwrap();peer.poll().unwrap();peer.set_ready(true,1).unwrap();});
            app.network_poll();
            let session=app.multiplayer.session.as_ref().unwrap();
            if session.members.len()!=2 || session.members.iter().any(|m|!m.ready) {
                assert!(FUTURE_JOINED.with(|t|t.borrow().unwrap().elapsed().as_secs_f32()<5.0),"loopback peer did not join and become ready within five seconds");
                FUTURE_DELAY.with(|v|v.set(v.get()+1));
                return false;
            }
            assert_eq!(session.members.len(),2);select(app,Action::NetLaunch,2);
        }
        17=>{assert!(app.multiplayer.active());assert_eq!(app.state,State::Flying);key(app,KeyCode::Escape);}
        18=>{
            assert!(app.controls_open);assert!(!app.buttons.iter().any(|b|b.action==Action::Restart));
            let actors=app.combat.bandits.len();app.act(Action::Restart);assert_eq!(app.combat.bandits.len(),actors);
            select(app,Action::Online,2);
        }
        19=>{
            assert_eq!(app.state,State::Network);assert!(!app.controls_open);bounds(app);
            assert!(!app.buttons.iter().any(|b|matches!(b.action,Action::Jet|Action::NetEdit(_))));
            select(app,Action::NetResume,1);
        }
        20=>{assert_eq!(app.state,State::Flying);key(app,KeyCode::Escape);}
        21=>{select(app,Action::NetLeave,0);}
        22=>{
            assert!(app.multiplayer.session.is_none());assert!(!app.multiplayer.voice.capturing());assert!(!app.controls_open);
            FUTURE_PEER.with(|p|{p.borrow_mut().take();});
            key(app,KeyCode::Escape);assert_eq!(app.state,State::Attract);
        }
        23=>{
            let hub=crate::net::Directory::bind("127.0.0.1:0".parse().unwrap()).unwrap();
            app.multiplayer.menu.server=hub.local_addr().unwrap().to_string();
            let host=crate::net::Session::host("127.0.0.1:0".parse().unwrap(),Some(hub.local_addr().unwrap()),true,"BrowserHost",2,crate::net::Rules::default()).unwrap();
            FUTURE_HUB.with(|h|*h.borrow_mut()=Some(hub));
            FUTURE_HOST.with(|h|*h.borrow_mut()=Some(host));
            FUTURE_JOINED.with(|t|*t.borrow_mut()=Some(std::time::Instant::now()));
        }
        24=>{
            let registered=FUTURE_HOST.with(|h|h.borrow().as_ref().unwrap().status=="ROOM ONLINE, TCP RELAY");
            if !registered {
                assert!(FUTURE_JOINED.with(|t|t.borrow().unwrap().elapsed().as_secs_f32()<5.0));
                FUTURE_DELAY.with(|d|d.set(d.get()+1));return false;
            }
            app.multiplayer.menu.panel=crate::multiplayer::Panel::Browser;
            select(app,Action::Online,2);
        }
        25=>{
            let found=app.multiplayer.menu.browser.as_ref().is_some_and(|b|b.rooms.len()==1);
            if !found {
                assert!(FUTURE_JOINED.with(|t|t.borrow().unwrap().elapsed().as_secs_f32()<5.0),"browser did not discover public room");
                FUTURE_DELAY.with(|d|d.set(d.get()+1));return false;
            }
            bounds(app);
            select(app,Action::NetRoom(0),1);
        }
        26=>{
            if app.multiplayer.session.as_ref().unwrap().members.len()!=2 {
                assert!(FUTURE_JOINED.with(|t|t.borrow().unwrap().elapsed().as_secs_f32()<5.0));
                FUTURE_DELAY.with(|d|d.set(d.get()+1));return false;
            }
            bounds(app);select(app,Action::NetReady,2);
        }
        27=>{
            let ready=FUTURE_HOST.with(|h|h.borrow().as_ref().unwrap().members.iter().all(|m|m.ready));
            if !ready {
                assert!(FUTURE_JOINED.with(|t|t.borrow().unwrap().elapsed().as_secs_f32()<5.0));
                FUTURE_DELAY.with(|d|d.set(d.get()+1));return false;
            }
            FUTURE_HOST.with(|h|h.borrow_mut().as_mut().unwrap().launch().unwrap());
        }
        28=>{
            if !app.multiplayer.active() {
                assert!(FUTURE_JOINED.with(|t|t.borrow().unwrap().elapsed().as_secs_f32()<5.0));
                FUTURE_DELAY.with(|d|d.set(d.get()+1));return false;
            }
            assert!(!app.multiplayer.voice.capturing());key(app,KeyCode::Escape);
        }
        29=>select(app,Action::Online,0),
        30=>select(app,Action::NetLeave,1),
        31=>{
            assert!(app.multiplayer.session.is_none());
            FUTURE_HOST.with(|h|h.borrow_mut().take());FUTURE_HUB.with(|h|h.borrow_mut().take());
            key(app,KeyCode::Escape);assert_eq!(app.state,State::Attract);
            println!("PASS public server browser, controller join, TCP lobby ready, launch and leave");
        }
        _=>{
            println!("PASS flight school lessons through click, Enter and controller A");
            println!("PASS online fields, private hosting, launch gating, leaving and microphone disabled");
            println!("PASS future menu integration check complete");return true;
        }
    }false
}

/// Offline fixture states for long labels, full lobbies and all drawer variants.
/// Rendering these never calls voice tick or starts microphone capture.
pub(super) fn layout_scene(app:&mut App,scene:&str) {
    use crate::{net,settings,voice,tutorial};
    let known=matches!(scene,"keys"|"mouse"|"classic-invert"|"rebind"|"school-passed"|"fuel-min"|"lesson-passed"|"lesson-failed"|"lobby"|"lobby-client"|"lobby-voice"|"browser"|"fields"|"create"|"lan"|"browser-empty"|"server-error")
        ||scene.starts_with("training-")||scene.starts_with("threats-")||scene.starts_with("network-");
    assert!(known,"unknown layout scene: {scene}");
    if scene=="create"||scene.starts_with("network-") { app.multiplayer.menu.panel=crate::multiplayer::Panel::Create; }
    if scene=="lan" { app.multiplayer.menu.panel=crate::multiplayer::Panel::Lan; }
    if scene=="server-error" { app.multiplayer.menu.status="SERVER UNREACHABLE: CONNECTION REFUSED. CHECK ADDRESS / TCP PORT, THEN REFRESH.".into(); }
    if let Some(mode)=scene.strip_prefix("network-") {
        app.multiplayer.menu.rules.mode=net::GameMode::parse(mode).expect("known multiplayer layout mode");
    }
    if let Some(stage)=scene.strip_prefix("training-").and_then(|n|n.parse::<usize>().ok()) {
        let training=app.training.as_mut().expect("use --tutorial with a training layout scene");
        assert!(stage<training.lesson.steps()); training.stage=stage;
    }
    if scene.contains("classic") {app.settings.pad_layout=settings::PadLayout::Classic;}
    if scene.contains("invert") {app.settings.invert_pitch=true;}
    if scene.contains("mouse") {app.settings.pitch=settings::PitchSource::Mouse;}
    if scene.contains("keys") {app.settings.pitch=settings::PitchSource::Keys;}
    if scene=="rebind" {app.rebinding=Some((0,0));}
    if scene=="school-passed" {app.settings.tutorial_completed=31;}
    if let Some(index)=scene.strip_prefix("threats-").and_then(|n|n.parse::<usize>().ok()) {
        app.config.threats=crate::scenario::Threats::with_layout(crate::scenario::Layout::ALL[index],app.kind,true);
        app.config.threats.bandits=crate::scenario::MAX_BANDITS;
        app.config.threats.sams=crate::scenario::MAX_SAMS;
        app.config.threats.guns=crate::scenario::MAX_GUNS;
    }
    if scene=="fuel-min" {for _ in 0..16 {if app.config.fuel_is_min(){break;} app.config.cycle_fuel();}}
    if scene.starts_with("lesson-") {
        let passed=scene.ends_with("passed");
        let a=app.aircraft.as_ref().expect("use --screen win for lesson results");
        let mut training=tutorial::Training::new(tutorial::Lesson::Weapons,a,&app.combat);
        if passed {training.stage=training.lesson.steps();}
        app.training=Some(training);
    }
    if scene.starts_with("lobby") {
        let mut session=net::Session::host("127.0.0.1:0".parse().unwrap(),None,false,"Longest-Callsign",0,net::Rules{capacity:8,..Default::default()}).unwrap();
        for i in 1..8 {session.members.push(net::Member{id:session.id+i,name:"Longest-Callsign".into(),jet:(i%4)as u8,ready:i%2==0,team:(i%2)as u8});}
        if scene=="lobby-client" {session.host=false;}
        app.multiplayer.session=Some(session);
    }
    if scene=="browser" || scene=="browser-empty" {
        let mut browser=net::Browser::new("127.0.0.1:9".parse().unwrap()).unwrap();
        for i in 0..if scene=="browser"{6}else{0} {browser.rooms.push(net::Advert{title:"Longest-Callsign".into(),endpoint:"127.0.0.1:9".parse().unwrap(),key:[0;32],mode:net::GameMode::ALL[i],players:8,capacity:8});}
        app.multiplayer.menu.browser=Some(browser);
    }
    if scene=="fields" {
        app.multiplayer.menu.panel=crate::multiplayer::Panel::Invite;
        app.multiplayer.menu.name="Longest-Callsign".into();
        app.multiplayer.menu.invite="0123456789ABCDEF".repeat(10);
        app.multiplayer.menu.server="long-directory-hostname.example.invalid:42861".into();
        app.multiplayer.menu.port="65535".into();
        app.multiplayer.menu.editing=Some(crate::multiplayer::Field::Invite);
        app.multiplayer.menu.status="A LONG STATUS MESSAGE THAT MUST STAY INSIDE THE PANEL. ".repeat(5);
    }
    if scene.contains("voice") {
        app.multiplayer.voice.configure(voice::VoiceSettings{mode:voice::VoiceMode::VoiceActivation,muted:true,deafened:true,..Default::default()});
        assert!(!app.multiplayer.voice.capturing());
    }
}
