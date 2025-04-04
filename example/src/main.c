#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

#include "dllmd.h"
#include "echo.h"

int is_running = 1;
void trap(int signal) { is_running = 0; }

int main() {
  signal(SIGINT, &trap);

  dllmd_t *dllmd = dllmd_new();
  if (!dllmd) {
    fprintf(stderr, "Failed to create dllmd instance\n");
    return 1;
  }

  // start listening
  dllmd_handle_t *dllm_handle = dllmd_start(dllmd, "/ip4/0.0.0.0/tcp/0");
  while (is_running) {
    // wait for incoming connections, or poll messages etc.
    printf("Waiting for incoming connections...\n");
    sleep(1);
  }

  // cleanups
  dllmd_stop(dllmd, dllm_handle);
  dllmd_free(dllmd);

  signal(SIGINT, SIG_DFL);
  printf("Bye!");
  return 0;
}
