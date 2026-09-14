use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{
        Arc,
        atomic::{AtomicI32, AtomicU32, Ordering},
        nonpoison,
    },
};

use log::{info, warn};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    sync::Mutex as TokioMutex,
    task::JoinSet,
};

use crate::{
    announce::{Announce, AnnounceError, AnnounceEvent, AnnounceOpts, AnnounceResponse},
    network::{delegate::NetworkDelegate, peer_connection::InactivePeerConnection},
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
    announcer: TokioMutex<Box<dyn Announce + Send + Sync>>,

    active_connections: AtomicI32,

    tasks: nonpoison::Mutex<JoinSet<()>>,

    // { info_hash: delegate}
    delegates: TokioMutex<HashMap<[u8; 20], Arc<NetworkDelegate>>>,
}

impl NetworkContext {
    pub async fn new<A: Announce + Send + Sync + 'static>(
        listener_port: u16,
        announcer: A,
    ) -> Option<Self> {
        Some(Self {
            ip: None,
            tasks: nonpoison::Mutex::new(JoinSet::new()),
            listener_port,
            active_connections: 0.into(),
            announcer: TokioMutex::new(Box::new(announcer)),
            delegates: TokioMutex::new(HashMap::new()),
        })
    }
    pub(crate) async fn announce(
        &self,
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
        self.announcer
            .lock()
            .await
            .announce(announce_url, opts)
            .await
    }
    pub(crate) async fn close_delegate(&self, id: &[u8; 20]) {
        let Some(_delegate) = self.delegates.lock().await.remove(id) else {
            log::error!("NetworkContext::close_delegate() was called on a non-existent delegate");
            return;
        };
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

        Arc::clone(&delegate).register_connection(peer).await;
    }
    pub(crate) fn start_listener(self: Arc<NetworkContext>, listener: TcpListener) {
        Arc::clone(&self).tasks.lock().spawn(async move {
            loop {
                let Ok((stream, remote_socket)) = listener.accept().await else {
                    return;
                };
                info!("{} attempting to connect...", remote_socket);
                self.handle_incoming_connection(stream, remote_socket).await;
            }
        });
    }

    pub(crate) async fn add_delegate(
        self: Arc<Self>,
        info_hash: [u8; 20],
        peer_id: [u8; 20],
    ) -> Option<Arc<NetworkDelegate>> {
        if self.delegates.lock().await.contains_key(&info_hash) {
            return None;
        }

        let (delegate, _delegate_io) = NetworkDelegate::new(Arc::clone(&self), info_hash, peer_id);

        self.delegates
            .lock()
            .await
            .insert(info_hash, Arc::clone(&delegate));

        Some(delegate)
    }

    pub(crate) fn add_active_connections(&self, connections: i32) {
        self.active_connections
            .fetch_add(connections, Ordering::SeqCst);
    }
}

// no idea how to name this lol

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
