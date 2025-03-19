/*
** listener.c -- a datagram sockets "server" demo
*/

#include <arpa/inet.h>
#include <errno.h>
#include <netdb.h>
#include <netinet/in.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

#define LISTENER_MYPORT "4950"  // the port users will be connecting to
#define LISTENER_MAXBUFLEN 100

// get sockaddr, IPv4 or IPv6:
void *get_in_addr(struct sockaddr *sa) {
  if (sa->sa_family == AF_INET) {
    return &(((struct sockaddr_in *)sa)->sin_addr);
  }

  return &(((struct sockaddr_in6 *)sa)->sin6_addr);
}

int listener_main(void) {
  char buf[LISTENER_MAXBUFLEN];

  struct addrinfo hints;
  memset(&hints, 0, sizeof hints);
  hints.ai_family = AF_INET6;  // or, set to AF_INET to use IPv4
  hints.ai_socktype = SOCK_DGRAM;
  hints.ai_flags = AI_PASSIVE;  // use my IP

  struct addrinfo *servinfo;
  int rv;
  if ((rv = getaddrinfo(NULL, LISTENER_MYPORT, &hints, &servinfo)) != 0) {
    fprintf(stderr, "getaddrinfo: %s\n", gai_strerror(rv));
    return 1;
  }

  // loop through all the results and bind to the first we can
  int sockfd;
  struct addrinfo *conn;
  for (conn = servinfo; conn != NULL; conn = conn->ai_next) {
    if ((sockfd = socket(conn->ai_family, conn->ai_socktype, conn->ai_protocol)) == -1) {
      perror("listener: socket");
      continue;
    }

    if (bind(sockfd, conn->ai_addr, conn->ai_addrlen) == -1) {
      close(sockfd);
      perror("listener: bind");
      continue;
    }

    break;
  }

  if (conn == NULL) {
    fprintf(stderr, "listener: failed to bind socket\n");
    return 2;
  }

  freeaddrinfo(servinfo);

  printf("listener: waiting to recvfrom...\n");
  int numbytes;  // number of bytes received
  struct sockaddr_storage their_addr;
  socklen_t addr_len;
  addr_len = sizeof their_addr;
  if ((numbytes = recvfrom(sockfd, buf, LISTENER_MAXBUFLEN - 1, 0, (struct sockaddr *)&their_addr, &addr_len)) == -1) {
    perror("recvfrom");
    exit(1);
  }

  char server_addr[INET6_ADDRSTRLEN];
  printf("listener: got packet from %s\n",
         inet_ntop(their_addr.ss_family, get_in_addr((struct sockaddr *)&their_addr), server_addr, sizeof server_addr));
  printf("listener: packet is %d bytes long\n", numbytes);
  buf[numbytes] = '\0';
  printf("listener: packet contains \"%s\"\n", buf);

  close(sockfd);

  return 0;
}