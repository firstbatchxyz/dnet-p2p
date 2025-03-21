#include "util.h"

mdns_string_t ipv4_address_to_string(char* buffer, size_t capacity, const struct sockaddr_in* addr, size_t addrlen) {
  char host[NI_MAXHOST] = {0};
  char service[NI_MAXSERV] = {0};
  int ret = getnameinfo((const struct sockaddr*)addr, (socklen_t)addrlen, host, NI_MAXHOST, service, NI_MAXSERV,
                        NI_NUMERICSERV | NI_NUMERICHOST);

  int len = 0;
  if (ret == 0) {
    if (addr->sin_port != 0) {
      len = snprintf(buffer, capacity, "%s:%s", host, service);
    } else {
      len = snprintf(buffer, capacity, "%s", host);
    }
  }
  if (len >= (int)capacity) {
    len = (int)capacity - 1;
  }

  return (mdns_string_t){.str = buffer, .length = len};
}

mdns_string_t ipv6_address_to_string(char* buffer, size_t capacity, const struct sockaddr_in6* addr, size_t addrlen) {
  char host[NI_MAXHOST] = {0};
  char service[NI_MAXSERV] = {0};
  int ret = getnameinfo((const struct sockaddr*)addr, (socklen_t)addrlen, host, NI_MAXHOST, service, NI_MAXSERV,
                        NI_NUMERICSERV | NI_NUMERICHOST);
  int len = 0;
  if (ret == 0) {
    if (addr->sin6_port != 0) {
      len = snprintf(buffer, capacity, "[%s]:%s", host, service);
    } else {
      len = snprintf(buffer, capacity, "%s", host);
    }
  }
  if (len >= (int)capacity) {
    len = (int)capacity - 1;
  }

  return (mdns_string_t){.str = buffer, .length = len};
}

/// Convert IP address to string representation.
mdns_string_t ip_address_to_string(char* buffer, size_t capacity, const struct sockaddr* addr, size_t addrlen) {
  if (addr->sa_family == AF_INET6) {
    return ipv6_address_to_string(buffer, capacity, (const struct sockaddr_in6*)addr, addrlen);
  } else {
    return ipv4_address_to_string(buffer, capacity, (const struct sockaddr_in*)addr, addrlen);
  }
}

/** 
 * Parses the resource type and writes the record type name to given buffer.
 * 
 * @param buffer The buffer to write the string to, of size `RECORDNAME_SIZE`
 * @param rtype The record type to convert
 * @return 0 on success, -1 if record type is invalid
 */
int rtype_to_string(char buffer[RECORDNAME_SIZE], uint16_t rtype) {
  const char* name = NULL;
  if (rtype == MDNS_RECORDTYPE_PTR) {
    name = "PTR";
  } else if (rtype == MDNS_RECORDTYPE_SRV) {
    name = "SRV";
  } else if (rtype == MDNS_RECORDTYPE_A) {
    name = "A";
  } else if (rtype == MDNS_RECORDTYPE_AAAA) {
    name = "AAAA";
  } else if (rtype == MDNS_RECORDTYPE_TXT) {
    name = "TXT";
  } else if (rtype == MDNS_RECORDTYPE_ANY) {
    name = "ANY";
  } else {
    return -1;
  }

  snprintf(buffer, RECORDNAME_SIZE, "%s", name);
  return 0;
}
