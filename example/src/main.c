#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

#include "dnet.h"

static int is_running = 1;

static inline void signal_handler(int signal) {
  (void)signal; // unused
  is_running = 0;
}

static void listen_mode(dnet_t *dnet) {
  char buf[256];
  while (is_running) {
    int timeout_ms = 400;
    int bytes = dnet_receive(dnet, buf, sizeof(buf), timeout_ms);
    if (bytes < 0) {
      fprintf(stderr, "Failed to receive message\n");
      break;
    } else if (bytes == 0) {
      continue;
    } else {
      printf("Received %d bytes.", bytes);
    }
  }
}

static void send_mode(dnet_t *dnet, const char *message) {
  int ret = dnet_publish(dnet, message, strlen(message));
  if (ret != 0) {
    fprintf(stderr, "Failed to publish message: %d\n", ret);
  } else {
    printf("Published message: %s\n", message);
  }
}

static void matmul_example() { printf("TODO: Matrix multiplication\n"); }

int main(int argc, char *argv[]) {
  if (argc < 2) {
    fprintf(stderr, "Usage: %s <command> [args]\n", argv[0]);
    fprintf(stderr, "Commands:\n");
    fprintf(stderr, "  listen\n");
    fprintf(stderr, "  send <message>\n");
    fprintf(stderr, "  matmul\n");
    return 1;
  }

  signal(SIGINT, &signal_handler);
  dnet_t *dnet = dnet_new();
  if (!dnet) {
    fprintf(stderr, "Failed to create dnet instance\n");
    return 1;
  }
  dnet_handle_t *dllm_handle = dnet_start(dnet, "/ip4/0.0.0.0/tcp/0");

  if (strcmp(argv[1], "listen") == 0) {
    listen_mode(dnet);
  } else if (strcmp(argv[1], "send") == 0) {
    if (argc < 3) {
      fprintf(stderr, "Send command requires a message\n");
    } else {
      send_mode(dnet, argv[2]);
    }
  } else if (strcmp(argv[1], "matmul") == 0) {
    matmul_example();
  } else {
    fprintf(stderr, "Unknown command: %s\n", argv[1]);
  }

  dnet_stop(dnet, dllm_handle);
  dnet_free(dnet);
  signal(SIGINT, SIG_DFL);
  printf("Bye!\n");
  return 0;
}
