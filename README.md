# RACER

Peer-to-peer consensus protocol designed specifically for IoT networks.
Provides a Byzantine fault-tolerant foundation for distributed sensor networks without requiring a central leader.


The primary goal of RACER is to enable secure and consistent data synchronization across a mesh of unreliable IoT devices. It is built to handle:
- Tolerates packet loss and node churn.
- Optimized for low overhead on embedded devices.
- No single point of failure (leaderless).
- Verifiable integrity using cryptographic signatures.

## Core Components

RACER is built on two novel protocols:

### SDPE (Sequenced Probabilistic Double Echo)
**SPDE** is the consensus engine. It uses a leaderless directed acyclic graph (DAG) approach where nodes gossip "ECHO" and "READY" messages to reach agreement.
- **Phases**: The protocol moves through `ECHO` and `READY` sampling phases to commit messages.
- **Probabilistic guarantees**: Uses sampling thresholds to achieve safety and liveness with high probability, reducing message complexity compared to determinstic BFT algorithms.

### PLATO (Peer-assisted Latency-Aware Traffic Optimisation)
**PLATO** is the adaptive congestion control and routing mechanism. It dynamically adjusts network behavior based on real-time conditions.
- **Latency Targets**: Maintains target limits for end-to-end message delivery time.
- **Rate Limiting**: Uses **RSI (Relative Strength Index)** and **Savitzky-Golay filters** to smooth traffic signals and detect congestion (Overbought/Oversold states), throttling publishing frequencies accordingly.

## Configuration

RACER uses **TOML** for all configuration.

### TOML Configuration Mechanism
The library uses a structured TOML file to configure the Node, Consensus (SPDE), Plato (Congestion), and Peers.

Example structure (`racer.toml`):
```toml
[node]
router_bind = "tcp://0.0.0.0:20001"
# selection_type = "normal" # Peer selection strategy

[consensus]
echo_sample_size = 6     # Number of peers to query in ECHO phase
ready_sample_size = 6    # Number of peers to query in READY phase
ready_threshold = 4      # Quorum for READY
delivery_threshold = 6   # Threshold to commit/deliver

[plato]
target_latency_secs = 2.5
target_publishing_frequency_secs = 2.5

[peers]
routers = ["tcp://192.168.1.5:20001"]
```

### Custom Message Types
The library allows defining custom message payloads (e.g., Sensor Readings) directly in TOML, which are then compiled into Rust structs with validation via the `racer-macros` crate.

**1. Define message in TOML (`examples/config/sensor.toml`):**
```toml
[message]
name = "SensorReading"

[[message.fields]]
name = "temperature"
type = "f64"
min = -40.0
max = 100.0
```

**2. Use in Rust:**
```rust
use racer::prelude::*;

#[racer_message("examples/config/sensor.toml")]
pub struct SensorReading;
```
This generates a struct `SensorReading` that implements the `Message` trait and enforces the defined constraints (e.g., min/max values).

## BLS Feature Gate

The library includes an optional **BLS (Boneh-Lynn-Shacham)** signature aggregation feature.
- **Flag**: `bls`
- **Effect**:
  - Enables the `blst` dependency (requires C toolchain).
  - Activates `BlsPublicKey`, `BlsSecretKey`, and `BlsSignature` in the `crypto` module.
  - Allows signature aggregation to reduce bandwidth usage in large networks.
- **Usage**: Enable in `Cargo.toml` or via `--features bls`.

## Entry Points

### 1. The Library
The core Rust crate for embedding RACER into applications.
```rust
use racer::prelude::*;
// See crates/racer/src/lib.rs for Quick Start
```

### 2. The CLI
A standalone binary for running a RACER node, managing keys, or generating configuration.
- **Source**: `crates/racer/src/bin/main.rs`
- **Requires**: `cli` feature.
- **Commands**:
  - `racer run`: Start a node.
  - `racer keygen`: Generate cryptographic keys.
  - `racer config`: Generate default configuration.

**Build/Run**:
```bash
cargo run -p racer --features cli -- run --help
```

### 3. Network Sensors Example
A complete demonstration of a sensor mesh network.
- **Source**: `examples/src/sensor_network.rs`
- **Features**:
  - Spawns multiple nodes in a single process.
  - Connects them in a full mesh.
  - Simulates sensor data generation (`SensorReading`).
  - Verifies consensus consistency (High-level consistency check).
- **Run (without BLS)**:
  ```bash
  cargo run -p racer-examples --bin sensor-network -- examples/config/sensor_network.toml
  ```
- **Run (with BLS)**:
  ```bash
  cargo run -p racer-examples --bin sensor-network --features bls -- examples/config/sensor_network.toml
  ```
