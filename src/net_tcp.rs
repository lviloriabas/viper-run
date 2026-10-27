//! Bounded, nonblocking TCP relay. Players only make outbound connections.
use super::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
const QUEUE_LIMIT: usize = 512 * 1024;
pub(super) struct Connection {
    stream: TcpStream,
    input: Vec<u8>,
    output: VecDeque<u8>,
    pub seen: Instant,
}
impl Connection {
    pub fn connect(address: SocketAddr) -> io::Result<Self> {
        Self::new(TcpStream::connect_timeout(
            &address,
            Duration::from_secs(2),
        )?)
    }
    fn new(stream: TcpStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        stream.set_nodelay(true)?;
        Ok(Self {
            stream,
            input: Vec::new(),
            output: VecDeque::new(),
            seen: Instant::now(),
        })
    }
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.stream.local_addr()
    }
    pub fn send(&mut self, message: &Hub) -> io::Result<()> {
        let bytes = postcard::to_stdvec(message).map_err(|_| err("relay encoding failed"))?;
        if bytes.is_empty() || bytes.len() > MTU {
            return Err(err("relay frame too large"));
        }
        if self.output.len() + bytes.len() + 6 > QUEUE_LIMIT {
            return Err(err("relay too slow; reconnect"));
        }
        self.output.extend(b"VPT4");
        self.output.extend((bytes.len() as u16).to_le_bytes());
        self.output.extend(bytes);
        self.flush()
    }
    fn flush(&mut self) -> io::Result<()> {
        for _ in 0..32 {
            if self.output.is_empty() {
                break;
            }
            let (bytes, _) = self.output.as_slices();
            match self.stream.write(bytes) {
                Ok(0) => return Err(err("relay disconnected")),
                Ok(n) => {
                    self.output.drain(..n);
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
    pub fn poll(&mut self) -> io::Result<Vec<Hub>> {
        self.flush()?;
        let mut messages = Vec::new();
        for _ in 0..32 {
            let mut used = 0;
            while self.input.len() - used >= 6 && messages.len() < 192 {
                if &self.input[used..used + 4] != b"VPT4" {
                    return Err(err("incompatible relay; update viper-server"));
                }
                let len =
                    u16::from_le_bytes(self.input[used + 4..used + 6].try_into().unwrap()) as usize;
                if len == 0 || len > MTU {
                    return Err(err("invalid relay frame length"));
                }
                if self.input.len() - used < len + 6 {
                    break;
                }
                messages.push(
                    postcard::from_bytes(&self.input[used + 6..used + 6 + len])
                        .map_err(|_| err("invalid relay frame"))?,
                );
                used += len + 6;
            }
            self.input.drain(..used);
            if messages.len() >= 192 {
                break;
            }
            let mut buffer = [0; 8192];
            match self.stream.read(&mut buffer) {
                Ok(0) if messages.is_empty() => {
                    return Err(err("relay disconnected; check server address"))
                }
                Ok(0) => break,
                Ok(n) => {
                    self.input.extend_from_slice(&buffer[..n]);
                    self.seen = Instant::now();
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(messages)
    }
}
struct Room {
    host: u64,
    lease: [u8; 16],
    seen: Instant,
    public: Option<Advert>,
    peers: BTreeMap<u64, u64>,
}
pub(super) struct Relay {
    listener: TcpListener,
    clients: BTreeMap<u64, Connection>,
    addresses: BTreeMap<u64, SocketAddr>,
    rooms: BTreeMap<[u8; 8], Room>,
    next: u64,
}
impl Relay {
    pub fn bind(address: SocketAddr) -> io::Result<Self> {
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            clients: BTreeMap::new(),
            addresses: BTreeMap::new(),
            rooms: BTreeMap::new(),
            next: 0,
        })
    }
    pub fn poll(&mut self) -> io::Result<()> {
        for _ in 0..16 {
            match self.listener.accept() {
                Ok((stream, address)) => {
                    if self.clients.len() >= 256
                        || self
                            .addresses
                            .values()
                            .filter(|a| a.ip() == address.ip())
                            .count()
                            >= 64
                    {
                        continue;
                    }
                    self.next += 1;
                    if let Ok(connection) = Connection::new(stream) {
                        self.clients.insert(self.next, connection);
                        self.addresses.insert(self.next, address);
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        for id in self.clients.keys().copied().collect::<Vec<_>>() {
            let Some(c) = self.clients.get_mut(&id) else {
                continue;
            };
            match c.poll() {
                Ok(messages) => {
                    for message in messages {
                        self.handle(id, message);
                    }
                }
                Err(_) => {
                    self.clients.remove(&id);
                    self.addresses.remove(&id);
                }
            }
        }
        let now = Instant::now();
        self.clients
            .retain(|_, c| now.duration_since(c.seen) < Duration::from_secs(30));
        self.addresses.retain(|id, _| self.clients.contains_key(id));
        self.rooms.retain(|_, r| {
            self.clients.contains_key(&r.host)
                && now.duration_since(r.seen) < Duration::from_secs(20)
        });
        for room in self.rooms.values_mut() {
            room.peers.retain(|_, id| self.clients.contains_key(id));
        }
        Ok(())
    }
    fn send(&mut self, id: u64, message: Hub) {
        if self
            .clients
            .get_mut(&id)
            .is_some_and(|c| c.send(&message).is_err())
        {
            self.clients.remove(&id);
            self.addresses.remove(&id);
        }
    }
    fn handle(&mut self, id: u64, message: Hub) {
        match message {
            Hub::Register {
                room,
                lease,
                public,
            } => {
                if public
                    .as_ref()
                    .is_some_and(|a| !a.valid() || a.key[..8] != room)
                {
                    return;
                }
                if let Some(r) = self.rooms.get_mut(&room) {
                    if r.host != id || r.lease != lease {
                        return;
                    }
                    r.seen = Instant::now();
                    r.public = public;
                } else if self.rooms.len() < 128 {
                    self.rooms.insert(
                        room,
                        Room {
                            host: id,
                            lease,
                            seen: Instant::now(),
                            public,
                            peers: BTreeMap::new(),
                        },
                    );
                } else {
                    self.send(id, Hub::Failure("Relay full. Try another server.".into()));
                    return;
                }
                self.send(id, Hub::Registered);
            }
            Hub::Unregister { room, lease } => {
                if self
                    .rooms
                    .get(&room)
                    .is_some_and(|r| r.host == id && r.lease == lease)
                {
                    self.rooms.remove(&room);
                }
            }
            Hub::BrowsePage { nonce, mode, page } => {
                let rooms = self
                    .rooms
                    .values()
                    .filter_map(|r| r.public.as_ref())
                    .filter(|a| mode.is_none_or(|m| a.mode == m))
                    .cloned()
                    .collect::<Vec<_>>();
                let pages = rooms.len().div_ceil(BROWSER_PAGE).max(1) as u8;
                let page = page.min(pages - 1);
                self.send(
                    id,
                    Hub::Page {
                        nonce,
                        page,
                        pages,
                        rooms: rooms
                            .into_iter()
                            .skip(page as usize * BROWSER_PAGE)
                            .take(BROWSER_PAGE)
                            .collect(),
                    },
                );
            }
            Hub::Relay {
                room,
                peer,
                target,
                bytes,
            } => {
                if bytes.len() < HEADER + 16
                    || bytes.len() > HEADER + CHUNK + 16
                    || &bytes[..4] != MAGIC
                    || bytes[4..12] != room
                    || u64::from_le_bytes(bytes[12..20].try_into().unwrap()) != peer
                {
                    return;
                }
                let Some(r) = self.rooms.get_mut(&room) else {
                    self.send(id, Hub::RoomMissing);
                    return;
                };
                let destination = if peer == 0 {
                    if id != r.host {
                        return;
                    }
                    r.peers.get(&target).copied()
                } else {
                    if target != 0 || id == r.host || r.peers.get(&peer).is_some_and(|&c| c != id) {
                        return;
                    }
                    if !r.peers.contains_key(&peer) && r.peers.len() >= MAX_PEERS - 1 {
                        return;
                    }
                    r.peers.insert(peer, id);
                    Some(r.host)
                };
                if let Some(to) = destination {
                    self.send(
                        to,
                        Hub::Relay {
                            room,
                            peer,
                            target,
                            bytes,
                        },
                    );
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pair() -> (Connection, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let stream = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        (Connection::new(server).unwrap(), stream)
    }
    #[test]
    fn stream_reassembles_split_headers_and_coalesced_messages() {
        let (mut connection, mut writer) = pair();
        let bytes = postcard::to_stdvec(&Hub::Registered).unwrap();
        let mut frame = b"VPT4".to_vec();
        frame.extend((bytes.len() as u16).to_le_bytes());
        frame.extend(bytes);
        writer.write_all(&frame[..3]).unwrap();
        assert!(connection.poll().unwrap().is_empty());
        writer.write_all(&frame[3..]).unwrap();
        writer.write_all(&frame).unwrap();
        let end = Instant::now() + Duration::from_secs(1);
        let mut count = 0;
        while count < 2 && Instant::now() < end {
            count += connection.poll().unwrap().len();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(count, 2);
    }
    #[test]
    fn oversized_headers_and_slow_outgoing_queues_are_rejected() {
        let (mut connection, mut writer) = pair();
        writer.write_all(b"VPT4\xff\xff").unwrap();
        let end = Instant::now() + Duration::from_secs(1);
        loop {
            match connection.poll() {
                Err(_) => break,
                Ok(_) => assert!(Instant::now() < end),
            }
        }
        let (mut connection, _writer) = pair();
        connection.output.resize(QUEUE_LIMIT, 0);
        assert!(connection.send(&Hub::Registered).is_err());
    }
}
