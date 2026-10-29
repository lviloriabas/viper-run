//! Lobby, authoritative game integration and client prediction.
use crate::combat::network::{Control, Frame, RemotePilot};
use crate::combat::MissionKind;
use crate::sim::{Aircraft, Input, Type};
use crate::{combat, math::*, net, operation, voice, Action, App, State};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel { Browser, Create, Invite, Nearby }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Name,
    Invite,
    Server,
    PublicAddress,
}
pub struct Menu {
    pub panel: Panel,
    pub help: bool,
    pub filter: Option<net::GameMode>,
    pub name: String,
    pub invite: String,
    pub server: String,
    pub external_server: bool,
    pub public_address: String,
    pub editing: Option<Field>,
    before: String,
    pub replace_text: bool,
    pub rules: net::Rules,
    pub public: bool,
    pub status: String,
    pub browser: Option<net::Browser>,
    pub nearby: Option<net::Nearby>,
    pub muted: BTreeSet<u64>,
    pub gains: BTreeMap<u64, f32>,
}
impl Default for Menu {
    fn default() -> Self {
        Self {
            panel: Panel::Create,
            help: false,
            filter: None,
            name: "Pilot".into(),
            invite: String::new(),
            server: String::new(),
            external_server: false,
            public_address: String::new(),
            editing: None,
            before: String::new(),
            replace_text: false,
            rules: net::Rules::default(),
            public: true,
            status: "CREATE A GAME, OR JOIN YOUR FRIEND. NO SEPARATE SERVER NEEDED.".into(),
            browser: None,
            nearby: None,
            muted: BTreeSet::new(),
            gains: BTreeMap::new(),
        }
    }
}
impl Menu {
    pub fn text(&self, f: Field) -> &str {
        match f {
            Field::Name => &self.name,
            Field::Invite => &self.invite,
            Field::Server => &self.server,
            Field::PublicAddress => &self.public_address,
        }
    }
    fn text_mut(&mut self, f: Field) -> &mut String {
        match f {
            Field::Name => &mut self.name,
            Field::Invite => &mut self.invite,
            Field::Server => &mut self.server,
            Field::PublicAddress => &mut self.public_address,
        }
    }
    pub fn edit(&mut self, f: Field) {
        self.before = self.text(f).into();
        self.editing = Some(f);
        self.replace_text = true;
    }
    pub fn character(&mut self, ch: char) {
        if let Some(f) = self.editing {
            let allowed = match f {
                Field::Name => ch.is_ascii_alphanumeric() || " _-".contains(ch),
                _ => ch.is_ascii_graphic(),
            };
            let limit = match f {
                Field::Name => 16,
                _ => 160,
            };
            if allowed && self.replace_text { self.text_mut(f).clear(); self.replace_text = false; }
            if allowed && self.text(f).len() < limit {
                self.text_mut(f).push(ch);
            }
        }
    }
    pub fn backspace(&mut self) {
        if let Some(f) = self.editing {
            if self.replace_text { self.text_mut(f).clear(); self.replace_text = false; } else { self.text_mut(f).pop(); }
        }
    }
    pub fn finish(&mut self, cancel: bool) {
        if let Some(f) = self.editing.take() {
            if !cancel && f == Field::Server && self.before != self.server { self.browser = None; }
            if cancel {
                *self.text_mut(f) = self.before.clone();
            }
        }
    }
}
struct Remote {
    slot: usize,
    control: Control,
    pilot: RemotePilot,
    sequence: u32,
    age: f64,
    dead: f64,
    life: u32,
    connected: bool,
}
pub struct Battle {
    peers: BTreeMap<u64, Remote>,
    host: RemotePilot,
    host_dead: f64,
    host_life: u32,
    pub operation: operation::Operation,
    pub objective: String,
    pub finished: bool,
    pub tick: u32,
    last_frame: u32,
    pub scores: Vec<(u64, u8, u32, u32)>,
    send_clock: f64,
    frame_clock: f64,
    frames: u32,
    escort: Option<(usize, f64, V3)>,
    raiders: Vec<usize>,
    breaches: u32,
}
impl Battle {
    pub fn labels(&self, session: &net::Session, c: &combat::Combat) -> Vec<(V3, String)> {
        c.bandits
            .iter()
            .enumerate()
            .filter(|(_, b)| b.alive && b.team == c.player_team)
            .map(|(i, b)| {
                let escort = self.escort.is_some_and(|(slot, _, _)| slot == i);
                let name = if escort {
                    format!("ESCORT {:.0}%", b.hp)
                } else {
                    let id = self
                        .peers
                        .iter()
                        .find(|(_, r)| r.slot == i)
                        .map(|(&id, _)| id)
                        .unwrap_or(0);
                    session
                        .members
                        .iter()
                        .find(|m| m.id == id)
                        .map(|m| m.name.clone())
                        .unwrap_or_else(|| "ALLY".into())
                };
                (b.jet.pos, name)
            })
            .collect()
    }
}
pub struct Multiplayer {
    pub server: Option<net::LocalServer>,
    pub session: Option<net::Session>,
    pub battle: Option<Battle>,
    pub menu: Menu,
    pub voice: voice::VoiceChat,
    pub actions: [u32; 5],
    pub drop_lock: u32,
    sequence: u32,
    created: Instant,
}
impl Default for Multiplayer {
    fn default() -> Self {
        Self {
            server: None,
            session: None,
            battle: None,
            menu: Menu::default(),
            voice: voice::VoiceChat::new(),
            actions: [0; 5],
            drop_lock: 0,
            sequence: 0,
            created: Instant::now(),
        }
    }
}
impl Multiplayer {
    pub fn active(&self) -> bool {
        self.session.is_some() && self.battle.is_some()
    }
    pub fn disconnect(&mut self, audio: &crate::audio::Audio) {
        self.session = None;
        self.server = None;
        self.battle = None;
        self.voice.disconnect();
        self.voice.playback(audio);
        self.actions = [0; 5];
        self.drop_lock = 0;
        self.menu.muted.clear();
        self.menu.gains.clear();
    }
    pub fn action(&mut self, b: crate::keymap::Bind) -> bool {
        use crate::keymap::Bind;
        let index = match b {
            Bind::Weapon => 0,
            Bind::Lock => 1,
            Bind::Countermeasures => 2,
            Bind::Jettison => 3,
            Bind::Sensor => 4,
            _ => return false,
        };
        self.actions[index] = self.actions[index].wrapping_add(1);
        true
    }
    pub fn settings(&self) -> voice::VoiceSettings {
        self.voice.settings()
    }
    pub fn invitation(&self, internet: bool) -> Option<String> {
        let session = self.session.as_ref()?;
        match &self.server {
            Some(server) => {
                let address = if internet { server.internet_address()? } else { server.lan };
                Some(session.invitation_at(address))
            }
            None => Some(session.invitation()),
        }
    }
}
fn spawn(kind: Type, team: u8, slot: usize) -> Aircraft {
    let side = if team % 2 == 0 { -1.0 } else { 1.0 };
    combat::spawn_type(
        kind,
        v3(
            side * 6500.0,
            4300.0 + (slot % 3) as f64 * 150.0,
            -7500.0 + slot as f64 * 900.0,
        ),
        -side * 90.0 * DEG,
        220.0,
    )
}
fn mission(mode: net::GameMode) -> MissionKind {
    if mode == net::GameMode::Strike {
        MissionKind::Strike
    } else {
        MissionKind::Dogfight
    }
}
impl App {
    pub(super) fn network_action(&mut self, action: Action) -> bool {
        let result: Result<(), String> = (|| {
            match action {
                Action::Online => {
                    self.controls_open = false;
                    self.controls_was_flying = false;
                    self.multiplayer.menu.editing = None;
                    self.state = State::Network;
                    self.selected = 0;
                    self.grab(false);
                    if self.multiplayer.session.is_none() && self.multiplayer.menu.panel == Panel::Browser && !self.multiplayer.menu.server.is_empty() {
                        self.network_action(Action::NetBrowse);
                    }
                }
                Action::NetTab(panel) => {
                    self.multiplayer.menu.finish(false);
                    self.multiplayer.menu.panel = panel;
                    self.selected = 0;
                    if self.multiplayer.session.is_none() {
                        if panel == Panel::Nearby { self.network_action(Action::NetNearby); }
                        if panel == Panel::Browser && !self.multiplayer.menu.server.is_empty() { self.network_action(Action::NetBrowse); }
                    }
                }
                Action::NetFilter => {
                    let menu = &mut self.multiplayer.menu;
                    menu.filter = match menu.filter { None => Some(net::GameMode::ALL[0]), Some(net::GameMode::Defense) => None, Some(m) => Some(m.next()) };
                    if let Some(browser) = &mut menu.browser { browser.mode = menu.filter; browser.page = 0; browser.refresh(); }
                    if let Some(nearby) = &mut menu.nearby { nearby.mode = menu.filter; nearby.page = 0; }
                }
                Action::NetPage(next) => {
                    let menu = &mut self.multiplayer.menu;
                    if menu.panel == Panel::Nearby { if let Some(b) = &mut menu.nearby { b.turn_page(next); } }
                    else if let Some(b) = &mut menu.browser { b.turn_page(next); }
                }
                Action::NetHosting => self.multiplayer.menu.external_server = !self.multiplayer.menu.external_server,
                Action::NetHelp => self.multiplayer.menu.help = true,
                Action::NetHelpClose => self.multiplayer.menu.help = false,
                Action::NetNearby => {
                    if let Some(nearby) = &mut self.multiplayer.menu.nearby { nearby.refresh(); }
                    else { self.multiplayer.menu.nearby = Some(net::Nearby::new().map_err(|e| e.to_string())?); }
                    self.multiplayer.menu.status = "SEARCHING YOUR NETWORK. THE HOST CREATES A GAME FIRST.".into();
                }
                Action::NetEdit(f) => {
                    if self.multiplayer.session.is_none() || (f == Field::PublicAddress && self.multiplayer.server.is_some()) {
                        self.multiplayer.menu.edit(f);
                    }
                }
                Action::NetMode => {
                    if self.multiplayer.session.is_none() {
                        self.multiplayer.menu.rules.mode = self.multiplayer.menu.rules.mode.next();
                    }
                }
                Action::NetCapacity => {
                    let c = &mut self.multiplayer.menu.rules.capacity;
                    *c = if *c >= 8 { 2 } else { *c + 1 };
                }
                Action::NetPublic => self.multiplayer.menu.public = !self.multiplayer.menu.public,
                Action::NetHost => {
                    if self.multiplayer.session.is_some() {
                        return Ok(());
                    }
                    self.multiplayer.menu.finish(false);
                    let menu = &self.multiplayer.menu;
                    let server = if menu.external_server { None } else {
                        Some(if let Some(bind) = &self.opts.host_game {
                            let address = net::resolve(bind).map_err(|e| e.to_string())?;
                            net::LocalServer::bind(address, !address.ip().is_loopback())
                        } else { net::LocalServer::start() }.map_err(|e| format!("COULD NOT CREATE ROOM: {e}"))?)
                    };
                    let hub = Some(match &server {
                        Some(server) => server.connect_address(),
                        None => net::server_address(menu.server.trim()).map_err(|_| "ENTER YOUR RELAY SERVER ADDRESS FIRST")?,
                    });
                    let mut rules = menu.rules.clone();
                    rules.started = false;
                    let bind_ip = if self.opts.ui_check || self.opts.future_ui_check {
                        "127.0.0.1"
                    } else {
                        "0.0.0.0"
                    };
                    let s = net::Session::host(
                        format!("{bind_ip}:0").parse().unwrap(),
                        hub,
                        menu.public,
                        &menu.name,
                        self.config.jet.index() as u8,
                        rules,
                    )
                    .map_err(|e| e.to_string())?;
                    self.multiplayer.menu.invite = s.invitation();
                    self.multiplayer.menu.status = if server.is_some() { "GAME CREATED. SHARE AN INVITE OR ASK YOUR FRIEND TO OPEN NEARBY." } else { "CONNECTING TO RELAY. WAIT FOR ROOM ONLINE." }.into();
                    self.multiplayer.menu.browser = None;
                    self.multiplayer.menu.nearby = None;
                    self.multiplayer.menu.public_address.clear();
                    self.multiplayer.server = server;
                    self.multiplayer.session = Some(s);
                    self.save();
                }
                Action::NetJoin => {
                    if self.multiplayer.session.is_some() {
                        return Ok(());
                    }
                    self.multiplayer.menu.finish(false);
                    let menu = &self.multiplayer.menu;
                    let invite = net::Invite::parse(&menu.invite).map_err(|e| e.to_string())?;
                    let session = net::Session::join(invite, &menu.name, self.config.jet.index() as u8).map_err(|e| e.to_string())?;
                    self.multiplayer.menu.browser = None;
                    self.multiplayer.session = Some(session);
                    self.multiplayer.menu.status = "CONNECTING TO YOUR FRIEND...".into();
                }
                Action::NetPasteJoin => {
                    self.multiplayer.menu.invite = miniquad::window::clipboard_get()
                        .ok_or("COPY YOUR FRIEND'S INVITATION FIRST")?.trim().into();
                    self.network_action(Action::NetJoin);
                }
                Action::NetNearbyRoom(index) => {
                    if let Some(room) = self.multiplayer.menu.nearby.as_ref().and_then(|b| b.rooms.get(index as usize)).cloned() {
                        if room.players >= room.capacity { return Err("ROOM FULL. CHOOSE ANOTHER ROOM.".into()); }
                        self.multiplayer.menu.invite = net::Invite { endpoint: room.endpoint, key: room.key, relay: true, tcp: true }.text();
                        self.network_action(Action::NetJoin);
                    }
                }
                Action::NetBrowse => {
                    let server = net::server_address(self.multiplayer.menu.server.trim())
                        .map_err(|_| "ENTER RELAY HOST OR HOST:PORT FIRST")?;
                    let mut browser = net::Browser::new(server).map_err(|e| e.to_string())?;
                    browser.mode = self.multiplayer.menu.filter;
                    self.multiplayer.menu.browser = Some(browser);
                    self.multiplayer.menu.status = "CONNECTING TO SERVER...".into();
                    self.save();
                }
                Action::NetRoom(index) => {
                    let menu = &self.multiplayer.menu;
                    if let Some(advert) = menu
                        .browser
                        .as_ref()
                        .and_then(|b| b.rooms.get(index as usize))
                        .cloned()
                    {
                        if advert.players >= advert.capacity { return Err("ROOM FULL. CHOOSE ANOTHER ROOM.".into()); }
                        let endpoint = menu.browser.as_ref().unwrap().server;
                        self.multiplayer.menu.invite = net::Invite {
                            endpoint,
                            key: advert.key,
                            relay: true,
                            tcp: true,
                        }
                        .text();
                        self.network_action(Action::NetJoin);
                    }
                }
                Action::NetCopy(internet) => {
                    let invite = self.multiplayer.invitation(internet).ok_or("INTERNET SETUP IS NOT AVAILABLE. USE LAN OR AN EXTERNAL RELAY.")?;
                    miniquad::window::clipboard_set(&invite);
                    self.multiplayer.menu.status = if self.multiplayer.server.is_some() {
                        if internet { "INTERNET INVITATION COPIED. SEND IT TO YOUR FRIEND." }
                        else { "LAN INVITATION COPIED. BOTH PCS MUST USE THE SAME NETWORK." }
                    } else { "INVITATION COPIED. SEND IT TO YOUR FRIEND." }.into();
                }
                Action::NetCopyManual => {
                    self.multiplayer.menu.finish(false);
                    let server = self.multiplayer.server.as_ref().ok_or("CREATE A GAME ON THIS PC FIRST")?;
                    let endpoint = server.manual_address(&self.multiplayer.menu.public_address).map_err(|e|e.to_string())?;
                    let session = self.multiplayer.session.as_ref().ok_or("CREATE A GAME FIRST")?;
                    miniquad::window::clipboard_set(&session.invitation_at(endpoint));
                    self.multiplayer.menu.status = "INTERNET INVITE COPIED. SEND TO YOUR FRIEND AFTER FORWARDING THE TCP PORT.".into();
                }
                Action::NetCopySetup => {
                    let server = self.multiplayer.server.as_ref().ok_or("CREATE A GAME ON THIS PC FIRST")?;
                    miniquad::window::clipboard_set(&server.setup_instructions());
                    self.multiplayer.menu.status = "ROUTER INSTRUCTIONS COPIED, INCLUDING THIS PC'S IP AND TCP PORT.".into();
                }
                Action::NetReady => {
                    if let Some(s) = &mut self.multiplayer.session {
                        let ready = s
                            .members
                            .iter()
                            .find(|m| m.id == s.id)
                            .is_none_or(|m| !m.ready);
                        s.set_ready(ready, self.config.jet.index() as u8)
                            .map_err(|e| e.to_string())?;
                    }
                }
                Action::NetLaunch => {
                    if let Some(s) = &mut self.multiplayer.session {
                        s.launch().map_err(|e| e.to_string())?;
                    }
                }
                Action::NetResume => {
                    self.controls_open = false;
                    self.controls_was_flying = false;
                    if self.multiplayer.active() {
                        self.state = State::Flying;
                        self.grab(true);
                        self.keys.clear();
                    }
                }
                Action::NetLeave => {
                    self.controls_open = false;
                    self.controls_was_flying = false;
                    self.multiplayer.disconnect(&self.audio);
                    self.multiplayer.menu.status = "ROOM CLOSED. CREATE A GAME OR JOIN YOUR FRIEND.".into();
                    self.state = State::Network;
                    self.grab(false);
                }
                Action::VoiceMode => {
                    let mut s = self.multiplayer.settings();
                    s.mode = match s.mode {
                        voice::VoiceMode::Disabled => voice::VoiceMode::PushToTalk,
                        voice::VoiceMode::PushToTalk => voice::VoiceMode::VoiceActivation,
                        voice::VoiceMode::VoiceActivation => voice::VoiceMode::Disabled,
                    };
                    self.multiplayer.voice.configure(s);
                }
                Action::VoiceMute => {
                    let mut s = self.multiplayer.settings();
                    s.muted = !s.muted;
                    self.multiplayer.voice.configure(s);
                }
                Action::VoiceDeafen => {
                    let mut s = self.multiplayer.settings();
                    s.deafened = !s.deafened;
                    self.multiplayer.voice.configure(s);
                }
                Action::VoiceInput(up) => {
                    let mut s = self.multiplayer.settings();
                    s.input_gain = (s.input_gain + if up { 0.25 } else { -0.25 }).clamp(0.0, 4.0);
                    self.multiplayer.voice.configure(s);
                }
                Action::VoiceOutput(up) => {
                    let mut s = self.multiplayer.settings();
                    s.output_gain = (s.output_gain + if up { 0.25 } else { -0.25 }).clamp(0.0, 2.0);
                    self.multiplayer.voice.configure(s);
                }
                Action::VoiceThreshold(up) => {
                    let mut s = self.multiplayer.settings();
                    s.activation_threshold = (s.activation_threshold
                        + if up { 0.005 } else { -0.005 })
                    .clamp(0.005, 0.15);
                    self.multiplayer.voice.configure(s);
                }
                Action::VoicePeer(id) => {
                    let muted = if self.multiplayer.menu.muted.remove(&id) {
                        false
                    } else {
                        self.multiplayer.menu.muted.insert(id);
                        true
                    };
                    self.multiplayer.voice.set_peer_muted(id, muted);
                }
                Action::VoicePeerGain(id) => {
                    let gain = self.multiplayer.menu.gains.entry(id).or_insert(1.0);
                    *gain = if *gain >= 2.0 { 0.25 } else { *gain + 0.25 };
                    self.multiplayer.voice.set_peer_gain(id, *gain);
                }
                _ => return Err("__UNHANDLED__".into()),
            }
            Ok(())
        })();
        match result {
            Ok(()) => true,
            Err(e) if e == "__UNHANDLED__" => false,
            Err(e) => {
                self.multiplayer.menu.status = e;
                true
            }
        }
    }
    pub(super) fn network_options(&mut self) {
        if self.opts.host.is_none() && self.opts.host_game.is_none() && self.opts.join.is_none() {
            return;
        }
        self.state = State::Network;
        self.multiplayer.menu.name = self.opts.callsign.clone().unwrap_or_else(|| "Pilot".into());
        self.multiplayer.menu.server = self.opts.directory.clone().unwrap_or_default();
        if let Some(mode) = self.opts.online_mode {
            self.multiplayer.menu.rules.mode = mode;
        }
        if self.opts.host_game.is_some() {
            self.network_action(Action::NetHost);
            if let (Some(path), Some(invite)) = (&self.opts.invite_file, self.multiplayer.invitation(false)) {
                if let Err(e) = std::fs::write(path, invite) { self.multiplayer.menu.status = e.to_string(); }
            }
        } else if let Some(bind) = &self.opts.host {
            let result = (|| -> std::io::Result<net::Session> {
                let hub = self
                    .opts
                    .directory
                    .as_ref()
                    .map(|a| net::resolve(a))
                    .transpose()?;
                net::Session::host(
                    net::resolve(bind)?,
                    hub,
                    false,
                    &self.multiplayer.menu.name,
                    self.config.jet.index() as u8,
                    self.multiplayer.menu.rules.clone(),
                )
            })();
            match result {
                Ok(session) => {
                    if let Some(path) = &self.opts.invite_file {
                        if let Err(e) = std::fs::write(path, session.invitation()) {
                            self.multiplayer.menu.status = e.to_string();
                        }
                    }
                    self.multiplayer.menu.invite = session.invitation();
                    self.multiplayer.session = Some(session);
                }
                Err(e) => self.multiplayer.menu.status = e.to_string(),
            }
        } else if let Some(invite) = &self.opts.join {
            self.multiplayer.menu.invite = invite.clone();
            self.network_action(Action::NetJoin);
        }
    }
    fn network_report(&self) -> String {
        let Some(s) = &self.multiplayer.session else {
            return format!("started=false\nerror={}\n", self.multiplayer.menu.status);
        };
        let Some(b) = &self.multiplayer.battle else {
            return format!(
                "started=false\nhost={}\nplayers={}\nstatus={}\n",
                s.host,
                s.members.len(),
                self.multiplayer.menu.status
            );
        };
        let commands = b.peers.values().map(|r| r.sequence as u64).sum::<u64>();
        let a = self.aircraft.as_ref().unwrap();
        let wave = b.operation.wave();
        let blue = b.operation.team_scores().get(&0).map_or(0, |s| s.kills);
        let red = b.operation.team_scores().get(&1).map_or(0, |s| s.kills);
        format!("started=true\nhost={}\nplayers={}\ntick={}\nframes={}\ncommands={}\njet={}\nx={:.3}\ny={:.3}\nz={:.3}\nhp={:.1}\nrounds={}\nmic_capturing={}\nhost_wave={}\nblue_kills={}\nred_kills={}\nobjective={}\nstatus={}\nembedded_server={}\n",
            s.host,s.members.len(),if s.host{b.tick}else{b.last_frame},b.frames,commands,a.kind.name(),a.pos.x,a.pos.y,a.pos.z,self.combat.hp,self.combat.loadout.rounds,self.multiplayer.voice.capturing(),wave,blue,red,b.objective,self.multiplayer.menu.status,self.multiplayer.server.is_some())
    }
    pub(super) fn network_poll(&mut self) {
        if let Some(server) = &mut self.multiplayer.server {
            if let Err(e) = server.poll() {
                self.multiplayer.disconnect(&self.audio);
                self.multiplayer.menu.status = format!("ROOM SERVER STOPPED: {e}. CREATE A NEW GAME.");
                self.state = State::Network;
                self.grab(false);
                return;
            }
            let menu=&mut self.multiplayer.menu;
            if menu.public_address.is_empty() && menu.editing!=Some(Field::PublicAddress) && !server.checking_internet() {
                if let Some(ip)=server.detected_public_ip {
                    menu.public_address=server.internet_address().unwrap_or(std::net::SocketAddr::new(ip,server.lan.port())).to_string();
                }
            }
        }
        if self.multiplayer.session.is_none() && self.multiplayer.menu.panel == Panel::Nearby {
            if let Some(nearby) = &mut self.multiplayer.menu.nearby {
                if let Err(e) = nearby.poll() { self.multiplayer.menu.status = format!("NEARBY SEARCH FAILED: {e}. TRY AN INVITATION."); }
            }
        }
        if self.multiplayer.menu.panel == Panel::Browser { if let Some(browser) = &mut self.multiplayer.menu.browser {
            match browser.poll() {
                Ok(()) => self.multiplayer.menu.status = browser.status(),
                Err(e) => {
                    self.multiplayer.menu.status = format!("SERVER UNREACHABLE: {e}. CHECK ADDRESS / TCP PORT, THEN REFRESH.");
                    self.multiplayer.menu.browser = None;
                }
            }
        } }
        let Some(mut session) = self.multiplayer.session.take() else {
            return;
        };
        let events = match session.poll() {
            Ok(e) => e,
            Err(e) => {
                self.multiplayer.menu.status = format!("CANNOT CONNECT: {e}. CHECK THE INVITE AND ALLOW THE HOST ON ITS NETWORK.");
                self.multiplayer.disconnect(&self.audio);
                self.state = State::Network;
                self.grab(false);
                return;
            }
        };
        for event in events {
            match event {
                net::Event::Joined(peer, name, jet) => {
                    self.multiplayer.voice.set_peer_gain(peer, 1.0);
                    self.multiplayer.menu.status =
                        format!("{name} JOINED IN {}", Type::ALL[jet as usize].name());
                }
                net::Event::Disconnected(peer) => {
                    self.multiplayer.voice.remove_peer(peer);
                    if let Some(b) = &mut self.multiplayer.battle {
                        if peer == 0 && !session.host {
                            b.finished = true;
                            b.objective = "HOST DISCONNECTED. RETURN TO MAIN MENU.".into();
                        }
                        if let Some(r) = b.peers.get_mut(&peer) {
                            self.combat.bandits[r.slot].alive = false;
                            r.control = r.control.idle();
                            r.connected = false;
                        }
                    }
                    self.multiplayer.menu.status = if peer == 0 {
                        "HOST DISCONNECTED".into()
                    } else {
                        "PILOT DISCONNECTED".into()
                    };
                }
                net::Event::Data(
                    peer,
                    net::Message::Voice {
                        speaker,
                        sequence,
                        data,
                    },
                ) => {
                    let valid = if session.host {
                        speaker == peer
                    } else {
                        peer == 0 && session.members.iter().any(|m| m.id == speaker)
                    };
                    if valid && speaker != session.id {
                        self.multiplayer.voice.receive(speaker, sequence, &data);
                        if session.host {
                            for to in session
                                .members
                                .iter()
                                .filter(|m| m.id != 0 && m.id != peer)
                                .map(|m| m.id)
                                .collect::<Vec<_>>()
                            {
                                let _ = session.send(
                                    to,
                                    &net::Message::Voice {
                                        speaker,
                                        sequence,
                                        data: data.clone(),
                                    },
                                );
                            }
                        }
                    }
                }
                net::Event::Data(peer, net::Message::Input { sequence, bytes }) if session.host => {
                    if let Some(remote) = self
                        .multiplayer
                        .battle
                        .as_mut()
                        .and_then(|b| b.peers.get_mut(&peer))
                    {
                        if sequence.wrapping_sub(remote.sequence) < u32::MAX / 2
                            && sequence != remote.sequence
                        {
                            if let Ok(c) = postcard::from_bytes::<Control>(&bytes) {
                                if c.valid() {
                                    remote.control = c;
                                    remote.sequence = sequence;
                                    remote.age = 0.0;
                                }
                            }
                        }
                    }
                }
                net::Event::Data(0, net::Message::Frame { tick, bytes }) if !session.host => {
                    if let (Some(b), Some(a)) = (&mut self.multiplayer.battle, &mut self.aircraft) {
                        if tick.wrapping_sub(b.last_frame) < u32::MAX / 2 && tick != b.last_frame {
                            if let Ok(frame) = postcard::from_bytes::<Frame>(&bytes) {
                                if self.combat.network_apply_frame(
                                    a,
                                    &mut self.mission,
                                    &frame,
                                    session.id,
                                ) {
                                    b.last_frame = tick;
                                    b.frames += 1;
                                    b.scores = frame.scores;
                                    b.objective = frame.objective;
                                    b.finished = frame.finished;
                                }
                            }
                        }
                    }
                }
                net::Event::Data(_, net::Message::Error(e)) => self.multiplayer.menu.status = e,
                _ => {}
            }
        }
        if self.opts.ready && !session.rules.started && !session.host {
            let _ = session.set_ready(true, self.config.jet.index() as u8);
        }
        if self.opts.launch_ready
            && session.host
            && !session.rules.started
            && session.members.len() >= 2
            && session.members.iter().all(|m| m.ready)
        {
            let _ = session.launch();
        }
        self.multiplayer.session = Some(session);
        if self
            .opts
            .exit_after
            .is_some_and(|t| self.multiplayer.created.elapsed().as_secs_f64() > t)
        {
            if let Some(path) = &self.opts.net_report {
                let report = self.network_report();
                let _ = std::fs::write(path, report);
            }
            miniquad::window::order_quit();
        }
        let should_begin = self
            .multiplayer
            .session
            .as_ref()
            .is_some_and(|s| s.rules.started)
            && self.multiplayer.battle.is_none();
        if should_begin {
            self.network_begin();
        }
        let focused = crate::capture::is_foreground()
            && self.opts.shot.is_none()
            && self.opts.bench.is_none();
        let ptt = self
            .settings
            .keys
            .held(crate::keymap::Bind::PushToTalk, &self.keys)
            || self.pad.held(crate::gamepad::BACK);
        self.multiplayer.voice.tick(ptt, focused);
        self.multiplayer.voice.playback(&self.audio);
        let packets = self.multiplayer.voice.drain_outgoing();
        if let Some(s) = &mut self.multiplayer.session {
            for p in packets {
                let msg = net::Message::Voice {
                    speaker: s.id,
                    sequence: p.seq,
                    data: p.data,
                };
                let result = if s.host {
                    s.broadcast(&msg)
                } else {
                    s.send(0, &msg)
                };
                if let Err(e) = result {
                    self.multiplayer.menu.status = e.to_string();
                }
            }
        }
    }
    fn network_begin(&mut self) {
        let s = self.multiplayer.session.as_ref().unwrap();
        let members = s.members.clone();
        let rules = s.rules.clone();
        let id = s.id;
        let host = s.host;
        self.multiplayer.actions = [0; 5];
        self.multiplayer.drop_lock = 0;
        self.training = None;
        self.opts.tutorial = None;
        self.kind = mission(rules.mode);
        let kind = members
            .iter()
            .find(|m| m.id == id)
            .map_or(Type::F16, |m| Type::ALL[m.jet as usize]);
        self.config = combat::Config::for_jet(self.kind, kind);
        self.start_mission();
        self.combat.network_clear_ai();
        self.combat.networked = true;
        self.combat.player_team = 0;
        self.combat.network_hostile(!rules.mode.pvp());
        self.mission.timed = false;
        self.mission.time_left = rules.minutes as f64 * 60.0;
        let mut battle = Battle {
            peers: BTreeMap::new(),
            host: RemotePilot::default(),
            host_dead: 0.0,
            host_life: 0,
            operation: operation::Operation::new(rules.mode, rules.minutes as f64 * 60.0),
            objective: rules.mode.brief().into(),
            finished: false,
            tick: 0,
            last_frame: 0,
            scores: Vec::new(),
            send_clock: 0.0,
            frame_clock: 0.0,
            frames: 0,
            escort: None,
            raiders: Vec::new(),
            breaches: 0,
        };
        let host_member = &members[0];
        *self.aircraft.as_mut().unwrap() =
            spawn(Type::ALL[host_member.jet as usize], host_member.team, 0);
        combat::load(self.aircraft.as_mut().unwrap(), &self.combat.loadout);
        for (i, m) in members.iter().enumerate() {
            battle.operation.register_actor(operation::Actor {
                id: m.id,
                team: m.team,
                life: 0,
            });
            if m.id == 0 {
                continue;
            }
            let slot = self.combat.network_spawn(
                spawn(Type::ALL[m.jet as usize], m.team, i),
                m.team,
                true,
                self.kind,
            );
            battle.peers.insert(
                m.id,
                Remote {
                    slot,
                    control: Control::default(),
                    pilot: RemotePilot::default(),
                    sequence: 0,
                    age: 0.0,
                    dead: 0.0,
                    life: 0,
                    connected: true,
                },
            );
        }
        if rules.mode == net::GameMode::Escort {
            let ally = combat::spawn_type(Type::F18, v3(-1000.0, 4300.0, 7000.0), 0.0, 190.0);
            let target = v3(0.0, 3000.0, -15000.0);
            let distance = (ally.pos - target).len();
            let slot = self
                .combat
                .network_spawn(ally, 0, true, MissionKind::Airfield);
            battle.escort = Some((slot, distance, target));
        }
        if host && !rules.mode.pvp() && rules.mode != net::GameMode::Defense {
            if matches!(rules.mode, net::GameMode::Combat | net::GameMode::Escort) {
                self.apply_directives(
                    &mut battle,
                    vec![operation::Directive::SpawnWave {
                        wave: 1,
                        fighters: 4,
                    }],
                );
            } else {
                battle.operation.wave_spawned(1);
            }
        }
        if host {
            let directives = battle.operation.update(
                0.0,
                operation::Observation {
                    enemies_alive: self
                        .combat
                        .bandits
                        .iter()
                        .filter(|b| b.alive && b.team == 1)
                        .count(),
                    primary_targets_remaining: self
                        .combat
                        .sites
                        .iter()
                        .filter(|s| s.primary && !s.dead)
                        .count(),
                    escort_alive: true,
                    escort_progress: 0.0,
                    base_health: 1.0,
                    breaches: 0,
                },
            );
            self.apply_directives(&mut battle, directives);
        }
        if self.opts.weapon == Some(combat::Mode::Gun) {
            self.combat.select(combat::Mode::Nav);
            self.multiplayer.actions[0] = 1;
        }
        self.multiplayer.battle = Some(battle);
        self.gear_lever = false;
        self.flaps_switch = false;
        self.flash_toast(rules.mode.name(), 3.0);
    }
    fn apply_directives(&mut self, battle: &mut Battle, directives: Vec<operation::Directive>) {
        for d in directives {
            if let operation::Directive::SpawnWave { wave, fighters } = d {
                let mode = self.multiplayer.session.as_ref().unwrap().rules.mode;
                for i in 0..fighters {
                    let raider = mode == net::GameMode::Defense && i + 1 < fighters;
                    let pos = if mode == net::GameMode::Defense {
                        v3(
                            5000.0 + i as f64 * 1600.0,
                            3900.0,
                            -6000.0 - wave as f64 * 1500.0,
                        )
                    } else {
                        v3(
                            5500.0 + i as f64 * 1800.0,
                            4500.0 + i as f64 * 150.0,
                            -12000.0 + wave as f64 * 800.0,
                        )
                    };
                    let jet = combat::spawn_type(
                        if i % 2 == 0 { Type::Mig29 } else { Type::Su27 },
                        pos,
                        150.0 * DEG,
                        230.0,
                    );
                    let slot = self
                        .combat
                        .network_spawn(jet, 1, raider, MissionKind::Dogfight);
                    if raider {
                        battle.raiders.push(slot);
                    }
                }
                battle.operation.wave_spawned(wave);
            }
        }
    }
    pub(super) fn network_step(&mut self, dt: f64, input: Input) {
        let Some(mut battle) = self.multiplayer.battle.take() else {
            return;
        };
        let Some(session) = self.multiplayer.session.as_ref() else {
            return;
        };
        let (host, mode) = (session.host, battle.operation.mode());
        let cm = self
            .settings
            .keys
            .held(crate::keymap::Bind::Countermeasures, &self.keys)
            || self.pad.held(crate::gamepad::LS);
        let eject = (self
            .settings
            .keys
            .held(crate::keymap::Bind::Eject, &self.keys)
            || self.ejection_chord())
            && self.state == State::Flying;
        let active = ((self.state == State::Flying && !self.controls_open)
            || self.opts.net_report.is_some())
            && !battle.finished;
        let mut control = Control::from_input(
            if active {
                input
            } else {
                Input {
                    gear_down: self.gear_lever,
                    flaps: self.flaps_switch,
                    ..Default::default()
                }
            },
            active && self.trigger(),
            active && cm && !eject,
            active && eject,
            self.multiplayer.actions,
            self.combat.helmet,
        );
        control.drop_lock = self.multiplayer.drop_lock;
        if !host {
            if !battle.finished && !self.combat.player_dead {
                if let Some(a) = &mut self.aircraft {
                    a.update(dt, &self.pilot.controls(a, control.input()));
                    self.pilot.update(dt, a);
                }
            }
            self.combat.network_visual_step(dt);
            battle.send_clock += dt;
            if battle.send_clock >= 1.0 / 60.0 {
                battle.send_clock = 0.0;
                self.multiplayer.sequence = self.multiplayer.sequence.wrapping_add(1);
                if let Ok(bytes) = postcard::to_stdvec(&control) {
                    let _ = self.multiplayer.session.as_mut().unwrap().send(
                        0,
                        &net::Message::Input {
                            sequence: self.multiplayer.sequence,
                            bytes,
                        },
                    );
                }
            }
        } else if !battle.finished {
            let a = self.aircraft.as_mut().unwrap();
            self.mission.elapsed += dt;
            battle.tick = battle.tick.wrapping_add(1);
            self.combat
                .network_controls(a, &control, &mut battle.host, dt);
            if !self.combat.player_dead {
                a.update(dt, &self.pilot.controls(a, control.input()));
                self.pilot.update(dt, a);
                if a.crashed || a.lost.any() || battle.host.ejected() {
                    self.combat.network_lose_player(a, "AIRCRAFT LOST");
                }
            }
            for r in battle.peers.values_mut() {
                if !r.connected {
                    continue;
                }
                r.age += dt;
                let c = if r.age > 0.3 {
                    r.control.idle()
                } else {
                    r.control.clone()
                };
                self.combat
                    .network_remote_step(r.slot, a, &c, &mut r.pilot, dt);
            }
            if let Some((slot, _, target)) = battle.escort {
                self.combat.network_route(slot, target, dt);
            }
            let base = v3(
                crate::terrain::RUNWAY_A.0,
                crate::terrain::base_elevation() + 200.0,
                crate::terrain::RUNWAY_A.1,
            );
            for &slot in &battle.raiders {
                self.combat.network_route(slot, base, dt);
                let b = &mut self.combat.bandits[slot];
                if b.alive && (b.jet.pos - base).len() < 2000.0 {
                    b.alive = false;
                    battle.breaches += 1;
                }
            }
            self.combat.step(
                dt,
                a,
                control.trigger() && !self.combat.player_dead,
                &mut self.mission,
            );
            self.combat.network_collisions(a);
            let to_actor = |w: crate::weapons::Who| -> operation::Actor {
                match w {
                    crate::weapons::Who::Player => operation::Actor {
                        id: 0,
                        team: 0,
                        life: battle.host_life,
                    },
                    crate::weapons::Who::Bandit(i) => battle
                        .peers
                        .iter()
                        .find(|(_, r)| r.slot == i)
                        .map(|(&id, r)| operation::Actor {
                            id,
                            team: self.combat.bandits[i].team,
                            life: r.life,
                        })
                        .unwrap_or(operation::Actor {
                            id: u64::MAX - i as u64,
                            team: self.combat.bandits[i].team,
                            life: 0,
                        }),
                    _ => operation::Actor {
                        id: u64::MAX,
                        team: 1,
                        life: 0,
                    },
                }
            };
            let deaths = std::mem::take(&mut self.combat.net_kills);
            for (killer, victim) in deaths {
                let (k, v) = (to_actor(killer), to_actor(victim));
                if k.id == v.id {
                    battle.operation.record_death(v);
                } else {
                    battle.operation.record_kill(k, v);
                }
            }
            if mode.pvp() {
                if self.combat.player_dead {
                    battle.host_dead += dt;
                    if battle.host_dead > 5.0 {
                        let kind = a.kind;
                        self.combat
                            .network_reset_player(a, spawn(kind, 0, 0), self.kind);
                        battle.host_dead = 0.0;
                        battle.host_life += 1;
                        battle.operation.register_actor(operation::Actor {
                            id: 0,
                            team: 0,
                            life: battle.host_life,
                        });
                        self.pilot = crate::physio::Pilot::default();
                    }
                }
                for (&id, r) in &mut battle.peers {
                    if r.connected && !self.combat.bandits[r.slot].alive {
                        r.dead += dt;
                        if r.dead > 5.0 {
                            let b = &self.combat.bandits[r.slot];
                            let (kind, team) = (b.jet.kind, b.team);
                            self.combat.network_reset_remote(
                                r.slot,
                                spawn(kind, team, r.slot + 1),
                                self.kind,
                            );
                            r.dead = 0.0;
                            r.life += 1;
                            r.pilot = RemotePilot::default();
                            r.pilot.actions = r.control.actions;
                            battle.operation.register_actor(operation::Actor {
                                id,
                                team,
                                life: r.life,
                            });
                        }
                    }
                }
            }
            let alive = usize::from(!self.combat.player_dead)
                + battle
                    .peers
                    .values()
                    .filter(|r| self.combat.bandits[r.slot].alive)
                    .count();
            if !mode.pvp() && alive == 0 {
                battle.operation.fail(operation::Reason::AllPilotsLost);
            }
            let (escort_alive, escort_progress) =
                battle.escort.map_or((true, 0.0), |(i, total, target)| {
                    let b = &self.combat.bandits[i];
                    (
                        b.alive,
                        if (b.jet.pos - target).len() < 1000.0 {
                            1.0
                        } else {
                            (1.0 - (b.jet.pos - target).len() / total).clamp(0.0, 1.0) as f32
                        },
                    )
                });
            let observation = operation::Observation {
                enemies_alive: self
                    .combat
                    .bandits
                    .iter()
                    .filter(|b| b.alive && b.team == 1)
                    .count(),
                primary_targets_remaining: self
                    .combat
                    .sites
                    .iter()
                    .filter(|s| s.primary && !s.dead)
                    .count(),
                escort_alive,
                escort_progress,
                base_health: (1.0 - battle.breaches as f32 / 3.0).max(0.0),
                breaches: battle.breaches,
            };
            let directives = battle.operation.update(dt, observation);
            battle.objective = battle.operation.summary();
            battle.scores = battle
                .operation
                .scores()
                .iter()
                .map(|(&id, s)| (id, s.team, s.kills, s.deaths))
                .collect();
            if let Some(operation::Outcome::Victory {
                actor: Some(id), ..
            }) = battle.operation.outcome()
            {
                if let Some(name) = self
                    .multiplayer
                    .session
                    .as_ref()
                    .and_then(|s| s.members.iter().find(|m| m.id == id))
                    .map(|m| m.name.clone())
                {
                    battle.objective = format!("{name} WINS THE DOGFIGHT");
                }
            }
            battle.finished = battle.operation.outcome().is_some();
            if mode.pvp() && self.multiplayer.session.as_ref().unwrap().members.len() < 2 {
                battle.finished = true;
                battle.objective = "OTHER PILOTS LEFT. SORTIE ENDED.".into();
            }
            self.mission.time_left = battle.operation.remaining();
            self.apply_directives(&mut battle, directives);
        }
        if host {
            battle.frame_clock += dt;
            if battle.frame_clock >= 1.0 / 20.0 {
                battle.frame_clock = 0.0;
                let peers = battle
                    .peers
                    .iter()
                    .map(|(&id, r)| (id, r.slot, &r.pilot))
                    .collect::<Vec<_>>();
                let mut frame = self.combat.network_frame(
                    self.aircraft.as_ref().unwrap(),
                    &self.mission,
                    battle.tick,
                    &peers,
                    battle.objective.clone(),
                    battle.finished,
                );
                frame.scores = battle.scores.clone();
                match postcard::to_stdvec(&frame) {
                    Ok(bytes) => {
                        if let Err(e) = self.multiplayer.session.as_mut().unwrap().broadcast(
                            &net::Message::Frame {
                                tick: battle.tick,
                                bytes,
                            },
                        ) {
                            self.multiplayer.menu.status = e.to_string();
                        }
                    }
                    Err(e) => self.multiplayer.menu.status = e.to_string(),
                }
            }
        }
        self.multiplayer.battle = Some(battle);
        let feedback = crate::props::present(
            &mut self.combat,
            &mut self.world.fx,
            &self.audio,
            self.camera.eye,
            self.settings.muted,
        );
        if let Some(text) = feedback.toast {
            self.flash_toast(&text, 2.0);
        }
        if let Some(color) = feedback.flash {
            self.flash_color = color;
            self.flash = self.flash.max(0.3);
        }
    }
}

#[cfg(test)]
mod menu_tests {
    use super::*;
    #[test]
    fn editing_replaces_old_text_and_escape_restores_it() {
        let mut menu=Menu::default();
        menu.edit(Field::Name);
        for c in "Wing".chars(){menu.character(c);}
        assert_eq!(menu.name,"Wing");
        menu.finish(true);assert_eq!(menu.name,"Pilot");
        menu.edit(Field::Server);
        for c in "127.0.0.1:42861".chars(){menu.character(c);}
        menu.finish(false);assert_eq!(menu.server,"127.0.0.1:42861");
        menu.edit(Field::Server);menu.backspace();assert!(menu.server.is_empty());
    }
}
