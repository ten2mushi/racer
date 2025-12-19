use racer::network::RacerNetwork;
use zeromq::{Socket, SocketRecv, SocketSend};

#[tokio::test]
async fn test_dealer_feedback_loop() {

    let network = RacerNetwork::new("tcp://127.0.0.1:0", "tcp://127.0.0.1:0");
    network.bind().await.expect("failed to bind network");

    let mut peer_router = zeromq::RouterSocket::new();

    let endpoint = peer_router.bind("tcp://127.0.0.1:0").await.expect("failed to bind peer router");
    let peer_addr = endpoint.to_string(); // Get the actual bound address

    network.connect_to_peer("peer-1", &peer_addr).await.expect("failed to connect");

    let content_out = b"hello peer".to_vec();
    network.send_to_peer("peer-1", content_out.clone()).await.expect("failed to send");

    // Peer receives
    let msg = peer_router.recv().await.expect("peer failed to recv");
    let frames = msg.into_vec();
    assert!(frames.len() >= 2); // Identity + Content
    let identity = frames[0].clone();
    let content_in = frames[1].clone();
    assert_eq!(content_in, content_out);

    // 5. Peer sends feedback (CongestionUpdate simulation) back to Node
    let feedback_content = b"congestion_update_payload".to_vec();
    let mut reply = zeromq::ZmqMessage::from(identity); // Send to specific dealer
    reply.push_back(feedback_content.clone().into());
    peer_router.send(reply).await.expect("peer failed to send reply");


    let (from_peer, received_content) = network.recv_dealer().await.expect("node failed to recv dealer msg");
    
    assert_eq!(from_peer, "peer-1");
    assert_eq!(received_content, feedback_content);
}
