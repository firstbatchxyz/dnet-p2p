# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Build and Development
```bash
# Build the library (debug mode)
cargo build

# Build the library (release mode)
cargo build --release

# Run as worker with instance name
cargo run -- -i <instance_name>

# Run as manager with instance name
cargo run -- -i <instance_name> -m

# Run tests
cargo test

# Check code formatting
cargo fmt --check

# Apply code formatting
cargo fmt

# Run clippy lints
cargo clippy
```

### C/C++ Example
```bash
# Build C example (from example/c/)
make

# Clean C build
make clean

# Rebuild C example
make again

# Run C example
make run
```

### Python Example
```bash
# Run Python worker (from example/py/)
uv run main.py

# Run Python manager (from example/py/)
uv run main.py -m
```

## Architecture

This is a Rust library (`dnet-p2p`) that provides mDNS-based peer-to-peer connectivity for distributed computing. The system uses a manager-worker model inspired by MPI.

### Core Components

**DnetService** (`src/service/core.rs`): The main service class that handles:
- TCP socket binding on random OS-assigned ports
- mDNS service registration and discovery
- Peer management and property tracking
- System resource monitoring (CPU, GPU, memory)

**Service Properties** (`src/service/properties.rs`): Serializable properties published via mDNS TXT records:
- CPU information (brand, cores)
- GPU information (name, type: Integrated/Discrete/Virtual/CPU)
- Memory metrics (available, free, total)
- Service metadata (hostname, instance, address, manager status)

**mDNS Integration** (`src/service/mdns.rs`): Handles service discovery using the `_dnet_p2p._tcp` service type:
- Service registration with periodic property updates every 5 seconds
- Active browsing for other dnet services
- Conflict resolution (only one manager allowed)

**FFI Layer** (`src/ffi/mod.rs`): C-compatible interface for cross-language integration:
- `dnet_p2p_new()` - Create service instance
- `dnet_p2p_start()` - Start service in background thread
- `dnet_p2p_stop()` - Graceful shutdown
- `dnet_p2p_get_properties()` - Retrieve peer properties as JSON

### Key Design Patterns

- **Async Runtime**: Uses Tokio for async operations with graceful shutdown via CancellationToken
- **Cross-platform Signal Handling**: Supports Unix (SIGTERM/SIGINT) and Windows (CTRL_*) signals
- **Resource Monitoring**: Integrates `sysinfo` for system metrics and `wgpu` for GPU detection
- **Serialization**: Uses serde with custom TXT record serialization via `serde-txtrecord`
- **FFI Safety**: Careful memory management with Box allocation/deallocation for C interop

### Service Discovery Protocol

Services register with mDNS domain `_dnet_p2p._tcp.local` and publish properties via TXT records. The manager actively discovers workers and maintains a peer registry. Only one manager is allowed per network - conflicts are detected and rejected.

### Multi-language Support

The library compiles as both `cdylib` (for C/C++ FFI) and `rlib` (for Rust consumption). Examples demonstrate integration patterns for C and Python via ctypes.