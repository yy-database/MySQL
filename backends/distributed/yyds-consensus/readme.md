# Distributed Consensus Layer

`yyds-consensus` provides the distributed consensus layer for the YYDS ecosystem.

## Features

- **Raft Protocol Implementation**: Provides Leader election, log replication, and safety guarantees.
- **Pluggable Adapters**: Supports switching between different consensus protocols via the `ConsensusAdapter` trait.
- **WAL Integration**: Directly integrates with `yyds-wal` to ensure strong consistency synchronization of data within the cluster.
- **Multi-Replica Management**: Dynamically adjusts log acknowledgement strategies based on SDR levels.

## State Machine

- `RaftAdapter`: Adapter implementing standard Raft logic, managing node state transitions and heartbeat synchronization.
