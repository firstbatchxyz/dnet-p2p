/**
 * dllmd.c -- Distributed Leader Election Daemon using mDNS
 *
 * This file implements a distributed leader election protocol using mDNS for
 * service discovery and communication. The system allows nodes to:
 *
 * 1. Discover existing service nodes (leaders) on the network
 * 2. Monitor service health through periodic pings
 * 3. Automatically elect a new leader when an existing one fails
 * 4. Announce leadership through mDNS service records
 * 5. Maintain awareness of other nodes in the network
 *
 * The implementation uses a simple election algorithm where nodes wait a random
 * time after detecting leader failure before attempting to become the new
 * leader, helping to avoid election conflicts.
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

/// `41891 = 4 18 9 1 = D R I A`
#define DLLMD_PORT "41891"
/// Service name, as defined by RFC6762 to be `<srv>.local.`
#define DLLMD_SERVICE_NAME "_dllmd._tcp.local."
/// Check for service every 5 seconds
#define PING_INTERVAL 5
/// Consider service dead after 15 seconds of no response
#define PING_TIMEOUT 15
/// Maximum number of nodes to track
#define MAX_NODES 32
/// Maximum random delay for service election in seconds
#define RANDOM_DELAY_MAX 5

typedef enum {
  /// Regular node
  NODE_TYPE_CLIENT,
  /// Service node (leader)
  NODE_TYPE_SERVICE
} node_type_t;

typedef struct {
  char hostname[256];

  struct sockaddr_storage addr;
  time_t last_seen;
  int active;
} node_info_t;

/// A flag to indicate if the daemon is running.
static volatile bool is_running;
static node_type_t node_type = NODE_TYPE_CLIENT;

static node_info_t service_node;
static int service_socket = -1;

static node_info_t known_nodes[MAX_NODES];
static int node_count = 0;

static time_t last_ping_time = 0;

static char hostname_buffer[256];
static char message_buffer[2048];

static int client_sockets[32];
static int num_client_sockets = 0;

/// Handle interrupt signal to gracefully terminate the daemon.
static inline void signal_handler(int sig) {
  (void)sig;  // unused
  is_running = 0;
}

/// Get current time in seconds since epoch.
static inline time_t get_current_time(void) { return time(NULL); }

/** 
 * Compare two sockaddr structures to determine if they refer to the same address.
 * @return `true` if addresses are equal, `false` otherwise
 */
static bool sockaddr_equal(const struct sockaddr* a, const struct sockaddr* b) {
  if (a->sa_family != b->sa_family) {
    // address family mismatch (e.g. AF_INET vs AF_INET6)
    return false;
  }

  switch (a->sa_family) {
    case AF_INET: {
      const struct sockaddr_in* a_in = (const struct sockaddr_in*)a;
      const struct sockaddr_in* b_in = (const struct sockaddr_in*)b;

      return a_in->sin_addr.s_addr == b_in->sin_addr.s_addr && a_in->sin_port == b_in->sin_port;
    }

    case AF_INET6: {
      const struct sockaddr_in6* a_in6 = (const struct sockaddr_in6*)a;
      const struct sockaddr_in6* b_in6 = (const struct sockaddr_in6*)b;

      return memcmp(&a_in6->sin6_addr, &b_in6->sin6_addr, sizeof(struct in6_addr)) == 0 &&
             a_in6->sin6_port == b_in6->sin6_port;
    }

    default:
      // unsupported address family
      return false;
  }
}

int dllmd_main(int argc, char* argv[]) {
  // silence unused args (we may use them later)
  (void)argc;
  (void)argv;

  srand((unsigned int)time(NULL));
  memset(&service_node, 0, sizeof(service_node));
  memset(known_nodes, 0, sizeof(known_nodes));

  // get hostname (we enforce it), it is very unlikely that mutliple devices have the same hostname
  if (gethostname(hostname_buffer, sizeof(hostname_buffer)) != 0) {
    perror("gethostname");
    return 1;
  }
  printf("ddlmd started on host: %s\n", hostname_buffer);

  // set up signal handler for SIGINT
  signal(SIGINT, signal_handler);

  // make a query to see if there is an existing service
  printf("Querying for existing service nodes...\n");

  is_running = true;
  while (is_running) {
    time_t current_time = get_current_time();

    // TODO: !!!

    // sleep a bit to avoid busy waiting
    usleep(10000);  // 10ms
  }

  // close client sockets
  for (int i = 0; i < num_client_sockets; i++) {
    mdns_socket_close(client_sockets[i]);
  }

  // close service socket if open
  if (service_socket >= 0) {
    mdns_socket_close(service_socket);
  }

  printf("\ndllmd terminated, bye.\n");
  return 0;
}
