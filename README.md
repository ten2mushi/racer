# RACER

https://www.mdpi.com/2227-7080/13/4/151

peer-to-peer consensus protocol for IoT networks.

uses a structured toml file to configure the node, consensus (SPDE), congestion (plato), and peers.

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

defining custom message payloads (e.g., Sensor Readings) directly in toml

- define message in TOML (`examples/config/sensor.toml`):**
```toml
[message]
name = "SensorReading"

[[message.fields]]
name = "temperature"
type = "f64"
min = -40.0
max = 100.0
```

- use
```rust
use racer::prelude::*;

#[racer_message("examples/config/sensor.toml")]
pub struct SensorReading;
```
-> this generates a struct `SensorReading` which implements the `Message` trait

## BLS Feature Gate

optional boneh-lynn-shacham signature aggregation feature: `--features bls`

## Entry Points

### library

see crates/racer/src/lib.rs

### cli
binary for running a node, managing keys, or generating configuration.
  - `racer run`
  - `racer keygen`
  - `racer config`

**Build/Run**:
```bash
cargo run -p racer --features cli -- run --help
```

### example
demonstration of a sensor mesh on localnetwork.

  ```bash
  cargo run -p racer-examples --bin sensor-network -- examples/config/sensor_network.toml
  ```

  ```bash
  cargo run -p racer-examples --bin sensor-network --features bls -- examples/config/sensor_network.toml
  ```


### note
could be further optimized by batch processing READY and ECHO transmissions
