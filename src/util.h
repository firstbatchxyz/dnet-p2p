#ifndef DLLMD_UTIL_H
#define DLLMD_UTIL_H

#define RECORDNAME_SIZE 32

#include <netdb.h>
#include <stdio.h>
#include <sys/time.h>

#include "mdns.h"

mdns_string_t ipv4_address_to_string(char* buffer, size_t capacity, const struct sockaddr_in* addr, size_t addrlen);
mdns_string_t ipv6_address_to_string(char* buffer, size_t capacity, const struct sockaddr_in6* addr, size_t addrlen);
mdns_string_t ip_address_to_string(char* buffer, size_t capacity, const struct sockaddr* addr, size_t addrlen);
int rtype_to_string(char buffer[RECORDNAME_SIZE], uint16_t rtype);

/// Get current time in seconds since epoch.
static inline time_t get_current_time(void) { return time(NULL); }

#endif  // DLLMD_UTIL_H