#include <signal.h>
#include <stdio.h>
#include <unistd.h>

#include "dllmd.h"

static int is_running = 1;

static inline void signal_handler(int signal) {
  (void)signal; // unused
  is_running = 0;
}

int main() {
  signal(SIGINT, &signal_handler);

  dllmd_t *dllmd = dllmd_new();
  if (!dllmd) {
    fprintf(stderr, "Failed to create dllmd instance\n");
    return 1;
  }

  // start listening
  dllmd_handle_t *dllm_handle = dllmd_start(dllmd, "/ip4/0.0.0.0/tcp/0");
  char buf[256];
  while (is_running) {
    printf("polling for messages...\n");
    int timeout_ms = 400;
    int bytes = dllmd_receive(dllmd, buf, sizeof(buf), timeout_ms);
    if (bytes < 0) {
      fprintf(stderr, "Failed to receive message\n");
      break;
    } else if (bytes == 0) {
      // no message received
      printf("No message received\n");
      continue;
    } else {
      printf("Received %d bytes:\n", bytes);
      fwrite(buf, 1, bytes, stdout);
      printf("\n");
    }
  }

  // cleanups
  dllmd_stop(dllmd, dllm_handle);
  dllmd_free(dllmd);

  // reset signal handler to default
  signal(SIGINT, SIG_DFL);
  printf("Bye!");
  return 0;
}
