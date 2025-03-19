# Networking in C

This is a playground repo for networking implementations of [dLLM](https://github.com/firstbatchxyz/dLLM/). At the very least, it aims to provide an interface that can allow peers to discover each other in a local network, e.g. via `mDNS` or such.

> [!NOTE]
>
> The code here is based on [Beej's Guide to Network Programming](https://beej.us/guide/bgnet/).

## Usage

Compile with `make` to build files under `build` directory. The binary will be at `./build/main`.

### Address Info

To demonstrate `getaddrinfo` (from `netdb.h`) do:

```sh
./build/main showip <hostname>
```

For example `./build/main showip dria.co` returns:

```text
IP addresses for dria.co:

  IPv4: 76.76.21.21
```

### Client & Server

First, launch a server (which listens to loopback on a specific port):

```sh
./build/main server
```

Then, run a client with target as loopback address:

```sh
./build/main client localhost

# these are received as connections from ::1
./build/main client localhost
./build/main client ::1
./build/main client ::

# these are received as connections from ::ffff:127.0.0.1
./build/main client 127.0.0.1
./build/main client 0.0.0.0
```

You will see that client receives "Hello, world!" from the server.

### Datagram Sockets

First, launch a server that listens for UDP packets at a specific port:

```sh
./build/main listener
```

Then, you can send a packet there:

```sh
./build/main talker ::1 "hi there whats up?"
```

Note that since `talker` is using UDP, you can technically send to any host and the program wont fail.

### mDNS

Source code from public domain [mDNS](https://github.com/mjansson/mdns) header-only implementation, along with its example code.

```sh
# start mDNS service
./build/main mdns --service

# query an mDNS host
./build/main mdns --query <hostname>
./build/main mdns --query _dllmd._tcp.local.

# discovery services
./build/main mdns --discovery

# dump all mDNS queries and answers to stdout
./build/main mdns --dump

# start the daemon
./build/main mdns --daemon
```

For all available commands:

- Hostname can be overwritten via `--hostname <name>` option, otherwise it is attempted to be read via a systemcall to get machine's host name.
- Port can be overwritten via `--port <port>` option, otherwise it defaults to 41891 which stands for `dria` in alphabetic index.

When the daemon is running, we can detect it with the [`dns-sd`](https://man.netbsd.org/dns-sd.1) standard tool:

```sh
dns-sd -B _dllmd local
```

We can query the services registered for dLLM with:

```sh
dns-sd -Q _dllmd._tcp.local. PTR
```

This will return a hostname within `rrdata` field of the `PTR` record. We can query that host to get more details, and we will actually see this query within our logs, if we are the `my-hostname`:

```sh
dns-sd -q my-hostname.local._dllmd._tcp.local. SRV
```
