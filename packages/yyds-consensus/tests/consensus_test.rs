use yyds_consensus::*;
use yykv_types::{YYValue, Redundancy};
use yykv_event::{WalEvent, OpType};
use uuid::Uuid;
use chrono::Utc;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_raft_propose_and_step() {
    let node_id = Uuid::new_v4();
    let (tx, _rx) = mpsc::channel(10);
    let raft = RaftAdapter::new(node_id, vec![], tx);
    
    // Mocking leader state
    // Note: state is public in RaftAdapter
    {
        let mut state = raft.state().write().await;
        state.role = NodeRole::Leader;
        state.current_term = 1;
    }

    let event = WalEvent {
        id: Uuid::now_v7(),
        tenant_id: Uuid::new_v4(),
        timestamp: Utc::now(),
        table: "test".into(),
        op_type: OpType::Insert,
        sdr_level: Redundancy::SINGLE,
        data: YYValue::Int(100),
    };

    // Propose a change
    raft.propose(event.clone()).await.unwrap();
    
    {
        let state = raft.state().read().await;
        assert_eq!(state.log.len(), 1);
        assert_eq!(state.log[0].id, event.id);
    }

    // Simulate receiving AppendEntries as a follower
    let follower_id = Uuid::new_v4();
    let (tx2, _rx2) = mpsc::channel(10);
    let follower_raft = RaftAdapter::new(follower_id, vec![], tx2);
    
    let msg = ConsensusMessage::AppendEntries {
        term: 1,
        leader_id: node_id,
        prev_log_index: 0,
        prev_log_term: 0,
        entries: vec![event.clone()],
        leader_commit: 0,
    };

    follower_raft.step(msg).await.unwrap();

    {
        let state = follower_raft.state().read().await;
        assert_eq!(state.log.len(), 1);
        assert_eq!(state.log[0].id, event.id);
        assert_eq!(state.role, NodeRole::Follower);
        assert_eq!(state.current_term, 1);
    }
}
