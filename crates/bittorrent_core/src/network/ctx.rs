use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

use super::peer_connection::{Peer, PeerConnection};

use crate::{
    announce::{Announce, AnnounceError, AnnounceEvent, AnnounceOpts, AnnounceResponse},
    network::{Queue, message::Message, peer_connection::InactivePeerConnection},
};

pub enum HandshakeError {
    UnknownProtocol,
    UnknownInfoHash,
    WrongLength,
}

pub struct MinimalOpts {
    pub uploaded: usize,
    pub downloaded: usize,
    pub left: usize,
    pub event: AnnounceEvent,
    pub compact: bool,
}

pub struct ExtendedOpts {
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
    pub uploaded: usize,
    pub downloaded: usize,
    pub left: usize,
    pub event: AnnounceEvent,
    pub compact: bool,
}

pub(crate) struct NetworkContext {
    ip: Option<IpAddr>,
    listener_port: u16,
    announcer: Box<dyn Announce + Send + Sync>,

    // { info_hash: delegate}
    delegates: Arc<Mutex<HashMap<[u8; 20], Arc<Mutex<NetworkDelegate>>>>>,
}

impl NetworkContext {
    pub async fn new<A: Announce + Send + Sync + 'static>(
        listener_port: u16,
        announcer: A,
    ) -> Option<Self> {
        Some(Self {
            ip: None,
            listener_port,
            announcer: Box::new(announcer),
            delegates: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    pub(crate) async fn announce(
        &mut self,
        announce_url: &str,
        opts: ExtendedOpts,
    ) -> Result<AnnounceResponse, AnnounceError> {
        let opts = AnnounceOpts {
            info_hash: opts.info_hash,
            peer_id: opts.peer_id,
            ip: self.ip,
            port: self.listener_port,
            uploaded: opts.uploaded,
            downloaded: opts.downloaded,
            left: opts.left,
            event: opts.event,
            compact: opts.compact,
        };
        self.announcer.announce(announce_url, opts).await
    }

    pub(crate) async fn handle_incoming_connection(
        &self,
        mut stream: TcpStream,
        remote_socket: SocketAddr,
    ) {
        let mut buf: Vec<u8> = Vec::with_capacity(100);
        let Ok(_) = stream.read(&mut buf).await else {
            return;
        };
        let Ok((info_hash, peer_id)) = parse_handshake(&buf) else {
            return;
        };

        let delegates = self.delegates.lock().await;

        let Some(delegate) = delegates.get(&info_hash) else {
            return;
        };

        let peer = InactivePeerConnection::new(peer_id, remote_socket, stream);

        delegate.lock().await.register_connection(peer).await;
    }

    pub(crate) async fn add_delegate(
        ctx: Arc<Mutex<NetworkContext>>,
        info_hash: [u8; 20],
        peer_id: [u8; 20],
    ) -> Option<Arc<Mutex<NetworkDelegate>>> {
        if ctx
            .lock()
            .await
            .delegates
            .lock()
            .await
            .contains_key(&info_hash)
        {
            return None;
        }
        let delegate = Arc::new(Mutex::new(NetworkDelegate::new(
            Arc::clone(&ctx),
            info_hash,
            peer_id,
        )));

        ctx.lock()
            .await
            .delegates
            .lock()
            .await
            .insert(info_hash, Arc::clone(&delegate));

        Some(delegate)
    }
}

// no idea how to name this lol
// TODO: refactor into separate file
pub(crate) struct NetworkDelegate {
    ctx: Arc<Mutex<NetworkContext>>,
    info_hash: [u8; 20],
    peer_id: [u8; 20],

    raw_peers: Vec<Peer>,

    last_announce: Option<AnnounceResponse>,

    incoming_buffer: Arc<Mutex<Queue<Message>>>,

    // { peer_id: index_in_peer_connections}
    connected_peers: HashMap<Vec<u8>, PeerConnection>,
}

impl NetworkDelegate {
    fn new(ctx: Arc<Mutex<NetworkContext>>, info_hash: [u8; 20], peer_id: [u8; 20]) -> Self {
        Self {
            ctx,
            raw_peers: Vec::new(),
            incoming_buffer: Default::default(),
            last_announce: None,
            connected_peers: HashMap::new(),
            info_hash,
            peer_id,
        }
    }

    pub(crate) async fn announce(
        &mut self,
        announce_url: &str,
        opts: MinimalOpts,
    ) -> Result<(), AnnounceError> {
        let MinimalOpts {
            uploaded,
            downloaded,
            left,
            event,
            compact,
        } = opts;
        self.last_announce = Some(
            self.ctx
                .lock()
                .await
                .announce(
                    announce_url,
                    ExtendedOpts {
                        info_hash: self.info_hash,
                        peer_id: self.peer_id,
                        uploaded,
                        downloaded,
                        left,
                        event,
                        compact,
                    },
                )
                .await?,
        );
        println!("{:#?}", self.last_announce);
        Ok(())
    }
    pub(crate) async fn register_connection(&mut self, connection: InactivePeerConnection) {
        if self.connected_peers.contains_key(connection.id() as &[u8]) {
            return;
        }
        let mut connection = connection.activate(Arc::clone(&self.incoming_buffer));

        connection
            .send_handshake(&self.info_hash, &self.peer_id)
            .await;

        self.connected_peers
            .insert(connection.id().to_vec(), connection);
    }
}

pub(crate) fn parse_handshake(
    mut handshake_data: &[u8],
) -> Result<([u8; 20], [u8; 20]), HandshakeError> {
    let pstrlen = *handshake_data.get(0).ok_or(HandshakeError::WrongLength)? as usize;
    if pstrlen != 19 {
        return Err(HandshakeError::UnknownProtocol);
    }

    handshake_data = &handshake_data[1..];

    let pstr = handshake_data
        .get(0..pstrlen)
        .ok_or(HandshakeError::WrongLength)?;

    let pstr = String::from_utf8_lossy(pstr);

    if pstr != "BitTorrent protocol" {
        return Err(HandshakeError::UnknownProtocol);
    }

    if handshake_data.len() != 49 + pstrlen as usize {
        return Err(HandshakeError::WrongLength);
    }

    handshake_data = &handshake_data[pstrlen..];

    let reserved = &handshake_data[0..8];

    if reserved != [0; 8] {
        return Err(HandshakeError::UnknownProtocol);
    }
    handshake_data = &handshake_data[pstrlen..];
    let info_hash: [u8; 20] = handshake_data[0..20].try_into().unwrap();
    handshake_data = &handshake_data[20..];
    let peer_id: [u8; 20] = handshake_data[0..20].try_into().unwrap();

    Ok((info_hash, peer_id))
}

pub async fn create_tcp_listener(port: Option<u16>) -> Option<(TcpListener, u16)> {
    let mut listener: Option<TcpListener> = None;
    let mut good_port: Option<u16> = None;
    if let Some(port) = port
        && let Ok(tcp_listener) = TcpListener::bind(("127.0.0.1", port)).await
    {
        listener = Some(tcp_listener);
        good_port = Some(port);
    } else {
        for port in 6882u16..=6889 {
            if let Ok(tcp_listener) = TcpListener::bind(("127.0.0.1", port)).await {
                listener = Some(tcp_listener);
                good_port = Some(port);
                break;
            }
        }
    }

    let Some(listener) = listener else {
        return None;
    };
    let Some(port) = good_port else {
        return None;
    };

    Some((listener, port))
}

pub(crate) fn start_listener(ctx: Arc<Mutex<NetworkContext>>, listener: TcpListener) {
    tokio::spawn(async move {
        loop {
            let Ok((stream, remote_socket)) = listener.accept().await else {
                return;
            };
            ctx.lock()
                .await
                .handle_incoming_connection(stream, remote_socket)
                .await;
        }
    });
}
