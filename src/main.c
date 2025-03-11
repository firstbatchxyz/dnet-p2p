#include <stdio.h>
#include <string.h>

#include "client.h"
#include "mdns.h"
#include "server.h"
#include "showip.h"

int main(int argc, char *argv[]) {
  if (argc < 2) {
    fprintf(stderr, "usage: %s [showip|client|server] [args...]\n", argv[0]);
    return 1;
  }

  if (strcmp(argv[1], "showip") == 0) {
    return showip_main(argc - 1, &argv[1]);
  } else if (strcmp(argv[1], "client") == 0) {
    return client_main(argc - 1, &argv[1]);
  } else if (strcmp(argv[1], "server") == 0) {
    return server_main();
  } else if (strcmp(argv[1], "mdns") == 0) {
    return mdns_main(argc - 1, &argv[1]);
  } else {
    fprintf(stderr, "unknown program: %s\n", argv[1]);
    return 1;
  }
}
