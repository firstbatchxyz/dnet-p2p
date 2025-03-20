#ifndef DLLMD_SOCKETS_H
#define DLLMD_SOCKETS_H

#include <stdio.h>
#include <stdbool.h>
#include <arpa/inet.h>
// see `man getifaddrs`:
// > If both <net/if.h> and <ifaddrs.h> are being included,
// > <net/if.h> must be included before <ifaddrs.h>.
#include <net/if.h>
#include <ifaddrs.h>

#include "util.h"

int open_client_sockets(int* sockets, int max_sockets, int port, struct sockaddr_in* service_address_ipv4,
                        struct sockaddr_in6* service_address_ipv6);
int open_service_sockets(int* sockets, int max_sockets, struct sockaddr_in* service_address_ipv4,
                         struct sockaddr_in6* service_address_ipv6);

#endif  // DLLMD_SOCKETS_H