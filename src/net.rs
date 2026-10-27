//! Encrypted direct UDP sessions and an outbound TCP directory/relay.
//! All receive work, packet sizes, reassembly memory and peer counts are bounded.
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

#[path = "net_tcp.rs"]
mod tcp;
pub const BROWSER_PAGE: usize = 6;
pub const PORT: u16 = 42860;
#[allow(dead_code)]
pub const HUB_PORT: u16 = 42861;
pub const MAX_PEERS: usize = 8;
const MTU: usize = 1200;
const HEADER: usize = 50;
const CHUNK: usize = 960;
const MAX_MESSAGE: usize = 60 * CHUNK;
const TIMEOUT: Duration = Duration::from_secs(12);
const MAGIC: &[u8; 4] = b"VPR3";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GameMode {
    Dogfight,
    Teams,
    Combat,
    Strike,
    Escort,
    Defense,
}
impl GameMode {
    pub const ALL: [Self; 6] = [
        Self::Dogfight,
        Self::Teams,
        Self::Combat,
        Self::Strike,
        Self::Escort,
        Self::Defense,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Dogfight => "PVP DOGFIGHT",
            Self::Teams => "PVP TEAMS",
            Self::Combat => "COOP COMBAT",
            Self::Strike => "COOP STRIKE",
            Self::Escort => "COOP ESCORT",
            Self::Defense => "COOP AIR DEFENSE",
        }
    }
    pub fn brief(self) -> &'static str {
        match self {
        Self::Dogfight => "Every pilot for themselves. First to five kills wins.",
        Self::Teams => "Two teams fight for air superiority. First team to ten kills wins.",
        Self::Combat => "Work together to destroy the attacking fighter flight.",
        Self::Strike => "Destroy the bridge and SAM site together. Survive the air defenses.",
        Self::Escort => "Protect the allied aircraft until it reaches the coast. Intercept incoming fighters.",
        Self::Defense => "Defend the airfield against three waves. Stop attackers before they reach it.",
    }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "dogfight" => Some(Self::Dogfight),
            "teams" => Some(Self::Teams),
            "combat" => Some(Self::Combat),
            "strike" => Some(Self::Strike),
            "escort" => Some(Self::Escort),
            "defense" => Some(Self::Defense),
            _ => None,
        }
    }
    pub fn pvp(self) -> bool {
        matches!(self, Self::Dogfight | Self::Teams)
    }
    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|&m| m == self).unwrap();
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rules {
    pub mode: GameMode,
    pub capacity: u8,
    pub minutes: u16,
    pub started: bool,
}
impl Default for Rules {
    fn default() -> Self {
        Self {
            mode: GameMode::Combat,
            capacity: 4,
            minutes: 15,
            started: false,
        }
    }
}
impl Rules {
    fn valid(&self) -> bool {
        (2..=8).contains(&self.capacity) && (1..=60).contains(&self.minutes)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Member {
    pub id: u64,
    pub name: String,
    pub jet: u8,
    pub ready: bool,
    pub team: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Message {
    Hello {
        name: String,
        jet: u8,
    },
    Lobby {
        rules: Rules,
        members: Vec<Member>,
    },
    Ready {
        ready: bool,
        jet: u8,
    },
    Input {
        sequence: u32,
        bytes: Vec<u8>,
    },
    Frame {
        tick: u32,
        bytes: Vec<u8>,
    },
    Voice {
        speaker: u64,
        sequence: u32,
        data: Vec<u8>,
    },
    Ping(u64),
    Pong(u64),
    Leave,
    Error(String),
}
impl Message {
    fn valid(&self) -> bool {
        match self {
            Self::Hello { name, jet } => valid_name(name) && *jet < 4,
            Self::Lobby { rules, members } => {
                rules.valid()
                    && members.len() <= MAX_PEERS
                    && members.iter().all(|m| valid_name(&m.name) && m.jet < 4)
            }
            Self::Ready { jet, .. } => *jet < 4,
            Self::Voice { data, .. } => data.len() <= 512,
            Self::Input { bytes, .. } => bytes.len() <= 256,
            Self::Frame { bytes, .. } => bytes.len() < MAX_MESSAGE - 128,
            Self::Error(s) => s.len() <= 120,
            _ => true,
        }
    }
}
pub fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 16
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b" _-".contains(&b))
}
fn err(s: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, s)
}
pub fn resolve(s: &str) -> io::Result<SocketAddr> {
    let addresses: Vec<_> = s.to_socket_addrs()?.collect();
    addresses
        .iter()
        .find(|a| a.is_ipv4())
        .copied()
        .or_else(|| addresses.first().copied())
        .ok_or_else(|| err("address has no IP"))
}
pub fn server_address(s: &str) -> io::Result<SocketAddr> {
    let s = s.trim();
    if s.is_empty() {
        return Err(err("enter the relay server address"));
    }
    if let Ok(ip) = s.parse::<std::net::IpAddr>() {
        return Ok(SocketAddr::new(ip, HUB_PORT));
    }
    resolve(s).or_else(|_| resolve(&format!("{s}:{HUB_PORT}")))
}
fn random<const N: usize>() -> io::Result<[u8; N]> {
    let mut a = [0; N];
    getrandom::getrandom(&mut a).map_err(|_| err("system random source unavailable"))?;
    Ok(a)
}
fn hex(a: &[u8]) -> String {
    a.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex<const N: usize>(s: &str) -> io::Result<[u8; N]> {
    if s.len() != 2 * N || !s.is_ascii() {
        return Err(err("invalid invitation key"));
    }
    let mut out = [0; N];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[2 * i..2 * i + 2], 16)
            .map_err(|_| err("invalid invitation key"))?;
    }
    Ok(out)
}
#[derive(Clone, Debug)]
pub struct Invite {
    pub endpoint: SocketAddr,
    pub key: [u8; 32],
    pub relay: bool,
    pub tcp: bool,
}
impl Invite {
    pub fn parse(s: &str) -> io::Result<Self> {
        let (address, key) = s
            .trim()
            .split_once('#')
            .ok_or_else(|| err("use address:port#invitation-key"))?;
        let (tcp, address) = address
            .strip_prefix("relay+tcp:")
            .map(|s| (true, s))
            .unwrap_or((false, address));
        let (relay, address) = address
            .strip_prefix("relay:")
            .map(|s| (true, s))
            .unwrap_or((false, address));
        Ok(Self {
            endpoint: resolve(address)?,
            key: unhex(key)?,
            relay: relay || tcp,
            tcp,
        })
    }
    pub fn text(&self) -> String {
        format!(
            "{}{}#{}",
            if self.tcp {
                "relay+tcp:"
            } else if self.relay {
                "relay:"
            } else {
                ""
            },
            self.endpoint,
            hex(&self.key)
        )
    }
    pub fn room(&self) -> [u8; 8] {
        self.key[..8].try_into().unwrap()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Advert {
    pub title: String,
    pub endpoint: SocketAddr,
    pub key: [u8; 32],
    pub mode: GameMode,
    pub players: u8,
    pub capacity: u8,
}
impl Advert {
    fn valid(&self) -> bool {
        valid_name(&self.title)
            && (2..=8).contains(&self.capacity)
            && (1..=self.capacity).contains(&self.players)
    }
}
#[derive(Serialize, Deserialize)]
enum Hub {
    Register {
        room: [u8; 8],
        lease: [u8; 16],
        public: Option<Advert>,
    },
    Unregister {
        room: [u8; 8],
        lease: [u8; 16],
    },
    Browse {
        nonce: u64,
        mode: Option<GameMode>,
        padding: Vec<u8>,
    },
    Listings {
        nonce: u64,
        rooms: Vec<Advert>,
    },
    Relay {
        room: [u8; 8],
        peer: u64,
        target: u64,
        bytes: Vec<u8>,
    },
    BrowsePage {
        nonce: u64,
        mode: Option<GameMode>,
        page: u8,
    },
    Page {
        nonce: u64,
        page: u8,
        pages: u8,
        rooms: Vec<Advert>,
    },
    Registered,
    Failure(String),
    RoomMissing,
}
#[cfg_attr(not(test), allow(dead_code))]
struct DirectoryRoom {
    endpoint: SocketAddr,
    lease: [u8; 16],
    seen: Instant,
    public: Option<Advert>,
    peers: BTreeMap<u64, (SocketAddr, Instant)>,
}
#[cfg_attr(not(test), allow(dead_code))]
pub struct Directory {
    tcp: tcp::Relay,
    socket: UdpSocket,
    rooms: BTreeMap<[u8; 8], DirectoryRoom>,
    requests: BTreeMap<std::net::IpAddr, (Instant, u16)>,
}
#[cfg_attr(not(test), allow(dead_code))]
impl Directory {
    pub fn bind(addr: SocketAddr) -> io::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;
        let tcp = tcp::Relay::bind(socket.local_addr()?)?;
        Ok(Self {
            tcp,
            socket,
            rooms: BTreeMap::new(),
            requests: BTreeMap::new(),
        })
    }
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }
    pub fn poll(&mut self) -> io::Result<()> {
        self.tcp.poll()?;
        let now = Instant::now();
        self.rooms
            .retain(|_, r| now.duration_since(r.seen) < Duration::from_secs(20));
        self.requests
            .retain(|_, (t, _)| now.duration_since(*t) < Duration::from_secs(2));
        let mut buf = [0u8; MTU];
        for _ in 0..256 {
            let (n, from) = match self.socket.recv_from(&mut buf) {
                Ok(v) => v,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            };
            if n < 4 || &buf[..4] != b"VPH3" {
                continue;
            }
            if !self.requests.contains_key(&from.ip()) && self.requests.len() >= 512 {
                continue;
            }
            let (start, count) = self.requests.entry(from.ip()).or_insert((now, 0));
            if now.duration_since(*start) > Duration::from_secs(1) {
                *start = now;
                *count = 0;
            }
            *count = count.saturating_add(1);
            if *count > 8192 {
                continue;
            }
            let Ok(msg) = postcard::from_bytes::<Hub>(&buf[4..n]) else {
                continue;
            };
            match msg {
                Hub::Register {
                    room,
                    lease,
                    public,
                } => {
                    if public.as_ref().is_some_and(|a| {
                        !valid_name(&a.title)
                            || a.capacity > 8
                            || a.players > 8
                            || a.key[..8] != room
                    }) {
                        continue;
                    }
                    if let Some(r) = self.rooms.get_mut(&room) {
                        if r.lease == lease && r.endpoint == from {
                            r.seen = now;
                            r.public = public;
                        }
                    } else if self.rooms.len() < 128 {
                        self.rooms.insert(
                            room,
                            DirectoryRoom {
                                endpoint: from,
                                lease,
                                seen: now,
                                public,
                                peers: BTreeMap::new(),
                            },
                        );
                    }
                }
                Hub::Unregister { room, lease } => {
                    if self
                        .rooms
                        .get(&room)
                        .is_some_and(|r| r.lease == lease && r.endpoint == from)
                    {
                        self.rooms.remove(&room);
                    }
                }
                Hub::Browse {
                    nonce,
                    mode,
                    padding,
                } => {
                    if padding.len() != 512 {
                        continue;
                    }
                    // Four listings keep replies inside one datagram and bound amplification.
                    let rooms = self
                        .rooms
                        .values()
                        .filter_map(|r| {
                            r.public
                                .as_ref()
                                .filter(|a| {
                                    a.players < a.capacity && mode.is_none_or(|m| a.mode == m)
                                })
                                .map(|a| {
                                    let mut a = a.clone();
                                    a.endpoint = r.endpoint;
                                    a
                                })
                        })
                        .take(4)
                        .collect();
                    self.send(from, &Hub::Listings { nonce, rooms })?;
                }
                Hub::Relay {
                    room,
                    peer,
                    target,
                    bytes,
                } => {
                    if bytes.len() > HEADER + CHUNK + 16
                        || bytes.len() < HEADER + 16
                        || &bytes[..4] != MAGIC
                    {
                        continue;
                    }
                    if bytes[4..12] != room
                        || u64::from_le_bytes(bytes[12..20].try_into().unwrap()) != peer
                    {
                        continue;
                    }
                    let Some(r) = self.rooms.get_mut(&room) else {
                        continue;
                    };
                    r.peers.retain(|_, (_, t)| now.duration_since(*t) < TIMEOUT);
                    let destination = if peer == 0 {
                        if from != r.endpoint {
                            continue;
                        }
                        r.peers.get(&target).map(|p| p.0)
                    } else {
                        if target != 0 {
                            continue;
                        }
                        if !r.peers.contains_key(&peer) && r.peers.len() >= MAX_PEERS - 1 {
                            continue;
                        }
                        if r.peers.get(&peer).is_some_and(|(p, _)| *p != from) {
                            continue;
                        }
                        r.peers.insert(peer, (from, now));
                        Some(r.endpoint)
                    };
                    if let Some(to) = destination {
                        self.send(
                            to,
                            &Hub::Relay {
                                room,
                                peer,
                                target,
                                bytes,
                            },
                        )?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn send(&self, to: SocketAddr, msg: &Hub) -> io::Result<()> {
        send_hub(&self.socket, to, msg)
    }
}
fn send_hub(socket: &UdpSocket, to: SocketAddr, msg: &Hub) -> io::Result<()> {
    let mut bytes = b"VPH3".to_vec();
    bytes.extend(postcard::to_stdvec(msg).map_err(|_| err("hub encoding failed"))?);
    if bytes.len() > MTU {
        return Err(err("directory packet too large"));
    }
    socket.send_to(&bytes, to)?;
    Ok(())
}
fn connect_relay(server: SocketAddr) -> std::sync::mpsc::Receiver<io::Result<tcp::Connection>> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let _ = tx.send(tcp::Connection::connect(server));
    });
    rx
}
pub struct Browser {
    pub mode: Option<GameMode>,
    connection: Option<tcp::Connection>,
    connecting: Option<std::sync::mpsc::Receiver<io::Result<tcp::Connection>>>,
    pub server: SocketAddr,
    nonce: u64,
    sent: Instant,
    received: Option<Instant>,
    requested: Instant,
    pub rooms: Vec<Advert>,
    pub page: u8,
    pub pages: u8,
    pub latency_ms: u128,
}
impl Browser {
    pub fn new(server: SocketAddr) -> io::Result<Self> {
        Ok(Self {
            mode: None,
            connection: None,
            connecting: Some(connect_relay(server)),
            server,
            nonce: u64::from_le_bytes(random()?),
            sent: Instant::now() - Duration::from_secs(4),
            received: None,
            requested: Instant::now(),
            rooms: Vec::new(),
            page: 0,
            pages: 1,
            latency_ms: 0,
        })
    }
    pub fn refresh(&mut self) {
        self.rooms.clear();
        self.received = None;
        self.requested = Instant::now();
        self.nonce = self.nonce.wrapping_add(1);
        self.sent = Instant::now() - Duration::from_secs(4);
    }
    pub fn turn_page(&mut self, next: bool) {
        self.page = if next {
            (self.page + 1).min(self.pages - 1)
        } else {
            self.page.saturating_sub(1)
        };
        self.refresh();
    }
    pub fn status(&self) -> String {
        if self.connecting.is_some() {
            "CONNECTING TO SERVER...".into()
        } else if self.received.is_none() {
            "REQUESTING SERVER LIST...".into()
        } else if self.rooms.is_empty() {
            "SERVER ONLINE. NO ROOMS FOUND. CREATE A ROOM.".into()
        } else {
            format!(
                "SERVER ONLINE, {} MS. SELECT AN OPEN ROOM TO JOIN.",
                self.latency_ms
            )
        }
    }
    pub fn poll(&mut self) -> io::Result<()> {
        if let Some(rx) = &self.connecting {
            match rx.try_recv() {
                Ok(result) => {
                    self.connection = Some(result?);
                    self.connecting = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return Ok(()),
                Err(_) => return Err(err("server connection failed")),
            }
        }
        let Some(connection) = &mut self.connection else {
            return Ok(());
        };
        if self.sent.elapsed() > Duration::from_secs(3) {
            self.nonce = self.nonce.wrapping_add(1);
            connection.send(&Hub::BrowsePage {
                nonce: self.nonce,
                mode: self.mode,
                page: self.page,
            })?;
            self.sent = Instant::now();
        }
        for message in connection.poll()? {
            if let Hub::Page {
                nonce,
                page,
                pages,
                rooms,
            } = message
            {
                if nonce == self.nonce
                    && rooms.len() <= BROWSER_PAGE
                    && (1..=22).contains(&pages)
                    && page < pages
                {
                    self.rooms = rooms.into_iter().filter(Advert::valid).collect();
                    self.page = page;
                    self.pages = pages;
                    self.latency_ms = self.sent.elapsed().as_millis();
                    self.received = Some(Instant::now());
                }
            }
        }
        if self.received.unwrap_or(self.requested).elapsed() > Duration::from_secs(8) {
            return Err(err("server not responding; check address and TCP port"));
        }
        Ok(())
    }
}
#[derive(Default)]
struct Replay {
    high: u64,
    bits: u64,
    salt: Option<[u8; 16]>,
}
impl Replay {
    fn accept(&mut self, salt: [u8; 16], seq: u64) -> bool {
        if seq == 0 {
            return false;
        }
        if let Some(s) = self.salt {
            if s != salt {
                return false;
            }
        } else {
            self.salt = Some(salt);
        }
        if seq > self.high {
            let delta = seq - self.high;
            self.bits = if delta >= 64 {
                1
            } else {
                (self.bits << delta) | 1
            };
            self.high = seq;
            true
        } else {
            let d = self.high - seq;
            if d >= 64 || self.bits & (1 << d) != 0 {
                false
            } else {
                self.bits |= 1 << d;
                true
            }
        }
    }
}
struct Peer {
    endpoint: SocketAddr,
    direct: bool,
    seen: Instant,
    replay: Replay,
}
struct Assembly {
    at: Instant,
    count: u8,
    parts: BTreeMap<u8, Vec<u8>>,
    size: usize,
}
pub enum Event {
    Joined(u64, String, u8),
    Data(u64, Message),
    Disconnected(u64),
}
pub struct Session {
    socket: Option<UdpSocket>,
    tcp: Option<tcp::Connection>,
    connecting: Option<std::sync::mpsc::Receiver<io::Result<tcp::Connection>>>,
    pub host: bool,
    pub id: u64,
    key: [u8; 32],
    room: [u8; 8],
    salt: [u8; 16],
    seq: u64,
    message: u32,
    invite: Invite,
    hub: Option<SocketAddr>,
    lease: [u8; 16],
    public: bool,
    title: String,
    peers: BTreeMap<u64, Peer>,
    assemblies: BTreeMap<(u64, u32), Assembly>,
    pub members: Vec<Member>,
    pub rules: Rules,
    pub status: String,
    last_hello: Instant,
    last_lobby: Instant,
    last_ping: Instant,
    started: Instant,
    hello: Message,
    pending: VecDeque<Event>,
}
impl Session {
    pub fn host(
        bind: SocketAddr,
        hub: Option<SocketAddr>,
        public: bool,
        name: &str,
        jet: u8,
        rules: Rules,
    ) -> io::Result<Self> {
        if !valid_name(name) || jet >= 4 || !rules.valid() {
            return Err(err("invalid room setup"));
        }
        let socket = if hub.is_none() {
            let socket = UdpSocket::bind(bind)?;
            socket.set_nonblocking(true)?;
            Some(socket)
        } else {
            None
        };
        let mut endpoint = match &socket {
            Some(s) => s.local_addr()?,
            None => hub.unwrap(),
        };
        if endpoint.ip().is_unspecified() {
            let probe = UdpSocket::bind(if endpoint.is_ipv6() {
                "[::]:0"
            } else {
                "0.0.0.0:0"
            })?;
            if probe
                .connect(if endpoint.is_ipv6() {
                    "[2001:db8::1]:9"
                } else {
                    "192.0.2.1:9"
                })
                .is_ok()
            {
                if let Ok(addr) = probe.local_addr() {
                    endpoint.set_ip(addr.ip());
                }
            }
        }
        let invite = Invite {
            endpoint: hub.unwrap_or(endpoint),
            key: random()?,
            relay: hub.is_some(),
            tcp: hub.is_some(),
        };
        let mut s = Self::new(socket, true, 0, invite, hub, name, jet)?;
        s.public = public;
        s.rules = rules;
        s.members = vec![Member {
            id: 0,
            name: name.into(),
            jet,
            ready: true,
            team: 0,
        }];
        Ok(s)
    }
    pub fn join(invite: Invite, name: &str, jet: u8) -> io::Result<Self> {
        if !valid_name(name) || jet >= 4 {
            return Err(err("invalid callsign or aircraft"));
        }
        let socket = if invite.tcp {
            None
        } else {
            let socket = UdpSocket::bind(if invite.endpoint.is_ipv6() {
                "[::]:0"
            } else {
                "0.0.0.0:0"
            })?;
            socket.set_nonblocking(true)?;
            Some(socket)
        };
        let hub = invite.relay.then_some(invite.endpoint);
        let id = u64::from_le_bytes(random()?).max(1);
        Self::new(socket, false, id, invite, hub, name, jet)
    }
    fn new(
        socket: Option<UdpSocket>,
        host: bool,
        id: u64,
        invite: Invite,
        hub: Option<SocketAddr>,
        name: &str,
        jet: u8,
    ) -> io::Result<Self> {
        let now = Instant::now();
        let mut peers = BTreeMap::new();
        if !host {
            peers.insert(
                0,
                Peer {
                    endpoint: invite.endpoint,
                    direct: !invite.relay,
                    seen: now,
                    replay: Replay::default(),
                },
            );
        }
        Ok(Self {
            socket,
            tcp: None,
            connecting: invite.tcp.then(|| connect_relay(invite.endpoint)),
            host,
            id,
            key: invite.key,
            room: invite.room(),
            salt: random()?,
            seq: 0,
            message: 0,
            invite,
            hub,
            lease: random()?,
            public: false,
            title: name.into(),
            peers,
            assemblies: BTreeMap::new(),
            members: Vec::new(),
            rules: Rules::default(),
            status: if hub.is_some() {
                "CONNECTING TO RELAY"
            } else if host {
                "LAN ROOM OPEN"
            } else {
                "CONNECTING"
            }
            .into(),
            last_hello: now - Duration::from_secs(2),
            last_lobby: now - Duration::from_secs(2),
            last_ping: now,
            started: now,
            hello: Message::Hello {
                name: name.into(),
                jet,
            },
            pending: VecDeque::new(),
        })
    }
    pub fn invitation(&self) -> String {
        self.invite.text()
    }
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        if let Some(tcp) = &self.tcp {
            tcp.local_addr()
        } else if let Some(socket) = &self.socket {
            socket.local_addr()
        } else {
            Ok(self.invite.endpoint)
        }
    }
    fn send_directory(&mut self, message: &Hub) -> io::Result<()> {
        if let Some(tcp) = &mut self.tcp {
            tcp.send(message)
        } else if let (Some(socket), Some(hub)) = (&self.socket, self.hub) {
            send_hub(socket, hub, message)
        } else {
            Ok(())
        }
    }
    pub fn send(&mut self, to: u64, msg: &Message) -> io::Result<()> {
        if !msg.valid() {
            return Err(err("invalid message"));
        }
        let bytes = postcard::to_stdvec(msg).map_err(|_| err("message encoding failed"))?;
        if bytes.len() > MAX_MESSAGE {
            return Err(err("message too large"));
        }
        let Some(peer) = self.peers.get(&to) else {
            return Ok(());
        };
        let (endpoint, direct) = (peer.endpoint, peer.direct);
        self.message = self.message.wrapping_add(1);
        let count = bytes.len().div_ceil(CHUNK) as u8;
        for (part, data) in bytes.chunks(CHUNK).enumerate() {
            self.seq = self
                .seq
                .checked_add(1)
                .ok_or_else(|| err("session sequence exhausted"))?;
            let mut header = Vec::with_capacity(HEADER);
            header.extend(MAGIC);
            header.extend(self.room);
            header.extend(self.id.to_le_bytes());
            header.extend(self.salt);
            header.extend(self.seq.to_le_bytes());
            header.extend(self.message.to_le_bytes());
            header.push(part as u8);
            header.push(count);
            let mut nonce = [0; 24];
            nonce[..16].copy_from_slice(&self.salt);
            nonce[16..].copy_from_slice(&self.seq.to_le_bytes());
            let cipher = XChaCha20Poly1305::new_from_slice(&self.key).unwrap();
            let encrypted = cipher
                .encrypt(
                    XNonce::from_slice(&nonce),
                    Payload {
                        msg: data,
                        aad: &header,
                    },
                )
                .map_err(|_| err("packet encryption failed"))?;
            header.extend(encrypted);
            if direct {
                if let Some(socket) = &self.socket {
                    socket.send_to(&header, endpoint)?;
                }
            } else if self.hub.is_some() {
                self.send_directory(&Hub::Relay {
                    room: self.room,
                    peer: self.id,
                    target: to,
                    bytes: header,
                })?;
            }
        }
        Ok(())
    }
    pub fn broadcast(&mut self, msg: &Message) -> io::Result<()> {
        for id in self.peers.keys().copied().collect::<Vec<_>>() {
            self.send(id, msg)?;
        }
        Ok(())
    }
    pub fn set_ready(&mut self, ready: bool, jet: u8) -> io::Result<()> {
        if self.host {
            if let Some(m) = self.members.first_mut() {
                m.jet = jet;
                m.ready = ready;
            }
        } else {
            self.send(0, &Message::Ready { ready, jet })?;
        }
        Ok(())
    }
    pub fn launch(&mut self) -> io::Result<()> {
        if !self.host {
            return Err(err("only the host can launch"));
        }
        if self.members.len() < 2 || self.members.iter().any(|m| !m.ready) {
            return Err(err("at least two ready pilots are required"));
        }
        self.rules.started = true;
        self.last_lobby = Instant::now() - Duration::from_secs(2);
        Ok(())
    }
    pub fn poll(&mut self) -> io::Result<Vec<Event>> {
        let now = Instant::now();
        if let Some(rx) = &self.connecting {
            match rx.try_recv() {
                Ok(result) => {
                    self.tcp = Some(result?);
                    self.connecting = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return Ok(Vec::new()),
                Err(_) => return Err(err("relay connection failed")),
            }
        }
        if let Some(tcp) = &mut self.tcp {
            let messages = tcp.poll()?;
            for message in messages {
                match message {
                    Hub::Relay {
                        room,
                        target,
                        bytes,
                        ..
                    } if room == self.room && target == self.id => {
                        self.receive(self.invite.endpoint, &bytes, false, now);
                    }
                    Hub::Registered if self.host => self.status = "ROOM ONLINE, TCP RELAY".into(),
                    Hub::RoomMissing
                        if !self.host
                            && self.members.is_empty()
                            && self.started.elapsed() < TIMEOUT =>
                    {
                        self.last_hello = now - Duration::from_millis(450);
                        self.status = "WAITING FOR HOST TO REGISTER".into();
                    }
                    Hub::RoomMissing => return Err(err("room closed; refresh server browser")),
                    Hub::Failure(message) => return Err(err(&message)),
                    _ => {}
                }
            }
        }
        if self.invite.tcp
            && self.host
            && self.status == "CONNECTING TO RELAY"
            && self.started.elapsed() > Duration::from_secs(8)
        {
            return Err(err("relay did not confirm the room; update viper-server"));
        }
        let mut buf = [0; MTU];
        self.assemblies
            .retain(|_, a| now.duration_since(a.at) < Duration::from_secs(1));
        for _ in 0..192 {
            let Some(socket) = &self.socket else {
                break;
            };
            let (n, from) = match socket.recv_from(&mut buf) {
                Ok(v) => v,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            };
            if n >= 4 && &buf[..4] == b"VPH3" {
                if Some(from) != self.hub {
                    continue;
                }
                if let Ok(Hub::Relay {
                    room,
                    peer: _,
                    target,
                    bytes,
                }) = postcard::from_bytes::<Hub>(&buf[4..n])
                {
                    if room == self.room && target == self.id {
                        self.receive(from, &bytes, false, now);
                    }
                }
            } else {
                self.receive(from, &buf[..n], true, now);
            }
        }
        let lost: Vec<_> = self
            .peers
            .iter()
            .filter(|(_, p)| now.duration_since(p.seen) > TIMEOUT)
            .map(|(&id, _)| id)
            .collect();
        for id in lost {
            self.peers.remove(&id);
            self.members.retain(|m| m.id != id);
            self.pending.push_back(Event::Disconnected(id));
            if !self.host {
                self.status = "HOST DISCONNECTED".into();
            }
        }
        if !self.host
            && self.members.is_empty()
            && self.last_hello.elapsed() > Duration::from_millis(500)
        {
            self.send(0, &self.hello.clone())?;
            self.last_hello = now;
            if self.started.elapsed() > TIMEOUT {
                return Err(err(
                    "connection timed out; check invitation, host and firewall",
                ));
            }
        }
        if self.host && self.last_lobby.elapsed() > Duration::from_millis(500) {
            self.broadcast(&Message::Lobby {
                rules: self.rules.clone(),
                members: self.members.clone(),
            })?;
            if self.hub.is_some() {
                let public = (self.public && !self.rules.started).then(|| Advert {
                    title: self.title.clone(),
                    endpoint: self.local_addr().unwrap(),
                    key: self.key,
                    mode: self.rules.mode,
                    players: self.members.len() as u8,
                    capacity: self.rules.capacity,
                });
                self.send_directory(&Hub::Register {
                    room: self.room,
                    lease: self.lease,
                    public,
                })?;
            }
            self.last_lobby = now;
        }
        if self.last_ping.elapsed() > Duration::from_secs(1) {
            self.broadcast(&Message::Ping(self.seq))?;
            self.last_ping = now;
        }
        Ok(self.pending.drain(..).collect())
    }
    fn receive(&mut self, from: SocketAddr, bytes: &[u8], direct: bool, now: Instant) {
        if bytes.len() < HEADER + 16
            || bytes.len() > HEADER + CHUNK + 16
            || &bytes[..4] != MAGIC
            || bytes[4..12] != self.room
        {
            return;
        }
        let peer = u64::from_le_bytes(bytes[12..20].try_into().unwrap());
        if peer == self.id || (!self.host && peer != 0) {
            return;
        }
        if let Some(p) = self.peers.get(&peer) {
            if direct && p.direct && p.endpoint != from {
                return;
            }
        } else if !self.host
            || self.peers.len() >= self.rules.capacity as usize - 1
            || self.rules.started
        {
            return;
        }
        let salt: [u8; 16] = bytes[20..36].try_into().unwrap();
        let seq = u64::from_le_bytes(bytes[36..44].try_into().unwrap());
        let id = u32::from_le_bytes(bytes[44..48].try_into().unwrap());
        let (part, count) = (bytes[48], bytes[49]);
        if count == 0 || count > 60 || part >= count {
            return;
        }
        let mut nonce = [0; 24];
        nonce[..16].copy_from_slice(&salt);
        nonce[16..].copy_from_slice(&seq.to_le_bytes());
        let cipher = XChaCha20Poly1305::new_from_slice(&self.key).unwrap();
        let Ok(plain) = cipher.decrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &bytes[HEADER..],
                aad: &bytes[..HEADER],
            },
        ) else {
            return;
        };
        let p = self.peers.entry(peer).or_insert_with(|| Peer {
            endpoint: from,
            direct,
            seen: now,
            replay: Replay::default(),
        });
        if !p.replay.accept(salt, seq) {
            return;
        }
        p.seen = now;
        if direct {
            p.endpoint = from;
            p.direct = true;
        }
        if !self.assemblies.contains_key(&(peer, id)) && self.assemblies.len() >= 32 {
            return;
        }
        let a = self
            .assemblies
            .entry((peer, id))
            .or_insert_with(|| Assembly {
                at: now,
                count,
                parts: BTreeMap::new(),
                size: 0,
            });
        if a.count != count || a.parts.contains_key(&part) {
            return;
        }
        a.size += plain.len();
        if a.size > MAX_MESSAGE {
            self.assemblies.remove(&(peer, id));
            return;
        }
        a.parts.insert(part, plain);
        if a.parts.len() != count as usize {
            return;
        }
        let a = self.assemblies.remove(&(peer, id)).unwrap();
        let mut data = Vec::with_capacity(a.size);
        for (_, p) in a.parts {
            data.extend(p);
        }
        let Ok(msg) = postcard::from_bytes::<Message>(&data) else {
            return;
        };
        if !msg.valid() {
            return;
        }
        match msg {
            Message::Hello { name, jet } if self.host => {
                if !self.members.iter().any(|m| m.id == peer) {
                    let team = if self.rules.mode == GameMode::Teams {
                        let a = self.members.iter().filter(|m| m.team == 0).count();
                        let b = self.members.len() - a;
                        if a <= b {
                            0
                        } else {
                            1
                        }
                    } else if self.rules.mode == GameMode::Dogfight {
                        (1..8)
                            .find(|team| self.members.iter().all(|m| m.team != *team))
                            .unwrap_or(7)
                    } else {
                        0
                    };
                    self.members.push(Member {
                        id: peer,
                        name: name.clone(),
                        jet,
                        ready: false,
                        team,
                    });
                    self.pending.push_back(Event::Joined(peer, name, jet));
                }
                self.last_lobby = now - Duration::from_secs(1);
            }
            Message::Lobby { rules, members } if !self.host => {
                if members.iter().any(|m| m.id == self.id)
                    && members.first().is_some_and(|m| m.id == 0)
                {
                    self.rules = rules;
                    self.members = members;
                    self.status = "CONNECTED".into();
                }
            }
            Message::Ready { ready, jet } if self.host => {
                if !self.rules.started {
                    if let Some(m) = self.members.iter_mut().find(|m| m.id == peer) {
                        m.ready = ready;
                        m.jet = jet;
                    }
                }
            }
            Message::Ping(n) => {
                let _ = self.send(peer, &Message::Pong(n));
            }
            Message::Pong(_) => {}
            Message::Leave => {
                self.peers.remove(&peer);
                self.members.retain(|m| m.id != peer);
                self.pending.push_back(Event::Disconnected(peer));
            }
            m => {
                if self.members.iter().any(|m| m.id == peer) {
                    self.pending.push_back(Event::Data(peer, m));
                }
            }
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.broadcast(&Message::Leave);
        if self.host {
            if self.hub.is_some() {
                let _ = self.send_directory(&Hub::Unregister {
                    room: self.room,
                    lease: self.lease,
                });
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn pump(host: &mut Session, client: &mut Session) {
        for _ in 0..10 {
            client.poll().unwrap();
            host.poll().unwrap();
            client.poll().unwrap();
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    #[test]
    fn encrypted_direct_peers_join_ready_launch_and_exchange_fragmented_frames() {
        let mut host = Session::host(
            "127.0.0.1:0".parse().unwrap(),
            None,
            false,
            "Host",
            0,
            Rules::default(),
        )
        .unwrap();
        let mut client =
            Session::join(Invite::parse(&host.invitation()).unwrap(), "Wing", 1).unwrap();
        pump(&mut host, &mut client);
        assert_eq!(host.members.len(), 2);
        assert_eq!(client.members.len(), 2);
        assert!(host.launch().is_err());
        client.set_ready(true, 1).unwrap();
        pump(&mut host, &mut client);
        host.launch().unwrap();
        let payload = vec![0xA5; 12000];
        host.send(
            client.id,
            &Message::Frame {
                tick: 12,
                bytes: payload.clone(),
            },
        )
        .unwrap();
        let events = client.poll().unwrap();
        assert!(events
            .iter()
            .any(|e| matches!(e,Event::Data(0,Message::Frame{tick:12,bytes}) if *bytes==payload)));
    }
    #[test]
    fn replay_window_accepts_reordering_and_rejects_replay() {
        let mut r = Replay::default();
        let salt = [4; 16];
        assert!(r.accept(salt, 100));
        assert!(r.accept(salt, 98));
        assert!(!r.accept(salt, 98));
        assert!(!r.accept(salt, 1));
        assert!(!r.accept([5; 16], 101));
        assert!(r.accept(salt, 165));
        assert!(!r.accept(salt, 100));
    }
    #[test]
    fn wrong_key_does_not_join_and_malformed_invites_fail() {
        let mut host = Session::host(
            "127.0.0.1:0".parse().unwrap(),
            None,
            false,
            "Host",
            0,
            Rules::default(),
        )
        .unwrap();
        let mut inv = Invite::parse(&host.invitation()).unwrap();
        inv.key[31] ^= 1;
        let mut client = Session::join(inv, "Wing", 1).unwrap();
        pump(&mut host, &mut client);
        assert_eq!(host.members.len(), 1);
        for text in ["", "localhost#abcd", "localhost:abc#abcd", "###"] {
            assert!(Invite::parse(text).is_err());
        }
    }
    #[test]
    fn relay_directory_connects_peers_without_direct_host_port() {
        let mut hub = Directory::bind("127.0.0.1:0".parse().unwrap()).unwrap();
        let mut host = Session::host(
            "127.0.0.1:0".parse().unwrap(),
            Some(hub.local_addr().unwrap()),
            true,
            "Host",
            0,
            Rules::default(),
        )
        .unwrap();
        host.poll().unwrap();
        hub.poll().unwrap();
        let mut client =
            Session::join(Invite::parse(&host.invitation()).unwrap(), "Wing", 1).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while client.members.len() < 2 && Instant::now() < deadline {
            client.poll().unwrap();
            hub.poll().unwrap();
            host.poll().unwrap();
            hub.poll().unwrap();
            client.poll().unwrap();
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(host.members.len(), 2);
        assert_eq!(client.members.len(), 2);
    }
}

#[cfg(test)]
mod discovery_tests {
    use super::*;
    #[test]
    fn browser_finds_only_open_rooms_matching_the_requested_mode() {
        let mut directory = Directory::bind("127.0.0.1:0".parse().unwrap()).unwrap();
        let server = directory.local_addr().unwrap();
        let mut host = Session::host(
            "127.0.0.1:0".parse().unwrap(),
            Some(server),
            true,
            "Strike",
            0,
            Rules {
                mode: GameMode::Strike,
                ..Default::default()
            },
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while host.status != "ROOM ONLINE, TCP RELAY" && Instant::now() < deadline {
            host.poll().unwrap();
            directory.poll().unwrap();
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(host.status, "ROOM ONLINE, TCP RELAY");
        let mut browser = Browser::new(server).unwrap();
        browser.mode = Some(GameMode::Strike);
        for _ in 0..30 {
            browser.poll().unwrap();
            directory.poll().unwrap();
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(browser.rooms.len(), 1);
        assert_eq!(browser.rooms[0].mode, GameMode::Strike);
        assert_eq!(browser.rooms[0].key, host.key);
        let mut other = Browser::new(server).unwrap();
        other.mode = Some(GameMode::Teams);
        for _ in 0..30 {
            other.poll().unwrap();
            directory.poll().unwrap();
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(other.rooms.is_empty());
    }
}

#[cfg(test)]
mod full_room_tests {
    use super::*;
    #[test]
    fn eight_pilots_receive_fragmented_authoritative_state() {
        let mut host = Session::host(
            "127.0.0.1:0".parse().unwrap(),
            None,
            false,
            "Lead",
            0,
            Rules {
                capacity: 8,
                ..Default::default()
            },
        )
        .unwrap();
        let mut clients = (0..7)
            .map(|i| {
                Session::join(
                    Invite::parse(&host.invitation()).unwrap(),
                    &format!("Wing{i}"),
                    (i % 4) as u8,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        for _ in 0..12 {
            for c in &mut clients {
                c.poll().unwrap();
            }
            host.poll().unwrap();
            for c in &mut clients {
                c.poll().unwrap();
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(host.members.len(), 8);
        assert!(clients.iter().all(|c| c.members.len() == 8));
        let bytes = vec![73; 16000];
        host.broadcast(&Message::Frame {
            tick: 30,
            bytes: bytes.clone(),
        })
        .unwrap();
        for c in &mut clients {
            let events = c.poll().unwrap();
            assert!(events
                .iter()
                .any(|e| matches!(e,Event::Data(0,Message::Frame{tick:30,bytes:b})if *b==bytes)));
        }
    }
}

#[cfg(test)]
mod tcp_browser_tests {
    use super::*;
    fn pump(hub: &mut Directory, hosts: &mut [Session], browser: &mut Browser) {
        let until = Instant::now() + Duration::from_millis(120);
        while Instant::now() < until {
            for host in hosts.iter_mut() {
                host.poll().unwrap();
            }
            hub.poll().unwrap();
            browser.poll().unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    #[test]
    fn tcp_hosts_use_no_udp_listener_and_browser_pages_filter_and_hide_private_rooms() {
        let mut hub = Directory::bind("127.0.0.1:0".parse().unwrap()).unwrap();
        let server = hub.local_addr().unwrap();
        let occupied = UdpSocket::bind("127.0.0.1:0").unwrap();
        let mut hosts = (0..9)
            .map(|i| {
                Session::host(
                    occupied.local_addr().unwrap(),
                    Some(server),
                    i != 8,
                    &format!("Room{i}"),
                    0,
                    Rules {
                        mode: if i == 7 {
                            GameMode::Strike
                        } else {
                            GameMode::Combat
                        },
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        assert!(hosts.iter().all(|h| h.socket.is_none()));
        assert!(hosts
            .iter()
            .all(|h| h.invitation().starts_with("relay+tcp:")));
        let mut browser = Browser::new(server).unwrap();
        pump(&mut hub, &mut hosts, &mut browser);
        browser.refresh();
        pump(&mut hub, &mut hosts, &mut browser);
        assert_eq!(browser.pages, 2);
        assert_eq!(browser.rooms.len(), BROWSER_PAGE);
        let mut names = browser
            .rooms
            .iter()
            .map(|a| a.title.clone())
            .collect::<Vec<_>>();
        browser.turn_page(true);
        pump(&mut hub, &mut hosts, &mut browser);
        assert_eq!(browser.page, 1);
        assert_eq!(browser.rooms.len(), 2);
        names.extend(browser.rooms.iter().map(|a| a.title.clone()));
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 8);
        assert!(!names.iter().any(|n| n == "Room8"));
        browser.mode = Some(GameMode::Strike);
        browser.refresh();
        pump(&mut hub, &mut hosts, &mut browser);
        assert_eq!(browser.pages, 1);
        assert_eq!(browser.page, 0);
        assert_eq!(browser.rooms.len(), 1);
        assert_eq!(browser.rooms[0].title, "Room7");
        // Starting a sortie removes it from subsequent listings.
        hosts[7].rules.started = true;
        hosts[7].last_lobby = Instant::now() - Duration::from_secs(1);
        pump(&mut hub, &mut hosts, &mut browser);
        browser.refresh();
        pump(&mut hub, &mut hosts, &mut browser);
        assert!(browser.rooms.is_empty());
    }
    #[test]
    fn tcp_relay_delivers_large_frames_voice_and_disconnects() {
        let mut hub = Directory::bind("127.0.0.1:0".parse().unwrap()).unwrap();
        let mut host = Session::host(
            "127.0.0.1:0".parse().unwrap(),
            Some(hub.local_addr().unwrap()),
            true,
            "Lead",
            0,
            Rules::default(),
        )
        .unwrap();
        let mut wing =
            Session::join(Invite::parse(&host.invitation()).unwrap(), "Wing", 3).unwrap();
        let end = Instant::now() + Duration::from_secs(2);
        while wing.members.len() < 2 && Instant::now() < end {
            host.poll().unwrap();
            hub.poll().unwrap();
            wing.poll().unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(wing.members.len(), 2);
        assert!(host.socket.is_none() && wing.socket.is_none());
        wing.set_ready(true, 3).unwrap();
        let payload = vec![0x5a; 16000];
        host.send(
            wing.id,
            &Message::Frame {
                tick: 42,
                bytes: payload.clone(),
            },
        )
        .unwrap();
        wing.send(
            0,
            &Message::Voice {
                speaker: wing.id,
                sequence: 1,
                data: vec![12; 320],
            },
        )
        .unwrap();
        let (mut frame, mut voice) = (false, false);
        let end = Instant::now() + Duration::from_secs(2);
        while (!frame || !voice) && Instant::now() < end {
            hub.poll().unwrap();
            for e in host.poll().unwrap() {
                if matches!(e, Event::Data(_, Message::Voice { sequence: 1, .. })) {
                    voice = true;
                }
            }
            for e in wing.poll().unwrap() {
                if matches!(e,Event::Data(0,Message::Frame{tick:42,bytes})if bytes==payload) {
                    frame = true;
                }
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(frame && voice);
        assert!(host.members.iter().find(|m| m.id == wing.id).unwrap().ready);
        host.launch().unwrap();
        drop(hub);
        let end = Instant::now() + Duration::from_secs(2);
        while host.poll().is_ok() {
            assert!(Instant::now() < end, "host did not detect relay shutdown");
        }
        while wing.poll().is_ok() {
            assert!(Instant::now() < end, "client did not detect relay shutdown");
        }
    }
    #[test]
    fn addresses_and_new_and_legacy_invites_round_trip() {
        assert_eq!(server_address("127.0.0.1").unwrap().port(), HUB_PORT);
        assert_eq!(
            server_address("::1").unwrap().ip(),
            std::net::Ipv6Addr::LOCALHOST
        );
        assert!(server_address("").is_err());
        for prefix in ["", "relay:", "relay+tcp:"] {
            let text = format!("{prefix}127.0.0.1:42861#{}", "ab".repeat(32));
            let invite = Invite::parse(&text).unwrap();
            assert_eq!(invite.text(), text);
            assert_eq!(invite.tcp, prefix == "relay+tcp:");
        }
    }
    #[test]
    fn browser_expires_an_unanswered_list_instead_of_spinning_forever() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let mut browser = Browser::new(listener.local_addr().unwrap()).unwrap();
        let (stream, _) = listener.accept().unwrap();
        let end = Instant::now() + Duration::from_secs(2);
        while browser.connecting.is_some() && Instant::now() < end {
            browser.poll().unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        browser.requested = Instant::now() - Duration::from_secs(9);
        assert!(browser.poll().is_err());
        drop(stream);
    }
}
