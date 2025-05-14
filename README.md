# dnet p2p

dnet p2p is a shared library that add mDNS peer-to-peer connectibility. It exposes two modes (inspired from MPI terms):

- **Manager**: the leader of the local network, and there can only be one leader.
- **Worker**: every non-manager is expected to be a worker, connected via the LAN to serve their compute power to the manager.

Each worker runs on a random OS-assigned port, and published their information via mDNS. The manager will then browse mDNS services actively and keep a record of the workers in the network. There are several threads:

- **Service**: Binds to a TCP socket at a random port.
- **Worker mDNS Daemon**: Workers run a daemon to connect with mDNS, they periodically publish their info to their mDNS service properties.
- **Manager mDNS Daemon**: Worker that wants to issue something becomes a "manager", also denoted by its TXT property.

TODO: check [tokio-serial](https://github.com/berkowski/tokio-serial) for Serial port communication

## Installation

Install from the source via:

```sh

```

## Usage

### Worker

The default mode is to run as a worker.

```sh
cargo run
```

### Manager

To run as a manager, we need to give a task. You can imagine that the C/C++ calls the shared library in a similar fashion.

```sh
cargo run <message>
```

You can select the type of message with the last argument:

- if omitted, will simply `ping` the workers
- `matmul` will send a distributed matrix multiplication task, respecting device specs while giving the portion of matrix to them.
- TODO: ...

### FFI from C/C++

Include the shared library within your loader step, e.g. `-L some/directory -ldnet`. Then, include [`dnet.h`](./example/src/dnet.h) in your code.

- You can create a new service object with `dnet_new` which returns you an object pointer, and free it later with `dnet_free`.
- You can start the service with `dnet_start` which returns you a thread handle pointer, and then stop it with `dnet_stop`.
- You can publish a data to all peers with `dnet_publish`, or receive a data for your peer with `dnet_receive`.

See the [header file](./example/src/dnet.h) for more specific instructions.

### Discovering with [dns-sd](https://man.netbsd.org/dns-sd.1)

When the daemon is running, we can detect the libp2p MDNS service with the [dns-sd](https://manp.gs/mac/1/dns-sd) standard tool (following the definitions in [libp2p-mdns specification](https://github.com/libp2p/specs/blob/master/discovery/mdns.md)).

> [!TIP]
>
> You can use other DNS-based service discovery tools like [Avahi](https://avahi.org/) or [Bonjour](https://developer.apple.com/bonjour/) for this as well.

First, we can make a DNS-SD meta-query to see that indeed `_dnet_._tcp` is registered (can be piped to `grep dnet`):

```sh
$ dns-sd -Q _services._dns-sd._udp.local PTR
# ...
A/R  Flags         IF  Name                          Type   Class  Rdata
Add  2             12  _services._dns-sd._udp.local. PTR    IN     _dnet_._tcp.local.
# ...
```

Then, we can query PTRs of the service at the dnet mDNS domain with:

```sh
$ dns-sd -Q _dnet._tcp.local. PTR
A/R  Flags         IF  Name                      Type   Class  Rdata
Add  40000003      11  _dnet._tcp.local          PTR    IN     <your-service>
# ...
```

The `Rdata` returned by a PTR record points to another service, accessed by the returned domain (which are subdomains of `_dnet`). We can query any of them to get their details, such as service (SRV) details or additional records within TXT records.

```sh
# service records
$ dns-sd -Q <your-service> SRV
A/R  Flags         IF  Name                          Type   Class  Rdata
Add  40000003       1  <your-instance>._dnet._tcp.local.     SRV    IN     0 0 3456 <your-hostname>.local.

# additional records
$ dns-sd -Q <your-service> TXT
A/R  Flags         IF  Name                          Type   Class  Rdata
Add  40000003      11  <your-instance>._dnet._tcp.local.     TXT    IN     9 bytes: 08 50 41 54 48 3D 6F 6E 65
```

The returned bytes are to be decoded from hex, and can be treat as a string of the form `key=value`. The keys are case-insensitive.

> [!NOTE]
>
> When a `dnet` service is no longer online, its SRV records are gone, but TXT records may still be there.
