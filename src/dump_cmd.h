#ifndef DLLMD_DUMP_CMD_H
#define DLLMD_DUMP_CMD_H

#include <arpa/inet.h>
#include <errno.h>
#include <netdb.h>
#include <netinet/in.h>

#include "sockets.h"

// Callback handling questions and answers dump
static int dump_callback(int sock, const struct sockaddr* from, size_t addrlen, mdns_entry_type_t entry,
                         uint16_t query_id, uint16_t rtype, uint16_t rclass, uint32_t ttl, const void* data,
                         size_t size, size_t name_offset, size_t name_length, size_t record_offset,
                         size_t record_length, void* user_data) {
  char addrbuffer[64];
  char namebuffer[256];

  mdns_string_t fromaddrstr = ip_address_to_string(addrbuffer, sizeof(addrbuffer), from, addrlen);

  size_t offset = name_offset;
  mdns_string_t name = mdns_string_extract(data, size, &offset, namebuffer, sizeof(namebuffer));

  char record_name[RECORDNAME_SIZE] = {0};
  rtype_to_string(record_name, rtype);  // ignore return code

  const char* entry_type = "Question";
  if (entry == MDNS_ENTRYTYPE_ANSWER) {
    entry_type = "Answer";
  } else if (entry == MDNS_ENTRYTYPE_AUTHORITY) {
    entry_type = "Authority";
  } else if (entry == MDNS_ENTRYTYPE_ADDITIONAL) {
    entry_type = "Additional";
  }

  printf("%.*s: %s %s %.*s rclass 0x%x ttl %u\n", MDNS_STRING_FORMAT(fromaddrstr), entry_type, record_name,
         MDNS_STRING_FORMAT(name), (unsigned int)rclass, ttl);

  return 0;
}

// Dump all incoming mDNS queries and answers
static int dump_mdns(bool* is_running) {
  struct sockaddr_in service_address_ipv4 = {0};
  struct sockaddr_in6 service_address_ipv6 = {0};

  int sockets[32];
  int num_sockets =
      open_service_sockets(sockets, sizeof(sockets) / sizeof(sockets[0]), &service_address_ipv4, &service_address_ipv6);
  if (num_sockets <= 0) {
    printf("Failed to open any client sockets\n");
    return -1;
  }
  printf("Opened %d socket%s for mDNS dump\n", num_sockets, num_sockets > 1 ? "s" : "");

  size_t capacity = 2048;
  void* buffer = malloc(capacity);

  // This is a crude implementation that checks for incoming queries and answers
  while (*is_running) {
    int nfds = 0;
    fd_set readfs;
    FD_ZERO(&readfs);
    for (int i = 0; i < num_sockets; ++i) {
      if (sockets[i] >= nfds) {
        nfds = sockets[i] + 1;
      }
      FD_SET(sockets[i], &readfs);
    }

    struct timeval timeout;
    timeout.tv_sec = 0;
    timeout.tv_usec = 100000;

    if (select(nfds, &readfs, 0, 0, &timeout) >= 0) {
      for (int i = 0; i < num_sockets; ++i) {
        if (FD_ISSET(sockets[i], &readfs)) {
          mdns_socket_listen(sockets[i], buffer, capacity, dump_callback, 0);
        }
        FD_SET(sockets[i], &readfs);
      }
    } else {
      break;
    }
  }

  free(buffer);

  // close sockets
  for (int i = 0; i < num_sockets; ++i) {
    mdns_socket_close(sockets[i]);
  }
  printf("Closed socket%s\n", num_sockets > 1 ? "s" : "");

  return 0;
}

#endif  // DLLMD_DUMP_CMD_H