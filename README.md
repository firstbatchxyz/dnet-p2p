# dnet p2p

dnet p2p is a shared library that add mDNS peer-to-peer connectibility. It exposes two modes (inspired from MPI terms):

- **Manager**: the manager of the local network, is able to dispatch tasks to all other. There can only be one manager.
- **Worker**: every non-manager is expected to be a worker, connected via the LAN to serve their compute power to the manager.

## Installation

Install from the source via:

```sh
cargo install --git https://github.com/firstbatchxyz/dnet-p2p
```

## Usage

### Cargo from Source

You can run DnetP2P as follows:

```sh
# worker
cargo run

# manager
cargo run -m

# passive, just monitors others
cargo run -p
```

Note that there can only be one manager at a time.

### FFI from C/C++

Include the shared library within your loader step, e.g. `-L some/directory -ldnet_p2p`. Then, include [`dnet_p2p.h`](./bindings/c/src/dnet_p2p.h) in your code.

- You can create a new service object with `dnet_p2p_new` which returns you an object pointer, and free it later with `dnet_p2p_free`.
- You can start the service with `dnet_p2p_start` which returns you a thread handle pointer, and then stop it with `dnet_p2p_stop`.

See the [header file](./bindings/c/src/dnet_p2p.h) for more specific instructions.

### FFI from Python

A utility class is provided within [`dnet_p2p.py`](./bindings/py/src/dnet_p2p.py) that wraps the function calls for the shared library using `ctypes`. It provides both context usage (i.e. `with`) and normal usage.

See the example usage at [`main.py`](./bindings/py/main.py) that you can run with:

```sh
# worker
RUST_LOG=info uv run main.py

# manager
RUST_LOG=info uv run main.py -m
```

## Testing

Run tests with:

```sh
cargo test
```

### License

See [license file](./LICENSE).
