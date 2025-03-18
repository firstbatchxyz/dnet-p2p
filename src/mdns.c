#include "mdns.h"

#include <errno.h>
#include <ifaddrs.h>
#include <net/if.h>
#include <netdb.h>
#include <signal.h>
#include <stdio.h>
#include <sys/time.h>

// static buffers
static char addrbuffer[64];
static char entrybuffer[256];
static char namebuffer[256];
static char sendbuffer[1024];
static mdns_record_txt_t txtbuffer[128];

static struct sockaddr_in service_address_ipv4;
static struct sockaddr_in6 service_address_ipv6;

static int has_ipv4;
static int has_ipv6;

// flag to indicate if the daemon is running
volatile sig_atomic_t running = 1;

// Data for our service including the mDNS records
typedef struct {
  mdns_string_t service;
  mdns_string_t hostname;
  mdns_string_t service_instance;
  mdns_string_t hostname_qualified;
  struct sockaddr_in address_ipv4;
  struct sockaddr_in6 address_ipv6;
  int port;
  mdns_record_t record_ptr;
  mdns_record_t record_srv;
  mdns_record_t record_a;
  mdns_record_t record_aaaa;
  mdns_record_t txt_record[2];
} service_t;

static mdns_string_t ipv4_address_to_string(char* buffer, size_t capacity, const struct sockaddr_in* addr,
                                            size_t addrlen) {
  char host[NI_MAXHOST] = {0};
  char service[NI_MAXSERV] = {0};
  int ret = getnameinfo((const struct sockaddr*)addr, (socklen_t)addrlen, host, NI_MAXHOST, service, NI_MAXSERV,
                        NI_NUMERICSERV | NI_NUMERICHOST);
  int len = 0;
  if (ret == 0) {
    if (addr->sin_port != 0)
      len = snprintf(buffer, capacity, "%s:%s", host, service);
    else
      len = snprintf(buffer, capacity, "%s", host);
  }
  if (len >= (int)capacity) len = (int)capacity - 1;
  mdns_string_t str;
  str.str = buffer;
  str.length = len;
  return str;
}

static mdns_string_t ipv6_address_to_string(char* buffer, size_t capacity, const struct sockaddr_in6* addr,
                                            size_t addrlen) {
  char host[NI_MAXHOST] = {0};
  char service[NI_MAXSERV] = {0};
  int ret = getnameinfo((const struct sockaddr*)addr, (socklen_t)addrlen, host, NI_MAXHOST, service, NI_MAXSERV,
                        NI_NUMERICSERV | NI_NUMERICHOST);
  int len = 0;
  if (ret == 0) {
    if (addr->sin6_port != 0)
      len = snprintf(buffer, capacity, "[%s]:%s", host, service);
    else
      len = snprintf(buffer, capacity, "%s", host);
  }
  if (len >= (int)capacity) len = (int)capacity - 1;
  mdns_string_t str;
  str.str = buffer;
  str.length = len;
  return str;
}

static mdns_string_t ip_address_to_string(char* buffer, size_t capacity, const struct sockaddr* addr, size_t addrlen) {
  if (addr->sa_family == AF_INET6)
    return ipv6_address_to_string(buffer, capacity, (const struct sockaddr_in6*)addr, addrlen);
  return ipv4_address_to_string(buffer, capacity, (const struct sockaddr_in*)addr, addrlen);
}

// Callback handling parsing answers to queries sent
static int query_callback(int sock, const struct sockaddr* from, size_t addrlen, mdns_entry_type_t entry,
                          uint16_t query_id, uint16_t rtype, uint16_t rclass, uint32_t ttl, const void* data,
                          size_t size, size_t name_offset, size_t name_length, size_t record_offset,
                          size_t record_length, void* user_data) {
  (void)sizeof(sock);
  (void)sizeof(query_id);
  (void)sizeof(name_length);
  (void)sizeof(user_data);
  mdns_string_t fromaddrstr = ip_address_to_string(addrbuffer, sizeof(addrbuffer), from, addrlen);
  const char* entrytype =
      (entry == MDNS_ENTRYTYPE_ANSWER) ? "answer" : ((entry == MDNS_ENTRYTYPE_AUTHORITY) ? "authority" : "additional");
  mdns_string_t entrystr = mdns_string_extract(data, size, &name_offset, entrybuffer, sizeof(entrybuffer));
  if (rtype == MDNS_RECORDTYPE_PTR) {
    mdns_string_t namestr =
        mdns_record_parse_ptr(data, size, record_offset, record_length, namebuffer, sizeof(namebuffer));
    printf("%.*s : %s %.*s PTR %.*s rclass 0x%x ttl %u length %d\n", MDNS_STRING_FORMAT(fromaddrstr), entrytype,
           MDNS_STRING_FORMAT(entrystr), MDNS_STRING_FORMAT(namestr), rclass, ttl, (int)record_length);
  } else if (rtype == MDNS_RECORDTYPE_SRV) {
    mdns_record_srv_t srv =
        mdns_record_parse_srv(data, size, record_offset, record_length, namebuffer, sizeof(namebuffer));
    printf("%.*s : %s %.*s SRV %.*s priority %d weight %d port %d\n", MDNS_STRING_FORMAT(fromaddrstr), entrytype,
           MDNS_STRING_FORMAT(entrystr), MDNS_STRING_FORMAT(srv.name), srv.priority, srv.weight, srv.port);
  } else if (rtype == MDNS_RECORDTYPE_A) {
    struct sockaddr_in addr;
    mdns_record_parse_a(data, size, record_offset, record_length, &addr);
    mdns_string_t addrstr = ipv4_address_to_string(namebuffer, sizeof(namebuffer), &addr, sizeof(addr));
    printf("%.*s : %s %.*s A %.*s\n", MDNS_STRING_FORMAT(fromaddrstr), entrytype, MDNS_STRING_FORMAT(entrystr),
           MDNS_STRING_FORMAT(addrstr));
  } else if (rtype == MDNS_RECORDTYPE_AAAA) {
    struct sockaddr_in6 addr;
    mdns_record_parse_aaaa(data, size, record_offset, record_length, &addr);
    mdns_string_t addrstr = ipv6_address_to_string(namebuffer, sizeof(namebuffer), &addr, sizeof(addr));
    printf("%.*s : %s %.*s AAAA %.*s\n", MDNS_STRING_FORMAT(fromaddrstr), entrytype, MDNS_STRING_FORMAT(entrystr),
           MDNS_STRING_FORMAT(addrstr));
  } else if (rtype == MDNS_RECORDTYPE_TXT) {
    size_t parsed = mdns_record_parse_txt(data, size, record_offset, record_length, txtbuffer,
                                          sizeof(txtbuffer) / sizeof(mdns_record_txt_t));
    for (size_t itxt = 0; itxt < parsed; ++itxt) {
      if (txtbuffer[itxt].value.length) {
        printf("%.*s : %s %.*s TXT %.*s = %.*s\n", MDNS_STRING_FORMAT(fromaddrstr), entrytype,
               MDNS_STRING_FORMAT(entrystr), MDNS_STRING_FORMAT(txtbuffer[itxt].key),
               MDNS_STRING_FORMAT(txtbuffer[itxt].value));
      } else {
        printf("%.*s : %s %.*s TXT %.*s\n", MDNS_STRING_FORMAT(fromaddrstr), entrytype, MDNS_STRING_FORMAT(entrystr),
               MDNS_STRING_FORMAT(txtbuffer[itxt].key));
      }
    }
  } else {
    printf("%.*s : %s %.*s type %u rclass 0x%x ttl %u length %d\n", MDNS_STRING_FORMAT(fromaddrstr), entrytype,
           MDNS_STRING_FORMAT(entrystr), rtype, rclass, ttl, (int)record_length);
  }
  return 0;
}

// Callback handling questions incoming on service sockets
static int service_callback(int sock, const struct sockaddr* from, size_t addrlen, mdns_entry_type_t entry,
                            uint16_t query_id, uint16_t rtype, uint16_t rclass, uint32_t ttl, const void* data,
                            size_t size, size_t name_offset, size_t name_length, size_t record_offset,
                            size_t record_length, void* user_data) {
  (void)sizeof(ttl);
  if (entry != MDNS_ENTRYTYPE_QUESTION) return 0;

  const char dns_sd[] = "_services._dns-sd._udp.local.";
  const service_t* service = (const service_t*)user_data;

  mdns_string_t fromaddrstr = ip_address_to_string(addrbuffer, sizeof(addrbuffer), from, addrlen);

  size_t offset = name_offset;
  mdns_string_t name = mdns_string_extract(data, size, &offset, namebuffer, sizeof(namebuffer));

  const char* record_name = 0;
  if (rtype == MDNS_RECORDTYPE_PTR)
    record_name = "PTR";
  else if (rtype == MDNS_RECORDTYPE_SRV)
    record_name = "SRV";
  else if (rtype == MDNS_RECORDTYPE_A)
    record_name = "A";
  else if (rtype == MDNS_RECORDTYPE_AAAA)
    record_name = "AAAA";
  else if (rtype == MDNS_RECORDTYPE_TXT)
    record_name = "TXT";
  else if (rtype == MDNS_RECORDTYPE_ANY)
    record_name = "ANY";
  else
    return 0;
  printf("Query %s %.*s\n", record_name, MDNS_STRING_FORMAT(name));

  if ((name.length == (sizeof(dns_sd) - 1)) && (strncmp(name.str, dns_sd, sizeof(dns_sd) - 1) == 0)) {
    if ((rtype == MDNS_RECORDTYPE_PTR) || (rtype == MDNS_RECORDTYPE_ANY)) {
      // The PTR query was for the DNS-SD domain, send answer with a PTR record
      // for the service name we advertise, typically on the
      // "<_service-name>._tcp.local." format

      // Answer PTR record reverse mapping "<_service-name>._tcp.local." to
      // "<hostname>.<_service-name>._tcp.local."
      mdns_record_t answer = {.name = name, .type = MDNS_RECORDTYPE_PTR, .data.ptr.name = service->service};

      // Send the answer, unicast or multicast depending on flag in query
      uint16_t unicast = (rclass & MDNS_UNICAST_RESPONSE);
      printf("  --> answer %.*s (%s)\n", MDNS_STRING_FORMAT(answer.data.ptr.name), (unicast ? "unicast" : "multicast"));

      if (unicast) {
        mdns_query_answer_unicast(sock, from, addrlen, sendbuffer, sizeof(sendbuffer), query_id, rtype, name.str,
                                  name.length, answer, 0, 0, 0, 0);
      } else {
        mdns_query_answer_multicast(sock, sendbuffer, sizeof(sendbuffer), answer, 0, 0, 0, 0);
      }
    }
  } else if ((name.length == service->service.length) && (strncmp(name.str, service->service.str, name.length) == 0)) {
    if ((rtype == MDNS_RECORDTYPE_PTR) || (rtype == MDNS_RECORDTYPE_ANY)) {
      // The PTR query was for our service (usually
      // "<_service-name._tcp.local"), answer a PTR record reverse mapping the
      // queried service name to our service instance name (typically on the
      // "<hostname>.<_service-name>._tcp.local." format), and add additional
      // records containing the SRV record mapping the service instance name to
      // our qualified hostname (typically "<hostname>.local.") and port, as
      // well as any IPv4/IPv6 address for the hostname as A/AAAA records, and
      // two test TXT records

      // Answer PTR record reverse mapping "<_service-name>._tcp.local." to
      // "<hostname>.<_service-name>._tcp.local."
      mdns_record_t answer = service->record_ptr;

      mdns_record_t additional[5] = {0};
      size_t additional_count = 0;

      // SRV record mapping "<hostname>.<_service-name>._tcp.local." to
      // "<hostname>.local." with port. Set weight & priority to 0.
      additional[additional_count++] = service->record_srv;

      // A/AAAA records mapping "<hostname>.local." to IPv4/IPv6 addresses
      if (service->address_ipv4.sin_family == AF_INET) additional[additional_count++] = service->record_a;
      if (service->address_ipv6.sin6_family == AF_INET6) additional[additional_count++] = service->record_aaaa;

      // Add two test TXT records for our service instance name, will be
      // coalesced into one record with both key-value pair strings by the
      // library
      additional[additional_count++] = service->txt_record[0];
      additional[additional_count++] = service->txt_record[1];

      // Send the answer, unicast or multicast depending on flag in query
      uint16_t unicast = (rclass & MDNS_UNICAST_RESPONSE);
      printf("  --> answer %.*s (%s)\n", MDNS_STRING_FORMAT(service->record_ptr.data.ptr.name),
             (unicast ? "unicast" : "multicast"));

      if (unicast) {
        mdns_query_answer_unicast(sock, from, addrlen, sendbuffer, sizeof(sendbuffer), query_id, rtype, name.str,
                                  name.length, answer, 0, 0, additional, additional_count);
      } else {
        mdns_query_answer_multicast(sock, sendbuffer, sizeof(sendbuffer), answer, 0, 0, additional, additional_count);
      }
    }
  } else if ((name.length == service->service_instance.length) &&
             (strncmp(name.str, service->service_instance.str, name.length) == 0)) {
    if ((rtype == MDNS_RECORDTYPE_SRV) || (rtype == MDNS_RECORDTYPE_ANY)) {
      // The SRV query was for our service instance (usually
      // "<hostname>.<_service-name._tcp.local"), answer a SRV record mapping
      // the service instance name to our qualified hostname (typically
      // "<hostname>.local.") and port, as well as any IPv4/IPv6 address for the
      // hostname as A/AAAA records, and two test TXT records

      // Answer PTR record reverse mapping "<_service-name>._tcp.local." to
      // "<hostname>.<_service-name>._tcp.local."
      mdns_record_t answer = service->record_srv;

      mdns_record_t additional[5] = {0};
      size_t additional_count = 0;

      // A/AAAA records mapping "<hostname>.local." to IPv4/IPv6 addresses
      if (service->address_ipv4.sin_family == AF_INET) additional[additional_count++] = service->record_a;
      if (service->address_ipv6.sin6_family == AF_INET6) additional[additional_count++] = service->record_aaaa;

      // Add two test TXT records for our service instance name, will be
      // coalesced into one record with both key-value pair strings by the
      // library
      additional[additional_count++] = service->txt_record[0];
      additional[additional_count++] = service->txt_record[1];

      // Send the answer, unicast or multicast depending on flag in query
      uint16_t unicast = (rclass & MDNS_UNICAST_RESPONSE);
      printf("  --> answer %.*s port %d (%s)\n", MDNS_STRING_FORMAT(service->record_srv.data.srv.name), service->port,
             (unicast ? "unicast" : "multicast"));

      if (unicast) {
        mdns_query_answer_unicast(sock, from, addrlen, sendbuffer, sizeof(sendbuffer), query_id, rtype, name.str,
                                  name.length, answer, 0, 0, additional, additional_count);
      } else {
        mdns_query_answer_multicast(sock, sendbuffer, sizeof(sendbuffer), answer, 0, 0, additional, additional_count);
      }
    }
  } else if ((name.length == service->hostname_qualified.length) &&
             (strncmp(name.str, service->hostname_qualified.str, name.length) == 0)) {
    if (((rtype == MDNS_RECORDTYPE_A) || (rtype == MDNS_RECORDTYPE_ANY)) &&
        (service->address_ipv4.sin_family == AF_INET)) {
      // The A query was for our qualified hostname (typically
      // "<hostname>.local.") and we have an IPv4 address, answer with an A
      // record mappiing the hostname to an IPv4 address, as well as any IPv6
      // address for the hostname, and two test TXT records

      // Answer A records mapping "<hostname>.local." to IPv4 address
      mdns_record_t answer = service->record_a;

      mdns_record_t additional[5] = {0};
      size_t additional_count = 0;

      // AAAA record mapping "<hostname>.local." to IPv6 addresses
      if (service->address_ipv6.sin6_family == AF_INET6) additional[additional_count++] = service->record_aaaa;

      // Add two test TXT records for our service instance name, will be
      // coalesced into one record with both key-value pair strings by the
      // library
      additional[additional_count++] = service->txt_record[0];
      additional[additional_count++] = service->txt_record[1];

      // Send the answer, unicast or multicast depending on flag in query
      uint16_t unicast = (rclass & MDNS_UNICAST_RESPONSE);
      mdns_string_t addrstr =
          ip_address_to_string(addrbuffer, sizeof(addrbuffer), (struct sockaddr*)&service->record_a.data.a.addr,
                               sizeof(service->record_a.data.a.addr));
      printf("  --> answer %.*s IPv4 %.*s (%s)\n", MDNS_STRING_FORMAT(service->record_a.name),
             MDNS_STRING_FORMAT(addrstr), (unicast ? "unicast" : "multicast"));

      if (unicast) {
        mdns_query_answer_unicast(sock, from, addrlen, sendbuffer, sizeof(sendbuffer), query_id, rtype, name.str,
                                  name.length, answer, 0, 0, additional, additional_count);
      } else {
        mdns_query_answer_multicast(sock, sendbuffer, sizeof(sendbuffer), answer, 0, 0, additional, additional_count);
      }
    } else if (((rtype == MDNS_RECORDTYPE_AAAA) || (rtype == MDNS_RECORDTYPE_ANY)) &&
               (service->address_ipv6.sin6_family == AF_INET6)) {
      // The AAAA query was for our qualified hostname (typically
      // "<hostname>.local.") and we have an IPv6 address, answer with an AAAA
      // record mappiing the hostname to an IPv6 address, as well as any IPv4
      // address for the hostname, and two test TXT records

      // Answer AAAA records mapping "<hostname>.local." to IPv6 address
      mdns_record_t answer = service->record_aaaa;

      mdns_record_t additional[5] = {0};
      size_t additional_count = 0;

      // A record mapping "<hostname>.local." to IPv4 addresses
      if (service->address_ipv4.sin_family == AF_INET) additional[additional_count++] = service->record_a;

      // Add two test TXT records for our service instance name, will be
      // coalesced into one record with both key-value pair strings by the
      // library
      additional[additional_count++] = service->txt_record[0];
      additional[additional_count++] = service->txt_record[1];

      // Send the answer, unicast or multicast depending on flag in query
      uint16_t unicast = (rclass & MDNS_UNICAST_RESPONSE);
      mdns_string_t addrstr =
          ip_address_to_string(addrbuffer, sizeof(addrbuffer), (struct sockaddr*)&service->record_aaaa.data.aaaa.addr,
                               sizeof(service->record_aaaa.data.aaaa.addr));
      printf("  --> answer %.*s IPv6 %.*s (%s)\n", MDNS_STRING_FORMAT(service->record_aaaa.name),
             MDNS_STRING_FORMAT(addrstr), (unicast ? "unicast" : "multicast"));

      if (unicast) {
        mdns_query_answer_unicast(sock, from, addrlen, sendbuffer, sizeof(sendbuffer), query_id, rtype, name.str,
                                  name.length, answer, 0, 0, additional, additional_count);
      } else {
        mdns_query_answer_multicast(sock, sendbuffer, sizeof(sendbuffer), answer, 0, 0, additional, additional_count);
      }
    }
  }
  return 0;
}

// Callback handling questions and answers dump
static int dump_callback(int sock, const struct sockaddr* from, size_t addrlen, mdns_entry_type_t entry,
                         uint16_t query_id, uint16_t rtype, uint16_t rclass, uint32_t ttl, const void* data,
                         size_t size, size_t name_offset, size_t name_length, size_t record_offset,
                         size_t record_length, void* user_data) {
  mdns_string_t fromaddrstr = ip_address_to_string(addrbuffer, sizeof(addrbuffer), from, addrlen);

  size_t offset = name_offset;
  mdns_string_t name = mdns_string_extract(data, size, &offset, namebuffer, sizeof(namebuffer));

  const char* record_name = 0;
  if (rtype == MDNS_RECORDTYPE_PTR)
    record_name = "PTR";
  else if (rtype == MDNS_RECORDTYPE_SRV)
    record_name = "SRV";
  else if (rtype == MDNS_RECORDTYPE_A)
    record_name = "A";
  else if (rtype == MDNS_RECORDTYPE_AAAA)
    record_name = "AAAA";
  else if (rtype == MDNS_RECORDTYPE_TXT)
    record_name = "TXT";
  else if (rtype == MDNS_RECORDTYPE_ANY)
    record_name = "ANY";
  else
    record_name = "<UNKNOWN>";

  const char* entry_type = "Question";
  if (entry == MDNS_ENTRYTYPE_ANSWER)
    entry_type = "Answer";
  else if (entry == MDNS_ENTRYTYPE_AUTHORITY)
    entry_type = "Authority";
  else if (entry == MDNS_ENTRYTYPE_ADDITIONAL)
    entry_type = "Additional";

  printf("%.*s: %s %s %.*s rclass 0x%x ttl %u\n", MDNS_STRING_FORMAT(fromaddrstr), entry_type, record_name,
         MDNS_STRING_FORMAT(name), (unsigned int)rclass, ttl);

  return 0;
}

// Open sockets for sending one-shot multicast queries from an ephemeral port
static int open_client_sockets(int* sockets, int max_sockets, int port) {
  // When sending, each socket can only send to one network interface
  // Thus we need to open one socket for each interface and address family
  int num_sockets = 0;

  // inferface addresses
  struct ifaddrs* ifaddr = 0;
  struct ifaddrs* ifa = 0;
  if (getifaddrs(&ifaddr) < 0) printf("Unable to get interface addresses\n");

  int first_ipv4 = 1;
  int first_ipv6 = 1;
  for (ifa = ifaddr; ifa; ifa = ifa->ifa_next) {
    // ensure we have an address and it is up and multicast capable
    if (!ifa->ifa_addr) continue;
    if (!(ifa->ifa_flags & IFF_UP) || !(ifa->ifa_flags & IFF_MULTICAST)) continue;
    if ((ifa->ifa_flags & IFF_LOOPBACK) || (ifa->ifa_flags & IFF_POINTOPOINT)) continue;

    // check if its ipv4 or ipv6
    if (ifa->ifa_addr->sa_family == AF_INET) {
      struct sockaddr_in* saddr = (struct sockaddr_in*)ifa->ifa_addr;
      if (saddr->sin_addr.s_addr != htonl(INADDR_LOOPBACK)) {
        int log_addr = 0;
        if (first_ipv4) {
          service_address_ipv4 = *saddr;
          first_ipv4 = 0;
          log_addr = 1;
        }
        has_ipv4 = 1;
        if (num_sockets < max_sockets) {
          saddr->sin_port = htons(port);
          int sock = mdns_socket_open_ipv4(saddr);
          if (sock >= 0) {
            sockets[num_sockets++] = sock;
            log_addr = 1;
          } else {
            log_addr = 0;
          }
        }
        if (log_addr) {
          char buffer[128];
          mdns_string_t addr = ipv4_address_to_string(buffer, sizeof(buffer), saddr, sizeof(struct sockaddr_in));
          printf("Local IPv4 address: %.*s\n", MDNS_STRING_FORMAT(addr));
        }
      }
    } else if (ifa->ifa_addr->sa_family == AF_INET6) {
      struct sockaddr_in6* saddr = (struct sockaddr_in6*)ifa->ifa_addr;
      // Ignore link-local addresses
      if (saddr->sin6_scope_id) continue;
      static const unsigned char localhost[] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1};
      static const unsigned char localhost_mapped[] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0x7f, 0, 0, 1};
      if (memcmp(saddr->sin6_addr.s6_addr, localhost, 16) && memcmp(saddr->sin6_addr.s6_addr, localhost_mapped, 16)) {
        int log_addr = 0;
        if (first_ipv6) {
          service_address_ipv6 = *saddr;
          first_ipv6 = 0;
          log_addr = 1;
        }
        has_ipv6 = 1;
        if (num_sockets < max_sockets) {
          saddr->sin6_port = htons(port);
          int sock = mdns_socket_open_ipv6(saddr);
          if (sock >= 0) {
            sockets[num_sockets++] = sock;
            log_addr = 1;
          } else {
            log_addr = 0;
          }
        }
        if (log_addr) {
          char buffer[128];
          mdns_string_t addr = ipv6_address_to_string(buffer, sizeof(buffer), saddr, sizeof(struct sockaddr_in6));
          printf("Local IPv6 address: %.*s\n", MDNS_STRING_FORMAT(addr));
        }
      }
    }
  }

  freeifaddrs(ifaddr);

  return num_sockets;
}

/** Open sockets to listen to incoming mDNS queries on port 5353. 
 * 
 * @param sockets The array to store the opened sockets.
 * @param max_sockets The maximum number of sockets to open.
 * @return The number of opened sockets.
 */
static int open_service_sockets(int* sockets, int max_sockets) {
  // When recieving, each socket can recieve data from all network interfaces
  // Thus we only need to open one socket for each address family
  int num_sockets = 0;

  // Call the client socket function to enumerate and get local addresses,
  // but not open the actual sockets
  open_client_sockets(0, 0, 0);

  if (num_sockets < max_sockets) {
    struct sockaddr_in sock_addr;
    memset(&sock_addr, 0, sizeof(struct sockaddr_in));
    sock_addr.sin_family = AF_INET;
    sock_addr.sin_addr.s_addr = INADDR_ANY;
    sock_addr.sin_port = htons(MDNS_PORT);
#ifdef __APPLE__
    sock_addr.sin_len = sizeof(struct sockaddr_in);
#endif
    int sock = mdns_socket_open_ipv4(&sock_addr);
    if (sock >= 0) sockets[num_sockets++] = sock;
  }

  if (num_sockets < max_sockets) {
    struct sockaddr_in6 sock_addr;
    memset(&sock_addr, 0, sizeof(struct sockaddr_in6));
    sock_addr.sin6_family = AF_INET6;
    sock_addr.sin6_addr = in6addr_any;
    sock_addr.sin6_port = htons(MDNS_PORT);
#ifdef __APPLE__
    sock_addr.sin6_len = sizeof(struct sockaddr_in6);
#endif
    int sock = mdns_socket_open_ipv6(&sock_addr);
    if (sock >= 0) sockets[num_sockets++] = sock;
  }

  return num_sockets;
}

// Send a DNS-SD query
static int send_dns_sd(void) {
  int sockets[32];
  int num_sockets = open_client_sockets(sockets, sizeof(sockets) / sizeof(sockets[0]), 0);
  if (num_sockets <= 0) {
    printf("Failed to open any client sockets\n");
    return -1;
  }
  printf("Opened %d socket%s for DNS-SD\n", num_sockets, num_sockets > 1 ? "s" : "");

  printf("Sending DNS-SD discovery\n");
  for (int isock = 0; isock < num_sockets; ++isock) {
    if (mdns_discovery_send(sockets[isock])) printf("Failed to send DNS-DS discovery: %s\n", strerror(errno));
  }

  size_t capacity = 2048;
  void* buffer = malloc(capacity);
  void* user_data = 0;
  size_t records;

  // This is a simple implementation that loops for 5 seconds or as long as we
  // get replies
  int res;
  printf("Reading DNS-SD replies\n");
  do {
    struct timeval timeout;
    timeout.tv_sec = 5;
    timeout.tv_usec = 0;

    int nfds = 0;
    fd_set readfs;
    FD_ZERO(&readfs);
    for (int isock = 0; isock < num_sockets; ++isock) {
      if (sockets[isock] >= nfds) nfds = sockets[isock] + 1;
      FD_SET(sockets[isock], &readfs);
    }

    records = 0;
    res = select(nfds, &readfs, 0, 0, &timeout);
    if (res > 0) {
      for (int isock = 0; isock < num_sockets; ++isock) {
        if (FD_ISSET(sockets[isock], &readfs)) {
          records += mdns_discovery_recv(sockets[isock], buffer, capacity, query_callback, user_data);
        }
      }
    }
  } while (res > 0);

  free(buffer);

  for (int isock = 0; isock < num_sockets; ++isock) mdns_socket_close(sockets[isock]);
  printf("Closed socket%s\n", num_sockets > 1 ? "s" : "");

  return 0;
}

// Send a mDNS query
static int send_mdns_query(mdns_query_t* query, size_t count) {
  int sockets[32];
  int query_id[32];

  // create client sockets for each query
  int num_sockets = open_client_sockets(sockets, sizeof(sockets) / sizeof(sockets[0]), 0);
  if (num_sockets <= 0) {
    printf("Failed to open any client sockets\n");
    return -1;
  }
  printf("Opened %d socket%s for mDNS query\n", num_sockets, num_sockets > 1 ? "s" : "");

  size_t capacity = 2048;
  void* buffer = malloc(capacity);
  void* user_data = 0;

  printf("Sending mDNS query");
  for (size_t iq = 0; iq < count; ++iq) {
    const char* record_name = "PTR";
    if (query[iq].type == MDNS_RECORDTYPE_SRV)
      record_name = "SRV";
    else if (query[iq].type == MDNS_RECORDTYPE_A)
      record_name = "A";
    else if (query[iq].type == MDNS_RECORDTYPE_AAAA)
      record_name = "AAAA";
    else
      query[iq].type = MDNS_RECORDTYPE_PTR;
    printf(" : %s %s", query[iq].name, record_name);
  }
  printf("\n");

  // send queries (one for each socket)
  for (int i = 0; i < num_sockets; ++i) {
    query_id[i] = mdns_multiquery_send(sockets[i], query, count, buffer, capacity, 0);

    if (query_id[i] < 0) {
      perror("Failed to send mDNS query");
    }
  }

  // This is a simple implementation that loops for 5 seconds or as long as we
  // get replies
  int res;
  printf("Reading mDNS query replies\n");
  int records = 0;
  do {
    // 10 seconds timeout
    struct timeval timeout;
    timeout.tv_sec = 10;
    timeout.tv_usec = 0;

    // the number of file descriptors to be checked,
    // note that this must be equal to `largest_fd + 1`
    // (see why at `man signal`)
    int nfds = 0;

    // file descriptor set
    fd_set readfs;

    // reset the fd set
    FD_ZERO(&readfs);

    // add sockets to the set
    for (int i = 0; i < num_sockets; ++i) {
      if (sockets[i] >= nfds) {
        nfds = sockets[i] + 1;
      }
      FD_SET(sockets[i], &readfs);
    }

    // wait on all sockets, this will mutate `readfs` to contain
    // the fd's that are ready
    res = select(nfds, &readfs, 0, 0, &timeout);
    if (res > 0) {
      for (int i = 0; i < num_sockets; ++i) {
        // check if the socket is ready (i.e. included in `readfs`)
        if (FD_ISSET(sockets[i], &readfs)) {
          // callback will handle the result
          size_t rec = mdns_query_recv(sockets[i], buffer, capacity, query_callback, user_data, query_id[i]);
          if (rec > 0) records += rec;
        }

        // add the socket in any case for next attempts
        FD_SET(sockets[i], &readfs);
      }
    }
  } while (res > 0);

  printf("Read %d records\n", records);

  free(buffer);

  // close each socket
  for (int i = 0; i < num_sockets; ++i) {
    mdns_socket_close(sockets[i]);
  }
  printf("Closed %d socket%s\n", num_sockets, num_sockets > 1 ? "s" : "");

  return 0;
}

// Provide a mDNS service, answering incoming DNS-SD and mDNS queries
static int service_mdns(const char* hostname, const char* service_name, int service_port) {
  // open service sockets
  int sockets[32];
  int num_sockets = open_service_sockets(sockets, sizeof(sockets) / sizeof(sockets[0]));
  if (num_sockets <= 0) {
    printf("Failed to open any service sockets\n");
    return -1;
  }
  printf("Opened %d socket%s for mDNS service\n", num_sockets, num_sockets > 1 ? "s" : "");

  size_t service_name_length = strlen(service_name);
  if (!service_name_length) {
    printf("Invalid service name\n");
    return -1;
  }

  // Append a '.' to the service name if it does not end with one
  char* service_name_buffer = malloc(service_name_length + 2);
  memcpy(service_name_buffer, service_name, service_name_length);
  if (service_name_buffer[service_name_length - 1] != '.') service_name_buffer[service_name_length++] = '.';
  service_name_buffer[service_name_length] = 0;  // null-terminate
  service_name = service_name_buffer;

  printf("Service mDNS: %s:%d\n", service_name, service_port);
  printf("Hostname: %s\n", hostname);

  size_t capacity = 2048;
  void* buffer = malloc(capacity);

  mdns_string_t service_string = (mdns_string_t){service_name, strlen(service_name)};
  mdns_string_t hostname_string = (mdns_string_t){hostname, strlen(hostname)};

  // build the service instance "<hostname>.<_service-name>._tcp.local." string
  char service_instance_buffer[256] = {0};
  snprintf(service_instance_buffer, sizeof(service_instance_buffer) - 1, "%.*s.%.*s",
           MDNS_STRING_FORMAT(hostname_string), MDNS_STRING_FORMAT(service_string));
  mdns_string_t service_instance_string = (mdns_string_t){service_instance_buffer, strlen(service_instance_buffer)};

  // build the "<hostname>.local." string
  char qualified_hostname_buffer[256] = {0};
  snprintf(qualified_hostname_buffer, sizeof(qualified_hostname_buffer) - 1, "%.*s.local.",
           MDNS_STRING_FORMAT(hostname_string));
  mdns_string_t hostname_qualified_string =
      (mdns_string_t){qualified_hostname_buffer, strlen(qualified_hostname_buffer)};

  service_t service = {0};
  service.service = service_string;
  service.hostname = hostname_string;
  service.service_instance = service_instance_string;
  service.hostname_qualified = hostname_qualified_string;
  service.address_ipv4 = service_address_ipv4;
  service.address_ipv6 = service_address_ipv6;
  service.port = service_port;

  // Setup our mDNS records

  // PTR record reverse mapping "<_service-name>._tcp.local." to
  // "<hostname>.<_service-name>._tcp.local."
  service.record_ptr = (mdns_record_t){.name = service.service,
                                       .type = MDNS_RECORDTYPE_PTR,
                                       .data.ptr.name = service.service_instance,
                                       .rclass = 0,
                                       .ttl = 0};

  // SRV record mapping "<hostname>.<_service-name>._tcp.local." to
  // "<hostname>.local." with port. Set weight & priority to 0.
  service.record_srv = (mdns_record_t){.name = service.service_instance,
                                       .type = MDNS_RECORDTYPE_SRV,
                                       .data.srv.name = service.hostname_qualified,
                                       .data.srv.port = service.port,
                                       .data.srv.priority = 0,
                                       .data.srv.weight = 0,
                                       .rclass = 0,
                                       .ttl = 0};

  // A/AAAA records mapping "<hostname>.local." to IPv4/IPv6 addresses
  service.record_a = (mdns_record_t){.name = service.hostname_qualified,
                                     .type = MDNS_RECORDTYPE_A,
                                     .data.a.addr = service.address_ipv4,
                                     .rclass = 0,
                                     .ttl = 0};
  service.record_aaaa = (mdns_record_t){.name = service.hostname_qualified,
                                        .type = MDNS_RECORDTYPE_AAAA,
                                        .data.aaaa.addr = service.address_ipv6,
                                        .rclass = 0,
                                        .ttl = 0};

  // Add two test TXT records for our service instance name, will be coalesced
  // into one record with both key-value pair strings by the library
  // TODO: this is where we publish our device capabilities
  service.txt_record[0] = (mdns_record_t){.name = service.service_instance,
                                          .type = MDNS_RECORDTYPE_TXT,
                                          .data.txt.key = {MDNS_STRING_CONST("test")},
                                          .data.txt.value = {MDNS_STRING_CONST("1")},
                                          .rclass = 0,
                                          .ttl = 0};
  service.txt_record[1] = (mdns_record_t){.name = service.service_instance,
                                          .type = MDNS_RECORDTYPE_TXT,
                                          .data.txt.key = {MDNS_STRING_CONST("other")},
                                          .data.txt.value = {MDNS_STRING_CONST("value")},
                                          .rclass = 0,
                                          .ttl = 0};

  // Send an announcement on startup of service
  {
    printf("Sending announce\n");
    mdns_record_t additional[5] = {0};
    size_t additional_count = 0;
    additional[additional_count++] = service.record_srv;
    if (service.address_ipv4.sin_family == AF_INET) additional[additional_count++] = service.record_a;
    if (service.address_ipv6.sin6_family == AF_INET6) additional[additional_count++] = service.record_aaaa;
    additional[additional_count++] = service.txt_record[0];
    additional[additional_count++] = service.txt_record[1];

    for (int i = 0; i < num_sockets; ++i)
      mdns_announce_multicast(sockets[i], buffer, capacity, service.record_ptr, 0, 0, additional, additional_count);
  }

  // This is a crude implementation that checks for incoming queries
  while (running) {
    int nfds = 0;
    fd_set readfs;
    FD_ZERO(&readfs);
    for (int isock = 0; isock < num_sockets; ++isock) {
      if (sockets[isock] >= nfds) nfds = sockets[isock] + 1;
      FD_SET(sockets[isock], &readfs);
    }

    // wait 100ms for each `select`
    struct timeval timeout;
    timeout.tv_sec = 0;
    timeout.tv_usec = 100000;

    if (select(nfds, &readfs, 0, 0, &timeout) >= 0) {
      for (int isock = 0; isock < num_sockets; ++isock) {
        if (FD_ISSET(sockets[isock], &readfs)) {
          mdns_socket_listen(sockets[isock], buffer, capacity, service_callback, &service);
        }
        FD_SET(sockets[isock], &readfs);
      }
    } else {
      break;
    }
  }

  // Send a goodbye on end of service
  {
    printf("Sending goodbye\n");
    mdns_record_t additional[5] = {0};
    size_t additional_count = 0;
    additional[additional_count++] = service.record_srv;
    if (service.address_ipv4.sin_family == AF_INET) additional[additional_count++] = service.record_a;
    if (service.address_ipv6.sin6_family == AF_INET6) additional[additional_count++] = service.record_aaaa;
    additional[additional_count++] = service.txt_record[0];
    additional[additional_count++] = service.txt_record[1];

    for (int isock = 0; isock < num_sockets; ++isock)
      mdns_goodbye_multicast(sockets[isock], buffer, capacity, service.record_ptr, 0, 0, additional, additional_count);
  }

  free(buffer);
  free(service_name_buffer);

  // close sockets
  for (int i = 0; i < num_sockets; ++i) {
    mdns_socket_close(sockets[i]);
  }
  printf("Closed socket%s\n", num_sockets > 1 ? "s" : "");

  return 0;
}

// Dump all incoming mDNS queries and answers
static int dump_mdns(void) {
  int sockets[32];
  int num_sockets = open_service_sockets(sockets, sizeof(sockets) / sizeof(sockets[0]));
  if (num_sockets <= 0) {
    printf("Failed to open any client sockets\n");
    return -1;
  }
  printf("Opened %d socket%s for mDNS dump\n", num_sockets, num_sockets > 1 ? "s" : "");

  size_t capacity = 2048;
  void* buffer = malloc(capacity);

  // This is a crude implementation that checks for incoming queries and answers
  while (running) {
    int nfds = 0;
    fd_set readfs;
    FD_ZERO(&readfs);
    for (int isock = 0; isock < num_sockets; ++isock) {
      if (sockets[isock] >= nfds) nfds = sockets[isock] + 1;
      FD_SET(sockets[isock], &readfs);
    }

    struct timeval timeout;
    timeout.tv_sec = 0;
    timeout.tv_usec = 100000;

    if (select(nfds, &readfs, 0, 0, &timeout) >= 0) {
      for (int isock = 0; isock < num_sockets; ++isock) {
        if (FD_ISSET(sockets[isock], &readfs)) {
          mdns_socket_listen(sockets[isock], buffer, capacity, dump_callback, 0);
        }
        FD_SET(sockets[isock], &readfs);
      }
    } else {
      break;
    }
  }

  free(buffer);

  // close sockets
  for (int isock = 0; isock < num_sockets; ++isock) {
    mdns_socket_close(sockets[isock]);
  }
  printf("Closed socket%s\n", num_sockets > 1 ? "s" : "");

  return 0;
}

/** Signal handler to gracefully stop the service. */
void signal_handler(int signal) {
  (void)signal;
  running = 0;
}

enum Mode {
  DISCOVERY_MODE = 0,
  QUERY_MODE = 1,
  SERVICE_MODE = 2,
  DUMP_MODE = 3,
  // this is our target!
  DAEMON_MODE = 4,
};

int mdns_main(int argc, const char* const* argv) {
  // port is `41891 = 4 18 9 1 = D R I A`
  int service_port = 41891;
  // service name
  const char* service_name = "_dllmd._tcp.local.";

  // queries for `--query` option
  const int MAX_QUERY_COUNT = 16;
  mdns_query_t query[MAX_QUERY_COUNT];
  size_t query_count = 0;

  // get hostname from the system (if available)
  char hostname_buffer[256];
  const char* hostname = "dummy-host";
  size_t hostname_size = sizeof(hostname_buffer);
  if (gethostname(hostname_buffer, hostname_size) == 0) {
    hostname = hostname_buffer;
  } else {
    perror("gethostname failed");
    return -1;
  }

  // add a signal handler to catch Ctrl-C and terminate the service
  signal(SIGINT, signal_handler);

  // choosen mode w.r.t args, defaults to Discover mode.
  enum Mode mode = DISCOVERY_MODE;
  for (int iarg = 0; iarg < argc; ++iarg) {
    if (strcmp(argv[iarg], "--discovery") == 0) {
      mode = DISCOVERY_MODE;
    } else if (strcmp(argv[iarg], "--daemon") == 0) {
      mode = DAEMON_MODE;
    } else if (strcmp(argv[iarg], "--query") == 0) {
      // Each query is either a service name, or a pair of record type and a
      // service name, e.g.:
      //   mdns --query _foo._tcp.local.
      //   mdns --query SRV myhost._foo._tcp.local.
      //   mdns --query A myhost._tcp.local. _service._tcp.local.
      mode = QUERY_MODE;
      ++iarg;
      while ((iarg < argc) && (query_count < MAX_QUERY_COUNT)) {
        query[query_count].name = argv[iarg++];
        query[query_count].type = MDNS_RECORDTYPE_PTR;
        if (iarg < argc) {
          mdns_record_type_t record_type = 0;
          if (strcmp(query[query_count].name, "PTR") == 0)
            record_type = MDNS_RECORDTYPE_PTR;
          else if (strcmp(query[query_count].name, "SRV") == 0)
            record_type = MDNS_RECORDTYPE_SRV;
          else if (strcmp(query[query_count].name, "A") == 0)
            record_type = MDNS_RECORDTYPE_A;
          else if (strcmp(query[query_count].name, "AAAA") == 0)
            record_type = MDNS_RECORDTYPE_AAAA;

          if (record_type != 0) {
            query[query_count].type = record_type;
            query[query_count].name = argv[iarg++];
          }
        }
        query[query_count].length = strlen(query[query_count].name);
        ++query_count;
      }
    } else if (strcmp(argv[iarg], "--service") == 0) {
      mode = SERVICE_MODE;
      // get optional service name
      if (++iarg < argc) service_name = argv[iarg];
    } else if (strcmp(argv[iarg], "--dump") == 0) {
      mode = DUMP_MODE;
    } else if (strcmp(argv[iarg], "--hostname") == 0) {
      if (++iarg < argc) {
        hostname = argv[iarg];
      } else {
        fprintf(stderr, "Hostname not provided\n");
        return -1;
      }
    } else if (strcmp(argv[iarg], "--port") == 0) {
      if (++iarg < argc) {
        service_port = atoi(argv[iarg]);
      } else {
        fprintf(stderr, "Port not provided\n");
        return -1;
      }
    }
  }

  int ret = 0;
  switch (mode) {
    case DISCOVERY_MODE:
      ret = send_dns_sd();
      break;
    case QUERY_MODE:
      ret = send_mdns_query(query, query_count);
      break;
    case SERVICE_MODE:
      ret = service_mdns(hostname, service_name, service_port);
      break;
    case DUMP_MODE:
      ret = dump_mdns();
      break;
    default:
      fprintf(stderr, "Invalid mode: %d\n", mode);
      ret = -1;
      break;
  }
  return ret;
}