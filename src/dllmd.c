/*
** dllmd.c -- Distributed Leader Election Daemon using mDNS
*/

#include <arpa/inet.h>
#include <errno.h>
#include <netdb.h>
#include <netinet/in.h>
#include <signal.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>

#include "mdns.h"
#include "util.h"

#define DLLMD_PORT "41891"  // 41891 = 4 18 9 1 = D R I A
#define DLLMD_SERVICE_NAME "_dllmd._udp.local."
#define PING_INTERVAL 5  // Check for service every 5 seconds
#define PING_TIMEOUT \
  15                  // Consider service dead after 15 seconds of no response
#define MAX_NODES 32  // Maximum number of nodes to track
#define RANDOM_DELAY_MAX \
  5  // Maximum random delay for service election in seconds
#define BUFFER_SIZE 2048

typedef enum {
  NODE_TYPE_CLIENT,  // Regular node
  NODE_TYPE_SERVICE  // Service node (leader)
} node_type_t;

typedef struct {
  char hostname[256];
  struct sockaddr_storage addr;
  socklen_t addrlen;
  time_t last_seen;
  int active;
} node_info_t;

// Global variables
static volatile int running = 1;
static node_type_t node_type = NODE_TYPE_CLIENT;
static node_info_t service_node;
static node_info_t known_nodes[MAX_NODES];
static int node_count = 0;
static time_t last_ping_time = 0;
static char hostname_buffer[256];
static int service_socket = -1;
static int client_sockets[32];
static int num_client_sockets = 0;
static char* message_buffer;

// Forward declarations
static void become_service(void);
static void query_for_service(void);
static void check_service_health(void);
static void send_ping(void);
static void process_incoming_messages(void);

// Signal handler
static void signal_handler(int sig) {
  (void)sig;
  running = 0;
}

// Get current time in seconds
static time_t get_current_time(void) { return time(NULL); }

// Check if two sockaddr structures refer to the same address
static bool sockaddr_equal(const struct sockaddr* a, const struct sockaddr* b) {
  if (a->sa_family != b->sa_family) return false;

  if (a->sa_family == AF_INET) {
    // ipv4
    const struct sockaddr_in* a_in = (const struct sockaddr_in*)a;
    const struct sockaddr_in* b_in = (const struct sockaddr_in*)b;
    return a_in->sin_addr.s_addr == b_in->sin_addr.s_addr &&
           a_in->sin_port == b_in->sin_port;

  } else if (a->sa_family == AF_INET6) {
    // ipv6
    const struct sockaddr_in6* a_in6 = (const struct sockaddr_in6*)a;
    const struct sockaddr_in6* b_in6 = (const struct sockaddr_in6*)b;
    return memcmp(&a_in6->sin6_addr, &b_in6->sin6_addr,
                  sizeof(struct in6_addr)) == 0 &&
           a_in6->sin6_port == b_in6->sin6_port;
  }

  return false;
}

// Format address to string
static mdns_string_t ip_address_to_string(char* buffer, size_t capacity,
                                          const struct sockaddr* addr,
                                          size_t addrlen) {
  char host[NI_MAXHOST] = {0};
  char service[NI_MAXSERV] = {0};
  int ret = getnameinfo(addr, (socklen_t)addrlen, host, NI_MAXHOST, service,
                        NI_MAXSERV, NI_NUMERICSERV | NI_NUMERICHOST);
  int len = 0;
  if (ret == 0) {
    if (addr->sa_family == AF_INET)
      len = snprintf(buffer, capacity, "%s:%s", host, service);
    else if (addr->sa_family == AF_INET6)
      len = snprintf(buffer, capacity, "[%s]:%s", host, service);
  }
  if (len >= (int)capacity) len = (int)capacity - 1;
  mdns_string_t str;
  str.str = buffer;
  str.length = len;
  return str;
}

// Callback for discovery and ping responses
static int query_callback(int sock, const struct sockaddr* from, size_t addrlen,
                          mdns_entry_type_t entry, uint16_t query_id,
                          uint16_t rtype, uint16_t rclass, uint32_t ttl,
                          const void* data, size_t size, size_t name_offset,
                          size_t name_length, size_t record_offset,
                          size_t record_length, void* user_data) {
  if (entry != MDNS_ENTRYTYPE_ANSWER) return 0;

  char namebuf[256];
  char addrbuf[128];
  mdns_string_t name =
      mdns_string_extract(data, size, &name_offset, namebuf, sizeof(namebuf));

  // Check if this is a response for our service
  if (rtype == MDNS_RECORDTYPE_SRV &&
      strstr(name.str, DLLMD_SERVICE_NAME) != NULL) {
    mdns_record_srv_t srv = mdns_record_parse_srv(
        data, size, record_offset, record_length, namebuf, sizeof(namebuf));

    // Found a service node
    memcpy(&service_node.addr, from, addrlen);
    service_node.addrlen = addrlen;
    strncpy(service_node.hostname, srv.name.str,
            sizeof(service_node.hostname) - 1);
    service_node.hostname[sizeof(service_node.hostname) - 1] = '\0';
    service_node.last_seen = get_current_time();
    service_node.active = 1;

    mdns_string_t addrstr =
        ip_address_to_string(addrbuf, sizeof(addrbuf), from, addrlen);
    printf("Found service: %.*s at %.*s\n", MDNS_STRING_FORMAT(srv.name),
           MDNS_STRING_FORMAT(addrstr));

    // If we were trying to become a service but found an existing one, go back
    // to client
    if (node_type == NODE_TYPE_SERVICE) {
      printf("Another node is already the service, reverting to client mode\n");
      node_type = NODE_TYPE_CLIENT;
    }

    return 1;  // Stop processing, we found what we need
  }
  // Process pings/pongs
  else if (rtype == MDNS_RECORDTYPE_TXT) {
    size_t parsed =
        mdns_record_parse_txt(data, size, record_offset, record_length,
                              (mdns_record_txt_t*)namebuf, 16);

    for (size_t i = 0; i < parsed; ++i) {
      mdns_record_txt_t* record = (mdns_record_txt_t*)namebuf + i;

      // Check for ping messages
      if (record->value.length >= 4 &&
          strncmp(record->key.str, "ping", 4) == 0) {
        printf("Received ping from %.*s\n", MDNS_STRING_FORMAT(name));

        // If we're the service, update the node's last_seen time
        if (node_type == NODE_TYPE_SERVICE) {
          int found = 0;
          for (int i = 0; i < node_count; i++) {
            if (sockaddr_equal((struct sockaddr*)&known_nodes[i].addr, from)) {
              known_nodes[i].last_seen = get_current_time();
              found = 1;
              break;
            }
          }

          // Add new node if not already known
          if (!found && node_count < MAX_NODES) {
            memcpy(&known_nodes[node_count].addr, from, addrlen);
            known_nodes[node_count].addrlen = addrlen;
            known_nodes[node_count].last_seen = get_current_time();
            known_nodes[node_count].active = 1;
            node_count++;

            mdns_string_t addrstr =
                ip_address_to_string(addrbuf, sizeof(addrbuf), from, addrlen);
            printf("New node registered: %.*s\n", MDNS_STRING_FORMAT(addrstr));
          }

          // Send pong response
          mdns_query_answer_unicast(
              sock, from, addrlen, message_buffer, BUFFER_SIZE, query_id,
              MDNS_RECORDTYPE_TXT, DLLMD_SERVICE_NAME,
              strlen(DLLMD_SERVICE_NAME),
              (mdns_record_t){
                  .name = {DLLMD_SERVICE_NAME, strlen(DLLMD_SERVICE_NAME)},
                  .type = MDNS_RECORDTYPE_TXT,
                  .data.txt.key = {MDNS_STRING_CONST("pong")},
                  .data.txt.value = {MDNS_STRING_CONST("1")},
              },
              0, 0, 0, 0);
        }
      }
      // Check for pong responses
      else if (record->value.length >= 4 &&
               strncmp(record->key.str, "pong", 4) == 0) {
        mdns_string_t addrstr =
            ip_address_to_string(addrbuf, sizeof(addrbuf), from, addrlen);
        printf("Received pong from %.*s\n", MDNS_STRING_FORMAT(addrstr));

        if (node_type == NODE_TYPE_CLIENT &&
            sockaddr_equal((struct sockaddr*)&service_node.addr, from)) {
          service_node.last_seen = get_current_time();
        }
      }
    }
  }

  return 0;
}

// Callback for service queries
static int service_callback(int sock, const struct sockaddr* from,
                            size_t addrlen, mdns_entry_type_t entry,
                            uint16_t query_id, uint16_t rtype, uint16_t rclass,
                            uint32_t ttl, const void* data, size_t size,
                            size_t name_offset, size_t name_length,
                            size_t record_offset, size_t record_length,
                            void* user_data) {
  if (entry != MDNS_ENTRYTYPE_QUESTION) return 0;

  char namebuf[256];
  char addrbuf[128];
  size_t offset = name_offset;
  mdns_string_t name =
      mdns_string_extract(data, size, &offset, namebuf, sizeof(namebuf));

  // Only respond if we're the service
  if (node_type != NODE_TYPE_SERVICE) return 0;

  // Check if this is a query for our service
  if ((name.length == strlen(DLLMD_SERVICE_NAME)) &&
      (strncmp(name.str, DLLMD_SERVICE_NAME, name.length) == 0)) {
    if ((rtype == MDNS_RECORDTYPE_PTR) || (rtype == MDNS_RECORDTYPE_ANY)) {
      // Create service instance name: hostname._dllmd._udp.local.
      char service_instance[512];
      snprintf(service_instance, sizeof(service_instance), "%s.%s",
               hostname_buffer, DLLMD_SERVICE_NAME);

      // Answer PTR record
      mdns_record_t answer = {
          .name = {DLLMD_SERVICE_NAME, strlen(DLLMD_SERVICE_NAME)},
          .type = MDNS_RECORDTYPE_PTR,
          .data.ptr.name = {service_instance, strlen(service_instance)},
          .rclass = rclass,
          .ttl = 60};

      // Create hostname.local. for SRV record
      char hostname_local[256];
      snprintf(hostname_local, sizeof(hostname_local), "%s.local.",
               hostname_buffer);

      // Set up SRV record
      mdns_record_t srv_record = {
          .name = {service_instance, strlen(service_instance)},
          .type = MDNS_RECORDTYPE_SRV,
          .data.srv.name = {hostname_local, strlen(hostname_local)},
          .data.srv.port = atoi(DLLMD_PORT),
          .data.srv.priority = 0,
          .data.srv.weight = 0,
          .rclass = rclass,
          .ttl = 60};

      // Set up TXT records
      mdns_record_t txt_record = {
          .name = {service_instance, strlen(service_instance)},
          .type = MDNS_RECORDTYPE_TXT,
          .data.txt.key = {MDNS_STRING_CONST("info")},
          .data.txt.value = {MDNS_STRING_CONST("dllmd service")},
          .rclass = rclass,
          .ttl = 60};

      // Set up additional records
      mdns_record_t additional[2] = {srv_record, txt_record};

      uint16_t unicast = (rclass & MDNS_UNICAST_RESPONSE);
      mdns_string_t addrstr =
          ip_address_to_string(addrbuf, sizeof(addrbuf), from, addrlen);
      printf("Answering query from %.*s (%s)\n", MDNS_STRING_FORMAT(addrstr),
             (unicast ? "unicast" : "multicast"));

      if (unicast) {
        mdns_query_answer_unicast(sock, from, addrlen, message_buffer,
                                  BUFFER_SIZE, query_id, rtype, name.str,
                                  name.length, answer, 0, 0, additional, 2);
      } else {
        mdns_query_answer_multicast(sock, message_buffer, BUFFER_SIZE, answer,
                                    0, 0, additional, 2);
      }
    }
  }

  return 0;
}

// Open client sockets for sending queries
static void open_client_sockets(void) {
  num_client_sockets = 0;

  // This is a simplified version - in production code, you would open
  // sockets for each network interface
  struct sockaddr_in addr;
  memset(&addr, 0, sizeof(addr));
  addr.sin_family = AF_INET;
  addr.sin_addr.s_addr = INADDR_ANY;
  addr.sin_port = 0;  // Let the OS assign a port

  int sock = mdns_socket_open_ipv4(&addr);
  if (sock >= 0) {
    client_sockets[num_client_sockets++] = sock;
    printf("Opened client socket\n");
  } else {
    printf("Failed to open client socket\n");
  }
}

// Open service socket for listening
static void open_service_socket(void) {
  struct sockaddr_in sock_addr;
  memset(&sock_addr, 0, sizeof(sock_addr));
  sock_addr.sin_family = AF_INET;
  sock_addr.sin_addr.s_addr = INADDR_ANY;
  sock_addr.sin_port = htons(MDNS_PORT);

  service_socket = mdns_socket_open_ipv4(&sock_addr);
  if (service_socket >= 0) {
    printf("Service socket opened successfully\n");
  } else {
    printf("Failed to open service socket: %s\n", strerror(errno));
  }
}

// Query for existing service
static void query_for_service(void) {
  printf("Querying for existing dllmd service...\n");

  if (num_client_sockets == 0) {
    open_client_sockets();
    if (num_client_sockets == 0) {
      printf("No client sockets available for querying\n");
      return;
    }
  }

  for (int i = 0; i < num_client_sockets; i++) {
    mdns_query_send(client_sockets[i], MDNS_RECORDTYPE_SRV, DLLMD_SERVICE_NAME,
                    strlen(DLLMD_SERVICE_NAME), message_buffer, BUFFER_SIZE, 0);
  }

  // Give time for responses to come in
  struct timeval timeout;
  timeout.tv_sec = 1;
  timeout.tv_usec = 0;

  fd_set readfds;
  int nfds = 0;

  for (int attempt = 0; attempt < 3; attempt++) {
    FD_ZERO(&readfds);
    for (int i = 0; i < num_client_sockets; i++) {
      FD_SET(client_sockets[i], &readfds);
      if (client_sockets[i] >= nfds) nfds = client_sockets[i] + 1;
    }

    if (select(nfds, &readfds, NULL, NULL, &timeout) >= 0) {
      for (int i = 0; i < num_client_sockets; i++) {
        // if (FD_SET(client_sockets[i], &readfds)) {
        mdns_query_recv(client_sockets[i], message_buffer, BUFFER_SIZE,
                        query_callback, NULL, 0);
        // }
      }
    } else {
      // TODO: should I log here?
      perror("select");
    }
  }

  // If no service found after all attempts, become the service
  if (!service_node.active) {
    printf("No existing dllmd service found\n");
    become_service();
  }
}

// Send ping to service
static void send_ping(void) {
  if (node_type == NODE_TYPE_CLIENT && service_node.active) {
    printf("Sending ping to service\n");

    for (int i = 0; i < num_client_sockets; i++) {
      // Create a TXT record for ping
      mdns_record_t ping_record = {
          .name = {DLLMD_SERVICE_NAME, strlen(DLLMD_SERVICE_NAME)},
          .type = MDNS_RECORDTYPE_TXT,
          .data.txt.key = {MDNS_STRING_CONST("ping")},
          .data.txt.value = {MDNS_STRING_CONST("1")},
          .rclass = MDNS_CLASS_IN,
          .ttl = 1};

      // Multicast the ping
      mdns_query_answer_multicast(client_sockets[i], message_buffer,
                                  BUFFER_SIZE, ping_record, 0, 0, 0, 0);
    }

    last_ping_time = get_current_time();
  }
}

// Check if service is still alive
static void check_service_health(void) {
  if (node_type == NODE_TYPE_CLIENT && service_node.active) {
    time_t current_time = get_current_time();

    // If we haven't seen the service for PING_TIMEOUT seconds, consider it dead
    if (current_time - service_node.last_seen > PING_TIMEOUT) {
      printf("Service node appears to be down! Last seen %ld seconds ago\n",
             current_time - service_node.last_seen);

      // Query for service one more time to confirm it's down
      query_for_service();

      // If still no response, prepare to become the service
      if (!service_node.active) {
        service_node.active = 0;

        // Wait random time to avoid race conditions with other nodes
        int delay = rand() % RANDOM_DELAY_MAX + 1;
        printf(
            "Will become service in %d seconds if no other node takes over\n",
            delay);
        sleep(delay);

        // Query one more time
        query_for_service();

        // If still no service, become one
        if (!service_node.active) {
          become_service();
        }
      }
    }
  }
}

// Become a service node
static void become_service(void) {
  printf("Becoming dllmd service node\n");

  // Open service socket if not already open
  if (service_socket < 0) {
    open_service_socket();
    if (service_socket < 0) {
      printf("Failed to open service socket, can't become service\n");
      return;
    }
  }

  node_type = NODE_TYPE_SERVICE;

  // Create service instance name: hostname._dllmd._udp.local.
  char service_instance[512];
  snprintf(service_instance, sizeof(service_instance), "%s.%s", hostname_buffer,
           DLLMD_SERVICE_NAME);

  // Create hostname.local. for SRV record
  char hostname_local[256];
  snprintf(hostname_local, sizeof(hostname_local), "%s.local.",
           hostname_buffer);

  // Announce the service
  mdns_record_t ptr_record = {
      .name = {DLLMD_SERVICE_NAME, strlen(DLLMD_SERVICE_NAME)},
      .type = MDNS_RECORDTYPE_PTR,
      .data.ptr.name = {service_instance, strlen(service_instance)},
      .rclass = MDNS_CLASS_IN,
      .ttl = 60};

  mdns_record_t srv_record = {
      .name = {service_instance, strlen(service_instance)},
      .type = MDNS_RECORDTYPE_SRV,
      .data.srv.name = {hostname_local, strlen(hostname_local)},
      .data.srv.port = atoi(DLLMD_PORT),
      .data.srv.priority = 0,
      .data.srv.weight = 0,
      .rclass = MDNS_CLASS_IN,
      .ttl = 60};

  mdns_record_t txt_record = {
      .name = {service_instance, strlen(service_instance)},
      .type = MDNS_RECORDTYPE_TXT,
      .data.txt.key = {MDNS_STRING_CONST("info")},
      .data.txt.value = {MDNS_STRING_CONST("dllmd service")},
      .rclass = MDNS_CLASS_IN,
      .ttl = 60};

  mdns_record_t additional[2] = {srv_record, txt_record};

  // Send service announcement
  mdns_announce_multicast(service_socket, message_buffer, BUFFER_SIZE,
                          ptr_record, 0, 0, additional, 2);

  printf("Service announced\n");
}

// Process incoming messages
static void process_incoming_messages(void) {
  fd_set readfds;
  int nfds = 0;

  FD_ZERO(&readfds);

  // Add client sockets
  for (int i = 0; i < num_client_sockets; i++) {
    FD_SET(client_sockets[i], &readfds);
    if (client_sockets[i] >= nfds) nfds = client_sockets[i] + 1;
  }

  // Add service socket if we're a service
  if (node_type == NODE_TYPE_SERVICE && service_socket >= 0) {
    FD_SET(service_socket, &readfds);
    if (service_socket >= nfds) nfds = service_socket + 1;
  }

  struct timeval timeout;
  timeout.tv_sec = 0;
  timeout.tv_usec = 100000;  // 100ms

  if (select(nfds, &readfds, NULL, NULL, &timeout) > 0) {
    // Check client sockets
    for (int i = 0; i < num_client_sockets; i++) {
      if (FD_ISSET(client_sockets[i], &readfds)) {
        mdns_query_recv(client_sockets[i], message_buffer, BUFFER_SIZE,
                        query_callback, NULL, 0);
      }
    }

    // Check service socket
    if (node_type == NODE_TYPE_SERVICE && service_socket >= 0 &&
        FD_ISSET(service_socket, &readfds)) {
      mdns_socket_listen(service_socket, message_buffer, BUFFER_SIZE,
                         service_callback, NULL);
    }
  }
}

int dllmd_main(int argc, char* argv[]) {
  // Initialize
  srand((unsigned int)time(NULL));
  memset(&service_node, 0, sizeof(service_node));
  memset(known_nodes, 0, sizeof(known_nodes));

  // Get hostname
  gethostname(hostname_buffer, sizeof(hostname_buffer));

  printf("DLLMD started on host: %s\n", hostname_buffer);

  // Allocate message buffer
  message_buffer = malloc(BUFFER_SIZE);
  if (!message_buffer) {
    printf("Failed to allocate message buffer\n");
    return -1;
  }

  // Set up signal handler
  signal(SIGINT, signal_handler);

  // Open client sockets
  open_client_sockets();

  // Query for existing service
  query_for_service();

  // Main loop
  while (running) {
    time_t current_time = get_current_time();

    // If client, ping service periodically
    if (node_type == NODE_TYPE_CLIENT) {
      if (current_time - last_ping_time >= PING_INTERVAL) {
        send_ping();
        check_service_health();
      }
    }

    process_incoming_messages();

    // Sleep a bit to avoid busy waiting
    usleep(10000);  // 10ms
  }

  // Clean up
  free(message_buffer);

  // Close client sockets
  for (int i = 0; i < num_client_sockets; i++) {
    mdns_socket_close(client_sockets[i]);
  }

  // Close service socket if open
  if (service_socket >= 0) {
    mdns_socket_close(service_socket);
  }

  printf("DLLMD terminated\n");
  return 0;
}
