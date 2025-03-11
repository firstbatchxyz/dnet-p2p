# DLLM

An application that creates a peer-to-peer network on LAN where nodes:

- Discover other nodes automatically via multicast
- Advertise themselves when joining the network
- Share system resource information with all peers
- Monitor network changes (joins/disconnects)

## Overview

### Components

- Network Discovery: UDP multicast for node detection
- Resource Monitoring: System stats collection
- Message Protocol: Data exchange format between nodes
- Connection Management: Peer list maintenance

### Implementation Notes

- Use UDP multicast for node discovery (224.0.0.0/4)
- Implement heartbeat mechanism for node health checks
- Create efficient data structures for peer information
- Handle graceful disconnection notifications

### System Resources to Monitor

- CPU usage
- Memory utilization
- Network bandwidth
- Disk space
- System uptime

## DLLM Daemon

```sh
dllmd
```

> [!NOTE]
>
> `dllmd` stands for dLLM daemon.
