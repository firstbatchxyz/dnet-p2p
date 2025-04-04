#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

#include "dllmd.h"
#include "echo.h"

int main() {
  dllmd_t *dllmd = dllmd_new();
  if (!dllmd) {
    fprintf(stderr, "Failed to create dllmd instance\n");
    return 1;
  }

  // start listening
  dllmd_start_daemon(dllmd, "/ip4/0.0.0.0/tcp/0");

  printf("Waiting a bit\n");
  sleep(5);
  dllmd_shutdown(dllmd);
  dllmd_free(dllmd);
  printf("Bye!");
  return 0;
}
