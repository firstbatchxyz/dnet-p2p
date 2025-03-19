#include "sockets.h"

#include <arpa/inet.h>
#include <ifaddrs.h>
#include <net/if.h>

#include "util.h"

/// Open sockets for sending one-shot multicast queries from an ephemeral port
static int open_client_sockets(int* sockets, int max_sockets, int port) {
  // When sending, each socket can only send to one network interface
  // Thus we need to open one socket for each interface and address family
  int num_sockets = 0;

  // inferface addresses
  struct ifaddrs* ifaddr = 0;
  struct ifaddrs* ifa = 0;
  if (getifaddrs(&ifaddr) < 0) {
    printf("Unable to get interface addresses\n");
  }

  // flags to indicate if we have seen the ipv4 and ipv6 address for the first time
  int first_ipv4 = 1;
  int first_ipv6 = 1;
  for (ifa = ifaddr; ifa; ifa = ifa->ifa_next) {
    // ensure we have an address and it is up and multicast capable
    if (!ifa->ifa_addr) {
      continue;
    }
    if (!(ifa->ifa_flags & IFF_UP) || !(ifa->ifa_flags & IFF_MULTICAST)) {
      continue;
    }
    if ((ifa->ifa_flags & IFF_LOOPBACK) || (ifa->ifa_flags & IFF_POINTOPOINT)) {
      continue;
    }

    // check if its ipv4 or ipv6
    if (ifa->ifa_addr->sa_family == AF_INET) {
      struct sockaddr_in* saddr = (struct sockaddr_in*)ifa->ifa_addr;
      if (saddr->sin_addr.s_addr != htonl(INADDR_LOOPBACK)) {
        int log_addr = 0;
        if (first_ipv4) {
          service_address_ipv4 = *saddr;
          first_ipv4 = 0;
          log_addr = 1;
        }
        has_ipv4 = 1;
        if (num_sockets < max_sockets) {
          saddr->sin_port = htons(port);
          int sock = mdns_socket_open_ipv4(saddr);
          if (sock >= 0) {
            sockets[num_sockets++] = sock;
            log_addr = 1;
          } else {
            log_addr = 0;
          }
        }
        if (log_addr) {
          char buffer[128];
          mdns_string_t addr = ipv4_address_to_string(buffer, sizeof(buffer), saddr, sizeof(struct sockaddr_in));
          printf("Local IPv4 address: %.*s\n", MDNS_STRING_FORMAT(addr));
        }
      }
    } else if (ifa->ifa_addr->sa_family == AF_INET6) {
      struct sockaddr_in6* saddr = (struct sockaddr_in6*)ifa->ifa_addr;
      // Ignore link-local addresses
      if (saddr->sin6_scope_id) {
        continue;
      }
      static const unsigned char localhost[] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1};
      static const unsigned char localhost_mapped[] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0x7f, 0, 0, 1};
      if (memcmp(saddr->sin6_addr.s6_addr, localhost, 16) && memcmp(saddr->sin6_addr.s6_addr, localhost_mapped, 16)) {
        int log_addr = 0;
        if (first_ipv6) {
          service_address_ipv6 = *saddr;
          first_ipv6 = 0;
          log_addr = 1;
        }
        has_ipv6 = 1;
        if (num_sockets < max_sockets) {
          saddr->sin6_port = htons(port);
          int sock = mdns_socket_open_ipv6(saddr);
          if (sock >= 0) {
            sockets[num_sockets++] = sock;
            log_addr = 1;
          } else {
            log_addr = 0;
          }
        }
        if (log_addr) {
          char buffer[128];
          mdns_string_t addr = ipv6_address_to_string(buffer, sizeof(buffer), saddr, sizeof(struct sockaddr_in6));
          printf("Local IPv6 address: %.*s\n", MDNS_STRING_FORMAT(addr));
        }
      }
    }
  }

  freeifaddrs(ifaddr);

  return num_sockets;
}

/** Open sockets to listen to incoming mDNS queries on port 5353. 
 * 
 * @param sockets The array to store the opened sockets.
 * @param max_sockets The maximum number of sockets to open.
 * @return The number of opened sockets.
 */
static int open_service_sockets(int* sockets, int max_sockets) {
  // When recieving, each socket can recieve data from all network interfaces
  // Thus we only need to open one socket for each address family
  int num_sockets = 0;

  // Call the client socket function to enumerate and get local addresses,
  // but not open the actual sockets
  open_client_sockets(0, 0, 0);

  if (num_sockets < max_sockets) {
    struct sockaddr_in sock_addr;
    memset(&sock_addr, 0, sizeof(struct sockaddr_in));
    sock_addr.sin_family = AF_INET;
    sock_addr.sin_addr.s_addr = INADDR_ANY;
    sock_addr.sin_port = htons(MDNS_PORT);
#ifdef __APPLE__
    sock_addr.sin_len = sizeof(struct sockaddr_in);
#endif
    int sock = mdns_socket_open_ipv4(&sock_addr);
    if (sock >= 0) {
      sockets[num_sockets++] = sock;
    }
  }

  if (num_sockets < max_sockets) {
    struct sockaddr_in6 sock_addr;
    memset(&sock_addr, 0, sizeof(struct sockaddr_in6));
    sock_addr.sin6_family = AF_INET6;
    sock_addr.sin6_addr = in6addr_any;
    sock_addr.sin6_port = htons(MDNS_PORT);
#ifdef __APPLE__
    sock_addr.sin6_len = sizeof(struct sockaddr_in6);
#endif
    int sock = mdns_socket_open_ipv6(&sock_addr);
    if (sock >= 0) {
      sockets[num_sockets++] = sock;
    }
  }

  return num_sockets;
}
