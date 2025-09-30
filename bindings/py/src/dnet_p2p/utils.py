import socket
import ifaddr


def get_adapters():
    # en0 can be used for over-the-WiFi connections on macOS
    # bridge0 can be used for Thunderbolt
    adapters = ifaddr.get_adapters()

    for adapter in adapters:
        print(f"IPs of network adapter: {adapter.nice_name} ({adapter.name})")
        for ip in adapter.ips:
            print(f"IP: {ip.ip}")


def get_public_ip():
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    s.connect(("8.8.8.8", 80))
    local_ip = s.getsockname()[0]
    s.close()

    return local_ip
