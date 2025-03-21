#ifndef DLLMD_H
#define DLLMD_H

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

#include "util.h"
#include "mdns.h"
#include "sockets.h"

int dllmd_main(int argc, char* argv[]);

#endif  // DLLMD_H
