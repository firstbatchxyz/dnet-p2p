# dnet p2p

dnet p2p is a shared library that add mDNS peer-to-peer connectibility. It exposes two modes (inspired from MPI terms):

- **Manager**: the manager of the local network, is able to dispatch tasks to all other. There can only be one manager.
- **Worker**: every non-manager is expected to be a worker, connected via the LAN to serve their compute power to the manager.

Each worker runs on a random OS-assigned port, and published their information via mDNS. The manager will then browse mDNS services actively and keep a record of the workers in the network. There are several threads:

- **Service**: Binds to a TCP socket at a random port.
- **Worker mDNS Daemon**: Workers run a daemon to connect with mDNS, they periodically publish their info to their mDNS service properties.
- **Manager mDNS Daemon**: Worker that wants to issue something becomes a "manager", also denoted by its TXT property.

## Installation

Install from the source via:

```sh
cargo install --git https://github.com/firstbatchxyz/dnet-p2p
```

## Usage

Here we describe both running from Rust and from C/C++.

> [!WARNING]
>
> If you happen to register with the same instance & hostname, on MacOS your hostname may be automatically updated, usually it goes from `myhost` to `myhost-2`, `myhost-3` etc.
>
> You may have to change that back from <kbd>Settings > Sharing > Local hostname</kbd>.

### Worker

The default mode is to run as a worker with a given instance name.

```sh
cargo run -i <instance>
```

You can register as a manager as well:

```sh
cargo run -i <instance> -m
```

Note that there can only be one manager at a time.

### FFI from C/C++

Include the shared library within your loader step, e.g. `-L some/directory -ldnet_p2p`. Then, include [`dnet_p2p.h`](./example/c/src/dnet_p2p.h) in your code.

- You can create a new service object with `dnet_p2p_new` which returns you an object pointer, and free it later with `dnet_p2p_free`.
- You can start the service with `dnet_p2p_start` which returns you a thread handle pointer, and then stop it with `dnet_p2p_stop`.

See the [header file](./example/c/src/dnet_p2p.h) for more specific instructions.

### FFI from Python

A utility class is provided within [`dnet_p2p.py`](./example/py/src/dnet_p2p.py) that wraps the function calls for the shared library using `ctypes`. It provides both context usage (i.e. `with`) and normal usage.

### Discovering with [dns-sd](https://man.netbsd.org/dns-sd.1)

When the daemon is running, we can detect the mDNS service with the [dns-sd](https://manp.gs/mac/1/dns-sd) standard tool.

> [!TIP]
>
> You can use other DNS-based service discovery tools like [Avahi](https://avahi.org/) or [Bonjour](https://developer.apple.com/bonjour/) for this as well.

First, we can make a DNS-SD meta-query to see that indeed `_dnet_p2p._tcp` is registered (can be piped to `grep dnet`):

```sh
$ dns-sd -Q _services._dns-sd._udp.local PTR
# ...
A/R  Flags         IF  Name                          Type   Class  Rdata
Add  2             12  _services._dns-sd._udp.local. PTR    IN     _dnet_p2p._tcp.local.
# ...
```

Then, we can query PTRs of the service at the dnet mDNS domain with:

```sh
$ dns-sd -Q _dnet_p2p._tcp.local. PTR
A/R  Flags         IF  Name                      Type   Class  Rdata
Add  40000003      11  _dnet_p2p._tcp.local          PTR    IN     <your-service>
# ...
```

The `Rdata` returned by a PTR record points to another service, accessed by the returned domain (which are subdomains of `_dnet_p2p`). We can query any of them to get their details, such as service (SRV) details or additional records within TXT records.

```sh
# service records
$ dns-sd -Q <your-service> SRV
A/R  Flags         IF  Name                          Type   Class  Rdata
Add  40000003       1  <your-instance>._dnet_p2p._tcp.local.     SRV    IN     0 0 3456 <your-hostname>.local.

# additional records
$ dns-sd -Q <your-service> TXT
A/R  Flags         IF  Name                          Type   Class  Rdata
Add  40000003      11  <your-instance>._dnet_p2p._tcp.local.     TXT    IN     9 bytes: 08 50 41 54 48 3D 6F 6E 65
```

The returned bytes are to be decoded from hex, and can be treat as a string of the form `key=value`. The keys are case-insensitive.

> [!NOTE]
>
> When a `dnet` service is no longer online, its SRV records are gone, but TXT records may still be there.
