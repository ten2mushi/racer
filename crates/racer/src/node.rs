use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::config::{RacerConfig, SelectionType};
use crate::crypto::{EcdsaSigner, KeyPair, PublicKey};
use crate::network::{PeerInfo, PeerRegistry, RacerNetwork};
use crate::plato::PlatoController;
use crate::protocol::{
    BatchedMessages, CongestionUpdate, Echo, EchoType, GossipState,
    PeerDiscovery, ProtocolMessage, ProtocolResponse, ProtocolResponseType, VectorClock,
};
use crate::util::logging::DeliveredMessageLogger;
use crate::Message;

pub struct Node<M: Message> {
    inner: Arc<NodeInner<M>>,
    router_handle: RwLock<Option<JoinHandle<()>>>,
    subscriber_handle: RwLock<Option<JoinHandle<()>>>,
    dealer_handle: RwLock<Option<JoinHandle<()>>>,
}

struct NodeInner<M: Message> {
    config: RacerConfig,
    id: String,
    keys: KeyPair,
    network: Arc<RacerNetwork>,
    peers: Arc<RwLock<PeerRegistry>>,
    gossip_state: Arc<RwLock<GossipState<M>>>,
    plato: Arc<RwLock<PlatoController>>,
    vector_clock: Arc<RwLock<VectorClock>>,
    running: Arc<AtomicBool>,
    delivered_logger: Option<DeliveredMessageLogger>,
}

impl<M> Node<M>
where
    M: Message + Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    pub async fn new(config: RacerConfig) -> Result<Self, NodeError> {
        let keys = KeyPair::generate();
        let id = config
            .node
            .id
            .clone()
            .unwrap_or_else(|| format!("node-{}", &keys.public_key().to_hex()[..8]));

        let network = Arc::new(RacerNetwork::new(
            &config.node.router_bind,
            &config.node.publisher_bind,
        ));

        let mut peers = PeerRegistry::new();
        peers.set_self_id(&id);

        let plato = PlatoController::new(config.plato.clone());
        let gossip_state = GossipState::new();

        let delivered_logger = DeliveredMessageLogger::new(&config.logging, &id);
        if delivered_logger.is_some() {
            tracing::debug!(id = %id, "delivered message logging enabled");
        }

        let inner = Arc::new(NodeInner {
            config,
            id,
            keys,
            network,
            peers: Arc::new(RwLock::new(peers)),
            gossip_state: Arc::new(RwLock::new(gossip_state)),
            plato: Arc::new(RwLock::new(plato)),
            vector_clock: Arc::new(RwLock::new(VectorClock::new())),
            running: Arc::new(AtomicBool::new(false)),
            delivered_logger,
        });

        Ok(Self {
            inner,
            router_handle: RwLock::new(None),
            subscriber_handle: RwLock::new(None),
            dealer_handle: RwLock::new(None),
        })
    }

    pub fn id(&self) -> &str {
        &self.inner.id
    }

    pub fn public_key(&self) -> PublicKey {
        self.inner.keys.public_key()
    }

    pub fn config(&self) -> &RacerConfig {
        &self.inner.config
    }

    pub fn is_running(&self) -> bool {
        self.inner.running.load(Ordering::SeqCst)
    }

    pub async fn start(&self) -> Result<(), NodeError> {
        self.inner
            .network
            .bind()
            .await
            .map_err(|e| NodeError::Network(e.to_string()))?;

        self.inner.running.store(true, Ordering::SeqCst);

        for (idx, router_addr) in self.inner.config.peers.routers.iter().enumerate() {
            let peer_id = format!("peer-{}", idx);
            if let Err(e) = self.inner.network.connect_to_peer(&peer_id, router_addr).await {
                tracing::warn!(addr = %router_addr, error = %e, "failed to connect to peer");
            }
        }

        let router_handle = self.spawn_router_listener();
        let subscriber_handle = self.spawn_subscriber_listener();
        let dealer_handle = self.spawn_dealer_listener();

        *self.router_handle.write().await = Some(router_handle);
        *self.subscriber_handle.write().await = Some(subscriber_handle);
        *self.dealer_handle.write().await = Some(dealer_handle);

        tracing::info!(
            id = %self.inner.id,
            router = %self.inner.config.node.router_bind,
            publisher = %self.inner.config.node.publisher_bind,
            "node started with background listeners"
        );

        Ok(())
    }

    pub async fn stop(&self) {
        self.inner.running.store(false, Ordering::SeqCst);

        if let Some(handle) = self.router_handle.write().await.take() {
            handle.abort();
        }
        if let Some(handle) = self.subscriber_handle.write().await.take() {
            handle.abort();
        }
        if let Some(handle) = self.dealer_handle.write().await.take() {
            handle.abort();
        }

        tracing::info!(id = %self.inner.id, "node stopped");
    }

    fn spawn_router_listener(&self) -> JoinHandle<()> {
        let inner = Arc::clone(&self.inner);
        
        tokio::spawn(async move {
            tracing::debug!(id = %inner.id, "router listener started");
            
            while inner.running.load(Ordering::SeqCst) {
                match inner.network.recv_router().await {
                    Ok((identity, content)) => {
                        if let Err(e) = Self::handle_router_message(&inner, identity, content).await {
                            tracing::warn!(id = %inner.id, error = %e, "failed to handle router message");
                        }
                    }
                    Err(e) => {
                        if inner.running.load(Ordering::SeqCst) {
                            tracing::warn!(id = %inner.id, error = %e, "router recv error");
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        }
                    }
                }
            }
            
            tracing::debug!(id = %inner.id, "router listener stopped");
        })
    }

    fn spawn_subscriber_listener(&self) -> JoinHandle<()> {
        let inner = Arc::clone(&self.inner);
        
        tokio::spawn(async move {
            tracing::debug!(id = %inner.id, "subscriber listener started");
            
            while inner.running.load(Ordering::SeqCst) {
                match inner.network.recv_subscriber().await {
                    Ok((topic, content)) => {
                        if let Err(e) = Self::handle_subscriber_message(&inner, &topic, content).await {
                            tracing::warn!(id = %inner.id, error = %e, "failed to handle subscriber message");
                        }
                    }
                    Err(e) => {
                        if inner.running.load(Ordering::SeqCst) {
                            tracing::warn!(id = %inner.id, error = %e, "subscriber recv error");
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        }
                    }
                }
            }
            
            tracing::debug!(id = %inner.id, "subscriber listener stopped");
        })
    }

    fn spawn_dealer_listener(&self) -> JoinHandle<()> {
        let inner = Arc::clone(&self.inner);
        
        tokio::spawn(async move {
            tracing::debug!(id = %inner.id, "dealer listener started");
            
            while inner.running.load(Ordering::SeqCst) {
                match inner.network.recv_dealer().await {
                    Ok((peer_id, content)) => {
                        if let Err(e) = Self::handle_dealer_message(&inner, &peer_id, content).await {
                            tracing::warn!(id = %inner.id, error = %e, "failed to handle dealer message");
                        }
                    }
                    Err(e) => {
                        if inner.running.load(Ordering::SeqCst) {
                            tracing::warn!(id = %inner.id, error = %e, "dealer recv error");
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        }
                    }
                }
            }
            
            tracing::debug!(id = %inner.id, "dealer listener stopped");
        })
    }

    async fn handle_dealer_message(
        inner: &NodeInner<M>,
        peer_id: &str,
        content: Vec<u8>,
    ) -> Result<(), NodeError> {
        let update: CongestionUpdate = serde_json::from_slice(&content)
            .map_err(|e| NodeError::Serialization(e.to_string()))?;

        tracing::debug!(
            id = %inner.id,
            from = %peer_id,
            latency = %update.current_latency,
            "received CongestionUpdate"
        );

        let mut plato = inner.plato.write().await;
        plato.record_peer_latency(update.current_latency);
        if update.recently_missed {
            plato.set_missed_delivery(true);
        }
        
        Ok(())
    }

    async fn handle_router_message(
        inner: &NodeInner<M>,
        identity: Vec<u8>,
        content: Vec<u8>,
    ) -> Result<(), NodeError> {
        let msg: ProtocolMessage<M> = serde_json::from_slice(&content)
            .map_err(|e| NodeError::Serialization(e.to_string()))?;

        let response = match msg {
            ProtocolMessage::BatchedMessages(bm) => {
                if !bm.verify_creator_signature() {
                    tracing::warn!(id = %inner.id, "received invalid creator signature on BatchedMessages");
                    CongestionUpdate::ok() // Drop silently-ish
                } else if !bm.verify_sender_signature() {
                    tracing::warn!(id = %inner.id, "received invalid sender signature on BatchedMessages");
                    CongestionUpdate::ok()
                } else {
                    Self::inbox_batched(inner, bm).await?
                }
            }
            ProtocolMessage::Echo(echo) => {
                if !echo.verify() {
                    tracing::warn!(id = %inner.id, "received invalid signature on Echo");
                    CongestionUpdate::ok()
                } else {
                    Self::inbox_echo(inner, echo).await?
                }
            }
            ProtocolMessage::PeerDiscovery(pd) => {
                Self::inbox_peer_discovery(inner, pd).await?
            }
            ProtocolMessage::Response(_) => {
                CongestionUpdate::ok()
            }
        };

        let reply = serde_json::to_vec(&response)
            .map_err(|e| NodeError::Serialization(e.to_string()))?;
        
        inner.network
            .send_router_reply(identity, reply)
            .await
            .map_err(|e| NodeError::Network(e.to_string()))?;

        Ok(())
    }

    async fn handle_subscriber_message(
        inner: &NodeInner<M>,
        _topic: &str,
        content: Vec<u8>,
    ) -> Result<(), NodeError> {
        let response: ProtocolResponse = serde_json::from_slice(&content)
            .map_err(|e| NodeError::Serialization(e.to_string()))?;

        if !response.verify() {
            tracing::warn!(id = %inner.id, "received invalid signature on ProtocolResponse");
            return Ok(());
        }

        let sender_id = response.sender_id();
        
        match response.response_type {
            ProtocolResponseType::EchoResponse => {
                tracing::debug!(
                    id = %inner.id,
                    topic = %response.topic,
                    from = %sender_id,
                    "received EchoResponse"
                );
                
                let mut should_publish_ready = false;
                {
                    let mut state = inner.gossip_state.write().await;
                    if let Some(round) = state.get_round_mut(&response.topic) {
                        round.record_echo(&sender_id);
                        if !round.echo_complete && round.echo_received.len() >= inner.config.consensus.ready_threshold {
                            round.echo_complete = true;
                            should_publish_ready = true;
                        }
                    }
                }
                
                if should_publish_ready {
                    Self::publish_ready_response(inner, &response.topic).await?;
                }
            }
            ProtocolResponseType::ReadyResponse => {
                tracing::debug!(
                    id = %inner.id,
                    topic = %response.topic,
                    from = %sender_id,
                    "received ReadyResponse"
                );
                
                let mut should_publish_ready = false;
                let mut should_deliver = false;
                let mut deliver_batch = None;

                {
                    let mut state = inner.gossip_state.write().await;
                    if let Some(round) = state.get_round_mut(&response.topic) {
                        round.record_ready(&sender_id);
                        tracing::debug!(id = %inner.id, from = %sender_id, "recorded ReadyResponse");

                        if !round.echo_complete && round.ready_received.len() >= inner.config.consensus.feedback_threshold {
                            round.echo_complete = true;
                            should_publish_ready = true;
                        }

                        if !round.delivered && round.ready_received.len() >= inner.config.consensus.delivery_threshold {
                            should_deliver = true;
                            round.ready_complete = true;
                            round.delivered = true;
                            state.mark_delivered(&response.topic);
                            deliver_batch = state.get_message(&response.topic).cloned();
                        }
                    } else {
                         tracing::warn!(id = %inner.id, topic = %response.topic, "ReadyResponse for UNKNOWN round");
                    }
                }

                if should_publish_ready {
                    Self::publish_ready_response(inner, &response.topic).await?;
                }

                if should_deliver {
                     if let Some(batch) = deliver_batch {
                        if let Some(ref logger) = inner.delivered_logger {
                            logger.log(
                                &batch.batch_id,
                                &batch.creator_ecdsa.to_hex(),
                                &batch.merkle_root,
                                batch.batch_size,
                                &batch.messages,
                            );
                        }
                     }
                     tracing::info!(id = %inner.id, hash = %response.topic, "message DELIVERED");
                }
            }
        }

        Ok(())
    }

    async fn inbox_batched(
        inner: &NodeInner<M>,
        bm: BatchedMessages<M>,
    ) -> Result<CongestionUpdate, NodeError> {
        let bm_hash = bm.compute_hash();
        
        {
            let state = inner.gossip_state.read().await;
            if state.has_message(&bm_hash) {
                return Ok(CongestionUpdate::already_received());
            }
        }

        let creator_id = bm.creator_ecdsa.to_hex()[..10].to_string();
        tracing::info!(
            id = %inner.id,
            hash = %bm_hash,
            creator = %creator_id,
            "received BatchedMessages"
        );

        {
            let mut state = inner.gossip_state.write().await;
            state.store_message(bm_hash.clone(), bm.clone());
            state.start_round(&bm_hash);
        }

        {
            let mut vc = inner.vector_clock.write().await;
            vc.merge(&bm.vector_clock);
        }

        let _ = inner.network.subscribe_topic(&format!("{}-echo", bm_hash)).await;
        let _ = inner.network.subscribe_topic(&format!("{}-ready", bm_hash)).await;
        tokio::time::sleep(Duration::from_millis(50)).await;

        Self::publish_echo_response(inner, &bm_hash).await?;

        let bm_as_sender = bm.become_sender(&inner.keys);
        let inner_clone = Arc::new(NodeInner {
            config: inner.config.clone(),
            id: inner.id.clone(),
            keys: inner.keys.clone(),
            network: Arc::clone(&inner.network),
            peers: Arc::clone(&inner.peers),
            gossip_state: Arc::clone(&inner.gossip_state),
            plato: Arc::clone(&inner.plato),
            vector_clock: Arc::clone(&inner.vector_clock),
            running: Arc::clone(&inner.running),
            delivered_logger: None, // Don't log on re-gossip
        });

        tokio::spawn(async move {
            if let Err(e) = Self::gossip_inner(&inner_clone, bm_as_sender).await {
                tracing::warn!(error = %e, "re-gossip failed");
            }
        });

        let latency = inner.plato.read().await.current_latency();
        Ok(CongestionUpdate::new(latency, false))
    }

    async fn inbox_echo(
        inner: &NodeInner<M>,
        echo: Echo,
    ) -> Result<CongestionUpdate, NodeError> {
        match echo.echo_type {
            EchoType::EchoSubscribe => {
                let state = inner.gossip_state.read().await;
                if state.has_message(&echo.topic) {
                    drop(state);
                    Self::publish_echo_response(inner, &echo.topic).await?;
                }
            }
            EchoType::ReadySubscribe => {
                let state = inner.gossip_state.read().await;
                if let Some(round) = state.get_round(&echo.topic) {
                    if round.echo_received.len() >= inner.config.consensus.ready_threshold 
                        || round.ready_received.len() >= inner.config.consensus.feedback_threshold 
                    {
                        drop(state);
                        Self::publish_ready_response(inner, &echo.topic).await?;
                    }
                }
            }
        }

        Ok(CongestionUpdate::ok())
    }

    async fn inbox_peer_discovery(
        inner: &NodeInner<M>,
        pd: PeerDiscovery,
    ) -> Result<CongestionUpdate, NodeError> {
        let peer_id = pd.ecdsa_public_key.to_hex()[..10].to_string();
        
        tracing::info!(
            id = %inner.id,
            peer = %peer_id,
            router = %pd.router_address,
            "received PeerDiscovery"
        );

        let peer = PeerInfo {
            id: peer_id.clone(),
            ecdsa_public: pd.ecdsa_public_key,
            router_address: pd.router_address.clone(),
            publisher_address: pd.publisher_address.clone(),
            reported_latency: 0.0,
            last_seen: None,
        };

        {
            let mut peers = inner.peers.write().await;
            peers.add_peer(peer);
        }

        if let Err(e) = inner.network.connect_to_peer(&peer_id, &pd.router_address).await {
            tracing::warn!(peer = %peer_id, error = %e, "failed to connect to peer router");
        }
        if let Err(e) = inner.network.subscribe_to_peer(&pd.publisher_address).await {
            tracing::warn!(peer = %peer_id, error = %e, "failed to subscribe to peer publisher");
        }

        Ok(CongestionUpdate::ok())
    }

    async fn publish_echo_response(inner: &NodeInner<M>, topic: &str) -> Result<(), NodeError> {
        let signer = EcdsaSigner::new(inner.keys.signing_key().clone());
        let mut response = ProtocolResponse::echo_response(topic, inner.keys.public_key());
        response.sign(&signer);

        let msg = serde_json::to_vec(&response)
            .map_err(|e| NodeError::Serialization(e.to_string()))?;

        inner.network
            .publish(&format!("{}-echo", topic), msg)
            .await
            .map_err(|e| NodeError::Network(e.to_string()))?;

        Ok(())
    }

    async fn publish_ready_response(inner: &NodeInner<M>, topic: &str) -> Result<(), NodeError> {
        let signer = EcdsaSigner::new(inner.keys.signing_key().clone());
        let mut response = ProtocolResponse::ready_response(topic, inner.keys.public_key());
        response.sign(&signer);

        let msg = serde_json::to_vec(&response)
            .map_err(|e| NodeError::Serialization(e.to_string()))?;

        inner.network
            .publish(&format!("{}-ready", topic), msg)
            .await
            .map_err(|e| NodeError::Network(e.to_string()))?;

        Ok(())
    }

    pub async fn add_peer(&self, peer: PeerInfo) {
        let router_addr = peer.router_address.clone();
        let pub_addr = peer.publisher_address.clone();
        let peer_id = peer.id.clone();
        
        self.inner.peers.write().await.add_peer(peer);

        if let Err(e) = self.inner.network.connect_to_peer(&peer_id, &router_addr).await {
            tracing::warn!(peer_id, error = %e, "failed to connect to peer router");
        }
        if let Err(e) = self.inner.network.subscribe_to_peer(&pub_addr).await {
            tracing::warn!(peer_id, error = %e, "failed to subscribe to peer");
        }
    }

    async fn select_peers(inner: &NodeInner<M>, n: usize) -> Vec<PeerInfo> {
        let peers = inner.peers.read().await;
        
        match inner.config.node.selection_type {
            SelectionType::Normal | SelectionType::Random | SelectionType::Poisson => {
                peers.select_random(n).into_iter().cloned().collect()
            }
        }
    }

    pub async fn submit(&self, message: M) -> Result<String, NodeError> {
        let batch_id = format!("{}-{}", self.inner.id, message.id());
        let merkle_root = crate::crypto::sha256_hex(&message.merkle_bytes());

        let signer = EcdsaSigner::new(self.inner.keys.signing_key().clone());
        
        let mut vc = self.inner.vector_clock.write().await;
        vc.increment(&self.inner.id);
        let vector_clock = vc.clone();
        drop(vc);

        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let mut bm = BatchedMessages {
            batch_id: batch_id.clone(),
            creator_ecdsa: self.inner.keys.public_key(),
            sender_ecdsa: self.inner.keys.public_key(),
            merkle_root,
            batch_size: 1,
            messages: vec![message],
            vector_clock,
            creator_signature: None,
            sender_signature: None,
            created_at,
            #[cfg(feature = "bls")]
            creator_bls: Some(self.inner.keys.bls_public_key()),
            #[cfg(feature = "bls")]
            aggregated_signature: None, // Will be set below
        };

        #[cfg(feature = "bls")]
        {
            let bls_secret = self.inner.keys.bls_secret();
            let mut signatures = Vec::with_capacity(bm.messages.len());
            
            for msg in &bm.messages {
                 let msg_bytes = serde_json::to_vec(msg)
                    .map_err(|e| NodeError::Serialization(e.to_string()))?;
                 signatures.push(bls_secret.sign(&msg_bytes));
            }
            
            if !signatures.is_empty() {
                let agg = crate::crypto::BlsSignature::aggregate(&signatures)
                    .map_err(|e| NodeError::Crypto(e.to_string()))?;
                bm.aggregated_signature = Some(agg);
            }
        }

        bm.sign_as_creator(&signer);
        bm.sign_as_sender(&signer);

        Self::gossip_inner(&self.inner, bm).await?;

        Ok(batch_id)
    }

    async fn gossip_inner(inner: &NodeInner<M>, bm: BatchedMessages<M>) -> Result<(), NodeError> {
        let hash = bm.compute_hash();
        let config = &inner.config.consensus;
        let i_am_creator = inner.keys.public_key().to_hex() == bm.creator_ecdsa.to_hex();

        tracing::debug!(
            id = %inner.id,
            hash = %hash,
            batch_id = %bm.batch_id,
            creator = %i_am_creator,
            "starting gossip"
        );

        let echo_peers = Self::select_peers(inner, config.echo_sample_size).await;
        let ready_peers = Self::select_peers(inner, config.ready_sample_size).await;

        {
            let mut state = inner.gossip_state.write().await;
            let round = state.start_round(&hash);
            for peer in &echo_peers {
                round.echo_waiting.insert(peer.id.clone());
            }
            for peer in &ready_peers {
                round.ready_waiting.insert(peer.id.clone());
            }
            state.store_message(hash.clone(), bm.clone());
        }

        if !echo_peers.is_empty() {
            let _ = inner.network.subscribe_topic(&format!("{}-echo", hash)).await;
        }
        if !ready_peers.is_empty() {
            let _ = inner.network.subscribe_topic(&format!("{}-ready", hash)).await;
        }
        
        tokio::time::sleep(Duration::from_millis(200)).await;

        let signer = EcdsaSigner::new(inner.keys.signing_key().clone());
        
        for peer in &echo_peers {
            let mut echo = Echo::new(EchoType::EchoSubscribe, &hash, inner.keys.public_key());
            echo.sign(&signer);
            let msg = serde_json::to_vec(&ProtocolMessage::<M>::Echo(echo))
                .map_err(|e| NodeError::Serialization(e.to_string()))?;
            let _ = inner.network.send_to_peer(&peer.id, msg).await;
        }

        for peer in &ready_peers {
            let mut echo = Echo::new(EchoType::ReadySubscribe, &hash, inner.keys.public_key());
            echo.sign(&signer);
            let msg = serde_json::to_vec(&ProtocolMessage::<M>::Echo(echo))
                .map_err(|e| NodeError::Serialization(e.to_string()))?;
            let _ = inner.network.send_to_peer(&peer.id, msg).await;
        }

        {
            let state = inner.gossip_state.read().await;
            if let Some(round) = state.get_round(&hash) {
                if round.ready_received.len() < config.feedback_threshold {
                    drop(state);
                    // Send BatchedMessages to echo peers
                    for peer in &echo_peers {
                        let msg = serde_json::to_vec(&ProtocolMessage::BatchedMessages(bm.clone()))
                            .map_err(|e| NodeError::Serialization(e.to_string()))?;
                        let _ = inner.network.send_to_peer(&peer.id, msg).await;
                    }
                }
            }
        }

        let timeout_secs = inner.plato.read().await.current_latency();
        let timeout = Duration::from_secs_f64(timeout_secs.max(5.0)); // Min 5 seconds
        let start = Instant::now();

        let echo_success = loop {
            {
                let state = inner.gossip_state.read().await;
                if let Some(round) = state.get_round(&hash) {
                    if round.echo_received.len() >= config.ready_threshold {
                        break true;
                    }
                }
            }
            
            if start.elapsed() > timeout {
                tracing::warn!(
                    id = %inner.id,
                    hash = %hash,
                    "echo phase timeout"
                );
                break false;
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        };

        if echo_success {
            {
                let mut state = inner.gossip_state.write().await;
                if let Some(round) = state.get_round_mut(&hash) {
                    round.echo_complete = true;
                }
            }
            Self::publish_ready_response(inner, &hash).await?;
            tracing::debug!(id = %inner.id, hash = %hash, "echo phase complete, published ReadyResponse");
        }

        let ready_success = if echo_success {
            let start = Instant::now(); // Reset timeout for this phase
            loop {
                {
                    let state = inner.gossip_state.read().await;
                    if let Some(round) = state.get_round(&hash) {
                        if round.ready_received.len() >= config.delivery_threshold {
                            break true;
                        }
                    }
                }
                
                if start.elapsed() > timeout {
                    tracing::warn!(
                        id = %inner.id,
                        hash = %hash,
                        "ready phase timeout"
                    );
                    break false;
                }
                
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        } else {
            false
        };

        if ready_success {
            let mut state = inner.gossip_state.write().await;
            if let Some(round) = state.get_round_mut(&hash) {
                round.ready_complete = true;
            }

            let already_delivered = if let Some(round) = state.get_round(&hash) {
                round.delivered
            } else {
                false
            };

            if !already_delivered {
                if let Some(ref logger) = inner.delivered_logger {
                    logger.log(
                        &bm.batch_id,
                        &bm.creator_ecdsa.to_hex(),
                        &bm.merkle_root,
                        bm.batch_size,
                        &bm.messages,
                    );
                }
                state.mark_delivered(&hash);
                tracing::info!(id = %inner.id, hash = %hash, "message DELIVERED (creator)");
            }
        } else {
            tracing::warn!(id = %inner.id, hash = %hash, "message delivery FAILED");
        }

        // Cleanup subscriptions
        let _ = inner.network.unsubscribe_topic(&format!("{}-echo", hash)).await;
        let _ = inner.network.unsubscribe_topic(&format!("{}-ready", hash)).await;

        Ok(())
    }

    pub async fn run_plato_check(&self) {
        let mut plato = self.inner.plato.write().await;
        plato.check_increasing_congestion();
        plato.check_decreasing_congestion();
    }

    pub async fn plato_stats(&self) -> crate::plato::PlatoStats {
        self.inner.plato.read().await.stats()
    }

    pub async fn vector_clock(&self) -> VectorClock {
        self.inner.vector_clock.read().await.clone()
    }

    pub async fn gossip_stats(&self) -> GossipStats {
        let state = self.inner.gossip_state.read().await;
        GossipStats {
            active_rounds: state.active_rounds(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GossipStats {
    pub active_rounds: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("protocol error: {0}")]
    Protocol(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use racer_core::message::DefaultMessage;

    #[tokio::test]
    async fn test_node_creation() {
        let config = RacerConfig::minimal();
        let node = Node::<DefaultMessage>::new(config).await.unwrap();

        assert!(!node.id().is_empty());
        assert!(!node.is_running());
    }

    #[tokio::test]
    async fn test_node_start_stop() {
        let config = RacerConfig::minimal();
        let node = Node::<DefaultMessage>::new(config).await.unwrap();

        node.start().await.unwrap();
        assert!(node.is_running());

        node.stop().await;
        assert!(!node.is_running());
    }
}
