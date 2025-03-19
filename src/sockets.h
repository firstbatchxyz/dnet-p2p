#ifndef DLLMD_SOCKETS_H
#define DLLMD_SOCKETS_H

#include <arpa/inet.h>

static int open_client_sockets(int* sockets, int max_sockets, int port);

static int open_service_sockets(int* sockets, int max_sockets);

#endif  // DLLMD_SOCKETS_H