/*
** server.c -- a stream socket server demo that creates a TCP server
** listening for incoming connections and sending "Hello, world!" to clients
*/

// Include necessary header files for networking, process management, and I/O
#include <arpa/inet.h>
#include <errno.h>
#include <netdb.h>
#include <netinet/in.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

#include "util.h"

#define SERVER_PORT "3490"  // Port number where server will listen
#define SERVER_BACKLOG 10   // Maximum number of pending connections in queue

// Handler for cleaning up zombie child processes
void sigchld_handler(int s) {
  (void)s;  // quiet unused variable warning

  // Save errno because waitpid might change it
  int saved_errno = errno;

  // Collect any dead child processes to prevent zombies
  while (waitpid(-1, NULL, WNOHANG) > 0);

  errno = saved_errno;
}

int server_main(void) {
  int sockfd;                            // Main socket file descriptor for listening
  int new_fd;                            // New socket file descriptor for accepted connections
  struct addrinfo hints, *servinfo, *p;  // Structures for storing address info
  struct sockaddr_storage their_addr;    // Client's address information
  socklen_t sin_size;
  struct sigaction sa;
  int yes = 1;                         // Used for setsockopt
  char client_addr[INET6_ADDRSTRLEN];  // Buffer to store IP address string
  int rv;

  // Initialize the hints structure with zeros
  memset(&hints, 0, sizeof hints);
  hints.ai_family = AF_UNSPEC;      // Use IPv4 or IPv6
  hints.ai_socktype = SOCK_STREAM;  // TCP stream sockets
  hints.ai_flags = AI_PASSIVE;      // Fill in my IP automatically

  // Get address information for binding
  if ((rv = getaddrinfo(NULL, SERVER_PORT, &hints, &servinfo)) != 0) {
    fprintf(stderr, "getaddrinfo: %s\n", gai_strerror(rv));
    return 1;
  }

  // Loop through all results and try to create and bind a socket
  for (p = servinfo; p != NULL; p = p->ai_next) {
    // Create a socket
    if ((sockfd = socket(p->ai_family, p->ai_socktype, p->ai_protocol)) == -1) {
      perror("server: socket");
      continue;
    }

    // Set socket option to reuse the address
    // (prevents "Address already in use" errors)
    if (setsockopt(sockfd, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof(int)) == -1) {
      perror("setsockopt");
      exit(1);
    }

    // Try to bind the socket to the address
    if (bind(sockfd, p->ai_addr, p->ai_addrlen) == -1) {
      close(sockfd);
      perror("server: bind");
      continue;
    }

    break;  // Successfully bound the socket
  }

  freeaddrinfo(servinfo);  // Free the linked list of addresses

  // Check if we successfully bound the socket
  if (p == NULL) {
    fprintf(stderr, "server: failed to bind\n");
    exit(1);
  }

  // Start listening for incoming connections
  if (listen(sockfd, SERVER_BACKLOG) == -1) {
    perror("listen");
    exit(1);
  }

  // Set up signal handler for child processes
  sa.sa_handler = sigchld_handler;
  sigemptyset(&sa.sa_mask);
  sa.sa_flags = SA_RESTART;
  if (sigaction(SIGCHLD, &sa, NULL) == -1) {
    perror("sigaction");
    exit(1);
  }

  printf("server: waiting for connections...\n");

  // Main server loop
  while (1) {
    sin_size = sizeof their_addr;
    // Accept incoming connection
    new_fd = accept(sockfd, (struct sockaddr *)&their_addr, &sin_size);
    if (new_fd == -1) {
      perror("accept");
      continue;
    }

    // Convert client's IP address to string and print it
    inet_ntop(their_addr.ss_family, get_in_addr((struct sockaddr *)&their_addr), client_addr, sizeof client_addr);
    printf("server: got connection from %s\n", client_addr);

    // Fork a child process to handle the connection
    if (!fork()) {    // Child process
      close(sockfd);  // Child doesn't need the listening socket
      // Send "Hello, world!" to the client
      if (send(new_fd, "Hello, world!", 13, 0) == -1) perror("send");
      close(new_fd);
      exit(0);
    }
    close(new_fd);  // Parent process closes the new socket
  }

  return 0;
}
