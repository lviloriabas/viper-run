//! An owned in-process relay, LAN discovery and temporary router mapping.
//! Dropping the host closes its listeners. Router operations run off the UI thread.
use super::*;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::mpsc::{self, Receiver, Sender};

const HOST_PORTS: std::ops::Range<u16> = HUB_PORT..HUB_PORT + 8;
const LEASE: u32 = 600;

pub(super) fn local_source(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        IpAddr::V6(_) => false,
    }
}
fn public_address(ip: IpAddr) -> bool {
    let IpAddr::V4(ip) = ip else { return false };
    let [a, b, c, _] = ip.octets();
    !local_source(ip.into()) && !ip.is_unspecified() && !ip.is_multicast()
        && !ip.is_documentation() && a != 0 && a < 224
        && !(a == 100 && (64..=127).contains(&b))
        && !(a == 198 && (b == 18 || b == 19))
        && !(a == 192 && b == 0 && c == 0)
}
fn route_address(to: SocketAddr) -> io::Result<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    // UDP connect selects an interface without sending a packet.
    socket.connect(to)?;
    Ok(socket.local_addr()?.ip())
}

#[derive(Clone, Debug)]
enum Internet {
    Checking,
    Mapped(SocketAddr),
    Unavailable(&'static str),
}
enum RouterUpdate {
    State(Internet),
    PublicIp(IpAddr),
    Gateway(SocketAddr),
}
struct Router {
    updates: Receiver<RouterUpdate>,
    stop: Sender<()>,
}
impl Router {
    fn start(port: u16) -> Self {
        let (updates_tx, updates) = mpsc::channel();
        let (stop, stopped) = mpsc::channel();
        std::thread::spawn(move || {
            use igd_next::{GatewayIpVersion, PortMappingProtocol as Protocol, SearchOptions};
            let mut options = SearchOptions::default();
            options.timeout = Some(Duration::from_secs(3));
            options.single_search_timeout = Some(Duration::from_secs(1));
            options.gateway_ip_version = GatewayIpVersion::V4;
            let mapped = (|| {
                let gateway = igd_next::search_gateway(options)
                    .map_err(|_| "ROUTER AUTO SETUP UNAVAILABLE")?;
                let _ = updates_tx.send(RouterUpdate::Gateway(gateway.addr));
                let ip = gateway.get_external_ip().map_err(|_| "ROUTER ADDRESS UNAVAILABLE")?;
                if !public_address(ip) {
                    return Err("ROUTER HAS NO PUBLIC IP (DOUBLE NAT / CGNAT)");
                }
                let _ = updates_tx.send(RouterUpdate::PublicIp(ip));
                let local = SocketAddr::new(route_address(gateway.addr)
                    .map_err(|_| "NO ROUTE TO ROUTER")?, port);
                if stopped.try_recv() != Err(mpsc::TryRecvError::Empty) {
                    return Err("ROOM CLOSED");
                }
                let external = gateway.add_any_port(Protocol::TCP, local, LEASE, "Viper Run room")
                    .map_err(|_| "ROUTER DID NOT ALLOW AN INTERNET CONNECTION")?;
                Ok((gateway, local, SocketAddr::new(ip, external)))
            })();
            let (gateway, local, external) = match mapped {
                Ok(mapping) => mapping,
                Err(reason) => { let _ = updates_tx.send(RouterUpdate::State(Internet::Unavailable(reason))); return; }
            };
            let _ = updates_tx.send(RouterUpdate::State(Internet::Mapped(external)));
            // Finite leases also expire after an unclean shutdown. Never request
            // an infinite mapping, including on routers that only support those.
            while stopped.recv_timeout(Duration::from_secs(180)) == Err(mpsc::RecvTimeoutError::Timeout) {
                if gateway.add_port(Protocol::TCP, external.port(), local, LEASE, "Viper Run room").is_err() {
                    let _ = updates_tx.send(RouterUpdate::State(Internet::Unavailable("INTERNET CONNECTION EXPIRED")));
                    break;
                }
            }
            let _ = gateway.remove_port(Protocol::TCP, external.port());
        });
        Self { updates, stop }
    }
}
impl Drop for Router {
    fn drop(&mut self) { let _ = self.stop.send(()); }
}

pub struct LocalServer {
    directory: Directory,
    pub lan: SocketAddr,
    pub detected_public_ip: Option<IpAddr>,
    pub router_address: Option<SocketAddr>,
    internet: Internet,
    router: Option<Router>,
}
impl LocalServer {
    pub fn start() -> io::Result<Self> {
        for port in HOST_PORTS {
            match Self::bind(SocketAddr::from(([0, 0, 0, 0], port)), true) {
                Ok(server) => return Ok(server),
                Err(e) if e.kind() == io::ErrorKind::AddrInUse => continue,
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(io::ErrorKind::AddrInUse, "ALL ROOM PORTS ARE BUSY. CLOSE ANOTHER HOSTED ROOM."))
    }
    pub fn bind(address: SocketAddr, internet: bool) -> io::Result<Self> {
        let directory = Directory::bind(address)?;
        let address = directory.local_addr()?;
        let ip = if address.ip().is_unspecified() {
            route_address(SocketAddr::from(([192, 0, 2, 1], 9)))
                .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
        } else { address.ip() };
        Ok(Self {
            directory,
            lan: SocketAddr::new(ip, address.port()),
            detected_public_ip: None,
            router_address: None,
            internet: if internet { Internet::Checking } else { Internet::Unavailable("LOCAL TEST ROOM") },
            router: internet.then(|| Router::start(address.port())),
        })
    }
    pub fn connect_address(&self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.lan.port()))
    }
    pub fn poll(&mut self) -> io::Result<()> {
        self.directory.poll()?;
        if let Some(router) = &self.router {
            while let Ok(update) = router.updates.try_recv() {
                match update {
                    RouterUpdate::State(state) => self.internet = state,
                    RouterUpdate::PublicIp(ip) => self.detected_public_ip = Some(ip),
                    RouterUpdate::Gateway(address) => self.router_address = Some(address),
                }
            }
        }
        Ok(())
    }
    pub fn internet_address(&self) -> Option<SocketAddr> {
        match self.internet { Internet::Mapped(address) => Some(address), _ => None }
    }
    pub fn internet_status(&self) -> &str {
        match self.internet {
            Internet::Checking => "INTERNET: SETTING UP YOUR ROUTER...",
            Internet::Mapped(_) => "INTERNET INVITE AVAILABLE. FRIENDS CAN TRY TO CONNECT.",
            Internet::Unavailable(reason) => reason,
        }
    }
    pub fn checking_internet(&self) -> bool { matches!(self.internet, Internet::Checking) }
    pub fn manual_address(&self, text: &str) -> io::Result<SocketAddr> {
        let text = text.trim();
        if text.is_empty() { return Err(err("ENTER THE PUBLIC IPV4 OR DNS SHOWN BY YOUR ROUTER")); }
        let address = match text.parse::<IpAddr>() {
            Ok(ip) => SocketAddr::new(ip, self.lan.port()),
            Err(_) => resolve(text).or_else(|_| resolve(&format!("{text}:{}", self.lan.port())))
                .map_err(|_| err("USE PUBLIC-IP:PORT OR A VALID DNS NAME"))?,
        };
        if address.port() == 0 || !public_address(address.ip()) {
            return Err(err("INTERNET NEEDS A PUBLIC IPV4. FOR A LOCAL IP, USE COPY LAN INVITE."));
        }
        Ok(address)
    }
    pub fn setup_instructions(&self) -> String {
        format!("Viper Run host setup\nHost PC IPv4: {}\nProtocol: TCP\nExternal port: {}\nInternal destination: {}\nIn your router, open Port Forwarding / Virtual Server and forward that TCP port to the host PC. Keep the host PC address reserved in DHCP if possible. Save/apply the rule. Allow Viper Run incoming connections on the Windows private network if asked. No UDP forwarding is needed.\nFind your router's WAN / Internet IPv4 (or use your public DNS name) and enter it under INTERNET HELP, PUBLIC IPV4 / DNS. COPY INTERNET INVITE generates the full address, port and room key for your friend; the friend uses PASTE AND JOIN.\nIf you choose a different external TCP port, include it after the public IP or DNS name.\nA WAN address starting with 10., 192.168., 172.16 through 172.31, or 100.64 through 100.127 is private/carrier NAT. Forwarding this router alone is insufficient. Try the other PC as host, ask the provider for a public IPv4 or use an accessible external relay.\nKeep the game open. The guest does not need to forward ports.\n", self.lan.ip(),self.lan.port(),self.lan)
    }
}

pub struct Nearby {
    socket: UdpSocket,
    nonce: [u8; 8],
    sent: Instant,
    servers: BTreeMap<SocketAddr, (Browser, Instant)>,
    pub rooms: Vec<Advert>,
    pub mode: Option<GameMode>,
    pub page: u8,
    pub pages: u8,
}
impl Nearby {
    pub fn new() -> io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_nonblocking(true)?;
        socket.set_broadcast(true)?;
        Ok(Self { socket, nonce: random()?, sent: Instant::now() - Duration::from_secs(4),
            servers: BTreeMap::new(), rooms: Vec::new(), mode: None, page: 0, pages: 1 })
    }
    pub fn refresh(&mut self) {
        self.sent = Instant::now() - Duration::from_secs(4);
        for (browser, _) in self.servers.values_mut() { browser.refresh(); }
    }
    pub fn turn_page(&mut self, next: bool) {
        self.page = if next { (self.page + 1).min(self.pages - 1) } else { self.page.saturating_sub(1) };
    }
    fn probe(&self, address: SocketAddr) {
        let mut query = b"VPD1".to_vec();
        query.extend(self.nonce);
        // A disabled/disconnected interface must not stop discovery on others.
        let _ = self.socket.send_to(&query, address);
    }
    pub fn poll(&mut self) -> io::Result<()> {
        let now = Instant::now();
        if now.duration_since(self.sent) > Duration::from_secs(3) {
            self.sent = now;
            for port in HOST_PORTS {
                self.probe(SocketAddr::from(([255, 255, 255, 255], port)));
                self.probe(SocketAddr::from(([127, 0, 0, 1], port)));
            }
        }
        let mut bytes = [0u8; 64];
        for _ in 0..64 {
            let (n, from) = match self.socket.recv_from(&mut bytes) {
                Ok(reply) => reply,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            };
            if n != 14 || &bytes[..4] != b"VPA1" || bytes[4..12] != self.nonce || !local_source(from.ip()) { continue; }
            let port = u16::from_le_bytes(bytes[12..14].try_into().unwrap());
            if !HOST_PORTS.contains(&port) || port != from.port() { continue; }
            if let Some((_, seen)) = self.servers.get_mut(&from) { *seen = now; }
            else if self.servers.len() < 32 {
                if let Ok(browser) = Browser::new(from) { self.servers.insert(from, (browser, now)); }
            }
        }
        self.servers.retain(|_, (browser, seen)| now.duration_since(*seen) < Duration::from_secs(10) && browser.poll().is_ok());
        let mut all = Vec::new();
        for (address, (browser, _)) in &self.servers {
            for room in &browser.rooms {
                if self.mode.is_some_and(|mode| mode != room.mode) { continue; }
                let mut room = room.clone();
                room.endpoint = *address;
                // The same host can reply via loopback and its LAN interface.
                if !all.iter().any(|other: &Advert| other.key == room.key) { all.push(room); }
            }
        }
        all.sort_by(|a, b| a.title.cmp(&b.title).then(a.key.cmp(&b.key)));
        self.pages = all.len().div_ceil(BROWSER_PAGE).max(1) as u8;
        self.page = self.page.min(self.pages - 1);
        self.rooms = all.into_iter().skip(self.page as usize * BROWSER_PAGE).take(BROWSER_PAGE).collect();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mapped_invites_never_advertise_private_or_carrier_nat_addresses() {
        for ip in ["0.1.2.3", "10.1.2.3", "127.0.0.1", "100.64.0.1", "100.127.255.254", "172.16.0.1", "192.168.1.1", "169.254.1.2", "192.0.2.1", "198.19.0.1", "224.0.0.1", "255.255.255.255"] {
            assert!(!public_address(ip.parse().unwrap()), "{ip}");
        }
        assert!(public_address("8.8.8.8".parse().unwrap()));
    }
    #[test]
    fn manual_invite_uses_the_actual_host_port_and_rejects_lan_or_carrier_nat() {
        let server=LocalServer::bind("127.0.0.1:0".parse().unwrap(),false).unwrap();
        assert_eq!(server.manual_address("8.8.8.8").unwrap().port(),server.lan.port());
        assert_eq!(server.manual_address("8.8.8.8:45678").unwrap().port(),45678);
        for bad in ["", "127.0.0.1", "192.168.1.50:42861", "100.64.1.2", "8.8.8.8:0", "::1"] {
            assert!(server.manual_address(bad).is_err(),"{bad}");
        }
        assert!(server.setup_instructions().contains(&server.lan.to_string()));
    }
    #[test]
    fn owned_server_connects_two_pilots_without_a_server_process_and_releases_ports() {
        let mut server = LocalServer::bind("127.0.0.1:0".parse().unwrap(), false).unwrap();
        let address = server.connect_address();
        let mut host = Session::host("127.0.0.1:0".parse().unwrap(), Some(address), true, "Lead", 0, Rules::default()).unwrap();
        let invite = Invite::parse(&host.invitation_at(server.lan)).unwrap();
        let mut guest = Session::join(invite, "Wing", 3).unwrap();
        let deadline = Instant::now() + Duration::from_secs(4);
        while host.members.len() != 2 || guest.members.len() != 2 {
            server.poll().unwrap(); host.poll().unwrap(); guest.poll().unwrap();
            assert!(Instant::now() < deadline, "embedded room did not connect");
            std::thread::sleep(Duration::from_millis(2));
        }
        drop(guest); drop(host); drop(server);
        assert!(std::net::TcpListener::bind(address).is_ok());
        assert!(UdpSocket::bind(address).is_ok());
    }
    #[test]
    fn nearby_discovery_finds_public_room_and_hides_private_room() {
        let mut server = HOST_PORTS.filter_map(|port| LocalServer::bind(SocketAddr::from(([127, 0, 0, 1], port)), false).ok()).next().unwrap();
        let mut host = Session::host("127.0.0.1:0".parse().unwrap(), Some(server.connect_address()), true, "NearbyLead", 0, Rules::default()).unwrap();
        let mut private = Session::host("127.0.0.1:0".parse().unwrap(), Some(server.connect_address()), false, "PrivateLead", 0, Rules::default()).unwrap();
        let mut nearby = Nearby::new().unwrap();
        let deadline = Instant::now() + Duration::from_secs(4);
        loop {
            server.poll().unwrap(); host.poll().unwrap(); private.poll().unwrap(); nearby.poll().unwrap();
            if nearby.rooms.iter().any(|room| room.title == "NearbyLead") { break; }
            assert!(Instant::now() < deadline, "nearby room was not found");
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(nearby.rooms.iter().all(|room| room.title != "PrivateLead"));
        let advert = nearby.rooms.iter().find(|room| room.title == "NearbyLead").unwrap();
        let mut guest = Session::join(Invite { endpoint: advert.endpoint, key: advert.key, relay: true, tcp: true }, "Wing", 1).unwrap();
        while host.members.len() < 2 {
            server.poll().unwrap(); host.poll().unwrap(); guest.poll().unwrap();
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}
