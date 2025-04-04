#include <signal.h>
#include <stdio.h>
#include <string.h>
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
  while (is_running) {
    // TODO: what should be done here?
    sleep(1);
  }

  // publish a message as you are closing
  const char *data = "Bye bye, world!";
  dllmd_publish(dllmd, data, strlen(data));
  sleep(1);

  // cleanups
  dllmd_stop(dllmd, dllm_handle);
  dllmd_free(dllmd);

  signal(SIGINT, SIG_DFL);
  printf("Bye!");
  return 0;
}
