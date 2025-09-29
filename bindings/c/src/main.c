#include <signal.h>
#include <stdio.h>
#include <unistd.h>

#include "dnet_p2p.h"

static int is_running = 1;

static inline void signal_handler(int signal) {
  (void)signal; // unused
  is_running = 0;
}

static void listen_mode(dnet_p2p_t *dnet) {
  while (is_running) {
    sleep(1);
    // TODO: get peer info and log them here?
  }
}

/// Example main function for a dnet P2P service
///
/// Simply starts the dnet P2P service with a given instance name.
///
/// Stops the service on SIGINT (Ctrl+C).
int main(int argc, char *argv[]) {
  // parse args, we only have one command for now though
  if (argc < 2) {
    fprintf(stderr, "Usage: %s <instance-name>\n", argv[0]);
    return 1;
  }

  // get hostname here with syscall
  char hostname[256];
  if (gethostname(hostname, sizeof(hostname)) != 0) {
    perror("gethostname");
    return 1;
  }

  // enable logging for dnet, respecting RUST_LOG env variable
  dnet_p2p_enable_logs();

  // create dnet instance
  const char *instance_name = argv[1];

  dnet_p2p_t *dnet_p2p =
      dnet_p2p_new(instance_name, hostname, "localhost" /* host */, 8080 /* server_port */, 50501 /* shard_port */, false /* not manager */, false /* not passive */);
  if (!dnet_p2p) {
    fprintf(stderr, "Failed to create dnet instance\n");
    return 1;
  }

  signal(SIGINT, &signal_handler);

  // start the dnet daemon
  dnet_p2p_handle_t *dnet_handle = dnet_p2p_start(dnet_p2p);
  listen_mode(dnet_p2p);

  // stop the dnet daemon
  dnet_p2p_stop(dnet_p2p, dnet_handle);
  dnet_p2p_free(dnet_p2p);

  // reset signal handler
  signal(SIGINT, SIG_DFL);
  printf("Bye!\n");
  return 0;
}
