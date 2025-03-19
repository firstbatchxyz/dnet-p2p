#include <stdio.h>
#include <string.h>

// mdns example
#include "mdns.h"

// dllm dameon
#include "dllmd.h"

int main(int argc, char *argv[]) {
  if (argc < 2) {
    fprintf(stderr, "usage: %s [showip|client|server] [args...]\n", argv[0]);
    return 1;
  }

  if (strcmp(argv[1], "mdns") == 0) {
    return mdns_main(argc - 1, &argv[1]);
  } else if (strcmp(argv[1], "dllmd") == 0) {
    return dllmd_main(argc - 1, &argv[1]);
  } else {
    fprintf(stderr, "unknown program: %s\n", argv[1]);
    return 1;
  }
}
