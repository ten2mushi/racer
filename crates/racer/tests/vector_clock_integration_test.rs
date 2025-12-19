use racer::config::RacerConfig;
use racer::crypto::{EcdsaSigner, KeyPair};
use racer::protocol::{BatchedMessages, ProtocolMessage, VectorClock};
use racer::node::Node;
use racer_core::message::DefaultMessage;
use zeromq::{Socket, SocketSend};

#[tokio::test]
async fn test_vector_clock_merge_on_receive() {
    // 1. Setup Node
    let mut config = RacerConfig::minimal();
    config.node.router_bind = "tcp://127.0.0.1:0".to_string(); // Random port
    config.node.publisher_bind = "tcp://127.0.0.1:0".to_string();
    
    let node = Node::<DefaultMessage>::new(config).await.expect("failed to create node");
    node.start().await.expect("failed to start node");
    
    let port = 56000 + (std::process::id() % 1000) as u16;
    let router_addr = format!("tcp://127.0.0.1:{}", port);
    
    // Re-create node with known port
    let mut config = RacerConfig::minimal();
    config.node.router_bind = router_addr.clone();
    config.node.publisher_bind = format!("tcp://127.0.0.1:{}", port + 1);
    
    let node = Node::<DefaultMessage>::new(config).await.expect("failed to create node");
    node.start().await.expect("failed to start node");

    // 2. Prepare External Peer
    let mut dealer = zeromq::DealerSocket::new();
    dealer.connect(&router_addr).await.expect("failed to connect dealer");

    // 3. Create a Signed BatchedMessage with Complex Vector Clock
    let peer_keys = racer::crypto::KeyPair::generate();
    let signer = EcdsaSigner::new(peer_keys.signing_key().clone());
    let peer_id = peer_keys.public_key().to_hex()[..10].to_string();

    let mut vc = VectorClock::new();
    vc.set(&peer_id, 10);
    vc.set("some_other_node", 99);

    let mut bm = BatchedMessages {
        batch_id: format!("{}-batch-1", peer_id),
        creator_ecdsa: peer_keys.public_key(),
        sender_ecdsa: peer_keys.public_key(), // Sending as creator
        merkle_root: "dummy_root".to_string(),
        batch_size: 1,
        messages: vec![],
        vector_clock: vc.clone(),
        creator_signature: None,
        sender_signature: None,
        created_at: 1234567890,
        #[cfg(feature = "bls")]
        creator_bls: None,
        #[cfg(feature = "bls")]
        aggregated_signature: None,
    };

    bm.sign_as_creator(&signer);
    bm.sign_as_sender(&signer); // Since we act as sender too

    // 4. Send to Node
    let msg = ProtocolMessage::<DefaultMessage>::BatchedMessages(bm);
    let payload = serde_json::to_vec(&msg).expect("failed to serialize");
    
    // ZMQ Router expects [Identity, Content]. Dealer just sends [Content] (and sets identity on connect if needed, or Router generates).
    // Send payload.
    dealer.send(zeromq::ZmqMessage::from(payload)).await.expect("failed to send frame");

    // 5. Wait for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 6. Verify Node's Vector Clock
    let node_vc = node.vector_clock().await;
    
    // Node should have merged the VC
    assert_eq!(node_vc.get(&peer_id), 10);
    assert_eq!(node_vc.get("some_other_node"), 99);
    
    node.stop().await;
}
