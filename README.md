# dLLM Daemon

dLLM Daemon (dllmd) is a daemon service that connects peers within a local network together, allowing them to create a topology and share information about their latency costs, device properties and such; all so that a host can do sharding to share a model across devices.

## Usage as library from C/C++

TODO: !!!

## Usage as CLI

```sh
cargo run dllmd
```

TODO: !!!

## Usage with dns-sd

When the daemon is running, we can detect it with the [dns-sd](https://manp.gs/mac/1/dns-sd) standard tool:

```sh
$ dns-sd -B _dllmd
A/R    Flags  if Domain               Service Type         Instance Name
Add        3  11 local.               _dllmd._tcp.         foobar
# ...
```

We can query PTRs of the service at the dLLM mDNS domain with:

```sh
$ dns-sd -Q _dllmd._tcp.local. PTR
A/R  Flags         IF  Name                          Type   Class  Rdata
Add  40000003      11  _dllmd._tcp.local.            PTR    IN     foobar._dllmd._tcp.local.
# ...
```

The `Rdata` returned by a PTR record points to another service, accessed by the returned domain (which are subdomains of `_dllmd`). We can query any of them to get their details, such as service (SRV) details or additional records within TXT records.

```sh
# service records
$ dns-sd -Q foobar._dllmd._tcp.local. SRV
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
