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
