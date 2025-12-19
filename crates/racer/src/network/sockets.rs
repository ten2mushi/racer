//! ZeroMQ socket management using the Actor Model.
//! 
//! To prevent deadlocks caused by sharing sockets with RwLocks, we isolate each socket
//! in its own background task (Actor). The `RacerNetwork` struct acts as a controller
//! that communicates with these actors via MPSC channels.

use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex, RwLock};
use zeromq::{DealerSocket, PubSocket, RouterSocket, Socket, SubSocket, SocketRecv, SocketSend};

const CHANNEL_BUFFER: usize = 100;


#[derive(Debug)]
enum RouterCommand {
    Bind(String),
    SendReply(Vec<u8>, Vec<u8>), // identity, content
}

#[derive(Debug)]
enum SubscriberCommand {
    Connect(String),
    Subscribe(String),
    Unsubscribe(String),
}

#[derive(Debug)]
enum PublisherCommand {
    Bind(String),
    Publish(String, Vec<u8>), // topic, content
}

#[derive(Debug)]
enum DealerCommand {
    Connect(String, String), // peer_id, address
    Send(String, Vec<u8>),   // peer_id, content
}

#[derive(Debug)]
enum DealerWorkerCommand {
    Connect(String), // address
    Send(Vec<u8>),   // content
}


pub struct RacerNetwork {
    // Command channels to actors
    router_tx: mpsc::Sender<RouterCommand>,
    publisher_tx: mpsc::Sender<PublisherCommand>,
    subscriber_tx: mpsc::Sender<SubscriberCommand>,
    dealer_tx: mpsc::Sender<DealerCommand>,

    router_rx: Arc<Mutex<mpsc::Receiver<(Vec<u8>, Vec<u8>)>>>,
    subscriber_rx: Arc<Mutex<mpsc::Receiver<(String, Vec<u8>)>>>,
    dealer_rx: Arc<Mutex<mpsc::Receiver<(String, Vec<u8>)>>>, // (peer_id, content)

    router_bind: String,
    publisher_bind: String,
    
    subscribed_topics: Arc<RwLock<HashSet<String>>>,
}

impl RacerNetwork {
    pub fn new(router_bind: impl Into<String>, publisher_bind: impl Into<String>) -> Self {
        let router_bind = router_bind.into();
        let publisher_bind = publisher_bind.into();

        let (router_cmd_tx, router_cmd_rx) = mpsc::channel(CHANNEL_BUFFER);
        let (router_msg_tx, router_msg_rx) = mpsc::channel(CHANNEL_BUFFER);

        let (pub_cmd_tx, pub_cmd_rx) = mpsc::channel(CHANNEL_BUFFER);

        let (sub_cmd_tx, sub_cmd_rx) = mpsc::channel(CHANNEL_BUFFER);
        let (sub_msg_tx, sub_msg_rx) = mpsc::channel(CHANNEL_BUFFER);

        let (dealer_cmd_tx, dealer_cmd_rx) = mpsc::channel(CHANNEL_BUFFER);
        let (dealer_msg_tx, dealer_msg_rx) = mpsc::channel(CHANNEL_BUFFER);

        tokio::spawn(router_actor(router_cmd_rx, router_msg_tx));
        tokio::spawn(publisher_actor(pub_cmd_rx));
        tokio::spawn(subscriber_actor(sub_cmd_rx, sub_msg_tx));
        tokio::spawn(dealer_actor(dealer_cmd_rx, dealer_msg_tx));

        Self {
            router_tx: router_cmd_tx,
            publisher_tx: pub_cmd_tx,
            subscriber_tx: sub_cmd_tx,
            dealer_tx: dealer_cmd_tx,
            router_rx: Arc::new(Mutex::new(router_msg_rx)),
            subscriber_rx: Arc::new(Mutex::new(sub_msg_rx)),
            dealer_rx: Arc::new(Mutex::new(dealer_msg_rx)),
            router_bind,
            publisher_bind,
            subscribed_topics: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub async fn bind(&self) -> Result<(), NetworkError> {
        self.router_tx
            .send(RouterCommand::Bind(self.router_bind.clone()))
            .await
            .map_err(|_| NetworkError::Send("Router actor closed".into()))?;

        self.publisher_tx
            .send(PublisherCommand::Bind(self.publisher_bind.clone()))
            .await
            .map_err(|_| NetworkError::Send("Publisher actor closed".into()))?;

        tracing::info!(
            router = %self.router_bind,
            publisher = %self.publisher_bind,
            "network sockets bound"
        );

        Ok(())
    }

    pub async fn connect_to_peer(&self, peer_id: &str, address: &str) -> Result<(), NetworkError> {
        self.dealer_tx
            .send(DealerCommand::Connect(peer_id.to_string(), address.to_string()))
            .await
            .map_err(|_| NetworkError::Send("Dealer actor closed".into()))?;
        Ok(())
    }

    pub async fn subscribe_to_peer(&self, address: &str) -> Result<(), NetworkError> {
        self.subscriber_tx
            .send(SubscriberCommand::Connect(address.to_string()))
            .await
            .map_err(|_| NetworkError::Send("Subscriber actor closed".into()))?;
        Ok(())
    }

    pub async fn subscribe_topic(&self, topic: &str) -> Result<(), NetworkError> {
        self.subscriber_tx
            .send(SubscriberCommand::Subscribe(topic.to_string()))
            .await
            .map_err(|_| NetworkError::Send("Subscriber actor closed".into()))?;
        
        self.subscribed_topics.write().await.insert(topic.to_string());
        Ok(())
    }

    pub async fn unsubscribe_topic(&self, topic: &str) -> Result<(), NetworkError> {
        self.subscriber_tx
            .send(SubscriberCommand::Unsubscribe(topic.to_string()))
            .await
            .map_err(|_| NetworkError::Send("Subscriber actor closed".into()))?;

        self.subscribed_topics.write().await.remove(topic);
        Ok(())
    }

    pub async fn is_subscribed(&self, topic: &str) -> bool {
        self.subscribed_topics.read().await.contains(topic)
    }

    pub async fn send_to_peer(&self, peer_id: &str, message: Vec<u8>) -> Result<(), NetworkError> {
        self.dealer_tx
            .send(DealerCommand::Send(peer_id.to_string(), message))
            .await
            .map_err(|_| NetworkError::Send("Dealer actor closed".into()))?;
        Ok(())
    }

    pub async fn publish(&self, topic: &str, message: Vec<u8>) -> Result<(), NetworkError> {
        self.publisher_tx
            .send(PublisherCommand::Publish(topic.to_string(), message))
            .await
            .map_err(|_| NetworkError::Send("Publisher actor closed".into()))?;
        Ok(())
    }

    pub async fn recv_router(&self) -> Result<(Vec<u8>, Vec<u8>), NetworkError> {
        let mut rx = self.router_rx.lock().await;
        rx.recv()
            .await
            .ok_or_else(|| NetworkError::Recv("Router actor closed".into()))
    }

    pub async fn recv_subscriber(&self) -> Result<(String, Vec<u8>), NetworkError> {
        let mut rx = self.subscriber_rx.lock().await;
        rx.recv()
            .await
            .ok_or_else(|| NetworkError::Recv("Subscriber actor closed".into()))
    }

    pub async fn recv_dealer(&self) -> Result<(String, Vec<u8>), NetworkError> {
        let mut rx = self.dealer_rx.lock().await;
        rx.recv()
            .await
            .ok_or_else(|| NetworkError::Recv("Dealer actor closed".into()))
    }

    pub async fn send_router_reply(
        &self,
        identity: Vec<u8>,
        message: Vec<u8>,
    ) -> Result<(), NetworkError> {
        self.router_tx
            .send(RouterCommand::SendReply(identity, message))
            .await
            .map_err(|_| NetworkError::Send("Router actor closed".into()))?;
        Ok(())
    }
}

async fn router_actor(
    mut commands: mpsc::Receiver<RouterCommand>,
    msg_sender: mpsc::Sender<(Vec<u8>, Vec<u8>)>,
) {
    let mut socket = RouterSocket::new();

    loop {
        tokio::select! {
            // 1. Handle Commands
            cmd = commands.recv() => {
                match cmd {
                    Some(RouterCommand::Bind(addr)) => {
                        if let Err(e) = socket.bind(&addr).await {
                            tracing::error!(error = %e, "Router bind failed");
                        }
                    }
                    Some(RouterCommand::SendReply(identity, content)) => {
                        let mut msg = zeromq::ZmqMessage::from(identity);
                        msg.push_back(content.into());
                        if let Err(e) = socket.send(msg).await {
                            tracing::error!(error = %e, "Router send failed");
                        }
                    }
                    None => break, // Channel closed
                }
            }
            
            res = socket.recv() => {
                match res {
                    Ok(msg) => {
                        let frames: Vec<_> = msg.into_vec();
                        if frames.len() >= 2 {
                            let identity = frames[0].to_vec();
                            let content = frames.get(1).map(|f| f.to_vec()).unwrap_or_default();
                            // If receiver is full or dropped, we just log and continue
                            if let Err(_) = msg_sender.send((identity, content)).await {
                                tracing::debug!("Router msg receiver closed");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Router recv failed");
                    }
                }
            }
        }
    }
}

async fn subscriber_actor(
    mut commands: mpsc::Receiver<SubscriberCommand>,
    msg_sender: mpsc::Sender<(String, Vec<u8>)>,
) {
    let mut socket = SubSocket::new();

    loop {
        tokio::select! {
            cmd = commands.recv() => {
                match cmd {
                    Some(SubscriberCommand::Connect(addr)) => {
                        if let Err(e) = socket.connect(&addr).await {
                            tracing::error!(error = %e, "Subscriber connect failed");
                        } else {
                            tracing::debug!(addr, "Subscriber connected");
                        }
                    }
                    Some(SubscriberCommand::Subscribe(topic)) => {
                        if let Err(e) = socket.subscribe(&topic).await {
                            tracing::error!(error = %e, "Subscriber subscribe failed");
                        }
                    }
                    Some(SubscriberCommand::Unsubscribe(topic)) => {
                        if let Err(e) = socket.unsubscribe(&topic).await {
                            tracing::error!(error = %e, "Subscriber unsubscribe failed");
                        }
                    }
                    None => break,
                }
            }

            res = socket.recv() => {
                match res {
                    Ok(msg) => {
                        let frames: Vec<_> = msg.into_vec();
                        if frames.len() >= 2 {
                            let topic = String::from_utf8_lossy(&frames[0]).to_string();
                            let content = frames[1].to_vec();
                            if let Err(_) = msg_sender.send((topic, content)).await {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Subscriber recv failed");
                    }
                }
            }
        }
    }
}

async fn publisher_actor(mut commands: mpsc::Receiver<PublisherCommand>) {
    let mut socket = PubSocket::new();

    while let Some(cmd) = commands.recv().await {
        match cmd {
            PublisherCommand::Bind(addr) => {
                if let Err(e) = socket.bind(&addr).await {
                    tracing::error!(error = %e, "Publisher bind failed");
                }
            }
            PublisherCommand::Publish(topic, content) => {
                let mut msg = zeromq::ZmqMessage::from(topic.as_bytes().to_vec());
                msg.push_back(content.into());
                if let Err(e) = socket.send(msg).await {
                    tracing::error!(error = %e, "Publisher send failed");
                }
            }
        }
    }
}

async fn dealer_actor(
    mut commands: mpsc::Receiver<DealerCommand>,
    msg_sender: mpsc::Sender<(String, Vec<u8>)>,
) {
    let mut workers: std::collections::HashMap<String, mpsc::Sender<DealerWorkerCommand>> = std::collections::HashMap::new();

    while let Some(cmd) = commands.recv().await {
        match cmd {
            DealerCommand::Connect(peer_id, addr) => {
                if !workers.contains_key(&peer_id) {
                    let (tx, rx) = mpsc::channel(CHANNEL_BUFFER);
                    tokio::spawn(dealer_worker(peer_id.clone(), addr.clone(), rx, msg_sender.clone()));
                    workers.insert(peer_id, tx);
                }
            }
            DealerCommand::Send(peer_id, content) => {
                if let Some(tx) = workers.get(&peer_id) {
                    if let Err(e) = tx.send(DealerWorkerCommand::Send(content)).await {
                        tracing::error!(peer_id, error = %e, "Failed to send command to dealer worker");
                        workers.remove(&peer_id); // Remove dead worker
                    }
                } else {
                    tracing::warn!(peer_id, "Dealer worker not found for peer");
                }
            }
        }
    }
}

async fn dealer_worker(
    peer_id: String,
    address: String,
    mut commands: mpsc::Receiver<DealerWorkerCommand>,
    msg_sender: mpsc::Sender<(String, Vec<u8>)>,
) {
    let mut socket = DealerSocket::new();
    
    if let Err(e) = socket.connect(&address).await {

    } else {
        tracing::debug!(peer_id, address, "Dealer worker connected");
    }

    loop {
        tokio::select! {
            cmd = commands.recv() => {
                match cmd {
                    Some(DealerWorkerCommand::Connect(new_addr)) => {
                         if let Err(e) = socket.connect(&new_addr).await {
                            tracing::error!(peer_id, error = %e, "Dealer worker reconnect failed");
                        }
                    }
                    Some(DealerWorkerCommand::Send(content)) => {
                        let msg = zeromq::ZmqMessage::from(content);
                        if let Err(e) = socket.send(msg).await {
                            tracing::error!(peer_id, error = %e, "Dealer worker send failed");
                        }
                    }
                    None => break, // Channel closed
                }
            }

            res = socket.recv() => {
                match res {
                    Ok(msg) => {

                        let frames: Vec<_> = msg.into_vec();
                        if let Some(content_frame) = frames.last() {
                             if let Err(_) = msg_sender.send((peer_id.clone(), content_frame.to_vec())).await {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(peer_id, error = %e, "Dealer worker recv failed");
                    }
                }
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("failed to bind socket: {0}")]
    Bind(String),
    #[error("failed to connect: {0}")]
    Connect(String),
    #[error("failed to subscribe: {0}")]
    Subscribe(String),
    #[error("failed to send: {0}")]
    Send(String),
    #[error("failed to receive: {0}")]
    Recv(String),
    #[error("peer not found: {0}")]
    PeerNotFound(String),
    #[error("invalid message: {0}")]
    InvalidMessage(String),
}
