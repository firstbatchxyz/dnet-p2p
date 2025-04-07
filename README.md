# dLLM Daemon

dLLM Daemon (dllmd) is a daemon service that connects peers within a local network together, allowing them to create a topology and share information about their latency costs, device properties and such; all so that a host can do sharding to share a model across devices.

## Usage as library from C/C++

Include the shared library within your loader step, e.g. `-L some/directory -ldllmd`. Then, include [`dllmd.h`](./example/src/dllmd.h) in your code.

- You can create a new service object with `dllmd_new` which returns you an object pointer, and free it later with `dllmd_free`.

- You can start the service with `dllmd_start` which returns you a thread handle pointer, and then stop it with `dllmd_stop`.

See the header file for more specific instructions.

> [!TIP]
> Debug builds of the library include diagnostic prints to `stderr`, otherwise nothing is printed.

## Usage as CLI

Simple run the daemon with:

```sh
cargo run dllmd
```

TODO: !!!

## Usage with [dns-sd](https://man.netbsd.org/dns-sd.1)

When the daemon is running, we can detect the libp2p MDNS service with the [dns-sd](https://manp.gs/mac/1/dns-sd) standard tool (following the definitions in [libp2p-mdns specification](https://github.com/libp2p/specs/blob/master/discovery/mdns.md)).

First, we can make a DNS-SD meta-query to see that indeed `_p2p._udp` is registered (can be piped to `grep p2p`):

```sh
$ dns-sd -Q _services._dns-sd._udp.local PTR
# ...
A/R  Flags         IF  Name                          Type   Class  Rdata
Add  2             12  _services._dns-sd._udp.local. PTR    IN     _p2p._udp.local.
# ...
```

Then, we can query PTRs of the service at the dLLM mDNS domain with:

```sh
$ dns-sd -Q _p2p._udp.local. PTR
A/R  Flags         IF  Name                      Type   Class  Rdata
Add  40000003      11  _p2p._udp.local           PTR    IN     <some-text>.
# ...
```

The `Rdata` returned by a PTR record points to another service, accessed by the returned domain (which are subdomains of `_dllmd`). We can query any of them to get their details, such as service (SRV) details or additional records within TXT records.

```sh
# service records
$ dns-sd -Q gZSkS6ITRpeQHyO0b99O0qV8imlYkJgdZCf. SRV
A/R  Flags         IF  Name                          Type   Class  Rdata
Add  40000003       1  foobar._dllmd._tcp.local.     SRV    IN     0 0 3456 erhant-work.local.

# additional records
$ dns-sd -Q foobar._dllmd._tcp.local. TXT
A/R  Flags         IF  Name                          Type   Class  Rdata
Add  40000003      11  foobar._dllmd._tcp.local.     TXT    IN     9 bytes: 08 50 41 54 48 3D 6F 6E 65
```

The returned bytes are to be decoded from hex, and can be treat as a string of the form `key=value`. The keys are case-insensitive.

> [!NOTE]
>
> When a dLLM service is no longer online, its SRV records are gone, but TXT records may still be there.
