/*
** client.c -- a stream socket client demo
** This program implements a basic TCP client that connects to a server
** and receives a message from it.
*/

// Required header files for networking, I/O, and system calls
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

// get sockaddr, IPv4 or IPv6:
void *get_in_addr(struct sockaddr *sa) {
  if (sa->sa_family == AF_INET) {
    return &(((struct sockaddr_in *)sa)->sin_addr);
  } else {
    return &(((struct sockaddr_in6 *)sa)->sin6_addr);
  }
}

// Configuration constants
#define CLIENT_PORT "3490"      // The port number the client will connect to
#define CLIENT_MAXDATASIZE 100  // Maximum size of received data buffer

int client_main(int argc, char *argv[]) {
  // Check if hostname was provided as command line argument
  if (argc != 2) {
    fprintf(stderr, "usage: client hostname\n");
    exit(1);
  }

  // Initialize the hints structure with zeros
  struct addrinfo hints;
  memset(&hints, 0, sizeof hints);
  hints.ai_family = AF_UNSPEC;      // Use IPv4 or IPv6, whichever address family
  hints.ai_socktype = SOCK_STREAM;  // Use TCP stream sockets

  // Get address information for the server
  int rv;                     // Return value (rv) for getaddrinfo
  struct addrinfo *servinfo;  // Linked list of address information
  if ((rv = getaddrinfo(argv[1], CLIENT_PORT, &hints, &servinfo)) != 0) {
    fprintf(stderr, "getaddrinfo: %s\n", gai_strerror(rv));
    return 1;
  }

  // Loop through all the results and try to connect to each address
  int sockfd;             // Socket file descriptor
  struct addrinfo *conn;  // Chosen connection
  for (conn = servinfo; conn != NULL; conn = conn->ai_next) {
    // Try to create a socket
    if ((sockfd = socket(conn->ai_family, conn->ai_socktype, conn->ai_protocol)) == -1) {
      // If failed, try next address
      perror("client: socket");
      continue;
    }

    // Try to connect to the server
    if (connect(sockfd, conn->ai_addr, conn->ai_addrlen) == -1) {
      // If failed, try next address, but close the socket as well
      close(sockfd);
      perror("client: connect");
      continue;
    }

    break;  // If we get here, we successfully connected
  }

  // Check if we failed to connect to any address
  if (conn == NULL) {
    fprintf(stderr, "client: failed to connect\n");
    return 2;
  }

  // Convert the server's address to string format and print it
  // we use ntop (network to presentation) to convert the address
  char server_addr[INET6_ADDRSTRLEN];  // String to hold IP address
  inet_ntop(conn->ai_family, get_in_addr((struct sockaddr *)conn->ai_addr), server_addr, sizeof server_addr);
  printf("client: connecting to %s\n", server_addr);

  // Free the linked list of addresses as we don't need it anymore
  freeaddrinfo(servinfo);

  // Receive data from the server
  int numbytes;                  // Number of bytes received
  char buf[CLIENT_MAXDATASIZE];  // Buffer to store received data
  if ((numbytes = recv(sockfd, buf, CLIENT_MAXDATASIZE - 1, 0)) == -1) {
    perror("recv");
    exit(1);
  }

  // Null-terminate the received data to make it a proper string
  buf[numbytes] = '\0';

  // Print the received message
  printf("client: received '%s'\n", buf);

  // Close the socket
  close(sockfd);

  return 0;
}
