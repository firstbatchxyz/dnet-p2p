#ifndef DNET_H
#define DNET_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef struct dnet_handle dnet_handle_t;
typedef struct dnet dnet_t;

/**
 * @brief Create a new dnet instance
 * @return dnet_t* pointer to the dnet instance
 */
extern dnet_t *dnet_new(void);

/**
 * @brief Free the dnet instance
 * @param ptr pointer to the dnet instance
 */
extern void dnet_free(dnet_t *ptr);

/**
 * @brief Start listening on the given address
 * @param ptr pointer to the dnet instance
 * @param addr address to listen on
 * @return dnet_handle_t* handle for the thread that runs `dnet`
 */
extern dnet_handle_t *dnet_start(dnet_t *ptr, const char *addr);

/**
 * @brief Stop the dnet instance
 * @param ptr pointer to the dnet instance
 * @param handle_ptr handle for the thread that runs `dnet`
 */
extern void dnet_stop(dnet_t *ptr, dnet_handle_t *handle_ptr);

#define DNET_PUBLISH_ERR_INSUFFICIENT_PEERS -1
#define DNET_PUBLISH_ERR_MSG_TOO_LARGE -2
#define DNET_PUBLISH_ERR_UNHANDLED -3
/**
 * @brief Publishes a message to all connected peers
 * @param ptr pointer to the dnet instance
 * @param data raw bytes
 * @param data_len length of `data`
 * @return non-zero if error
 */
extern int dnet_publish(dnet_t *ptr, const void *data, size_t data_len);

#define DNET_RECEIVE_NO_TIMEOUT 0
/**
 * @brief Receives a message that has been sent to this peer.
 * @param ptr pointer to the dnet instance
 * @param buf buffer to store the message
 * @param buf_size size of the buffer
 * @param timeout_ms timeout in milliseconds, or 0 for no timeout
 * @return int number of bytes received, or -1 on error
 */
extern int dnet_receive(dnet_t *ptr, void *buf, size_t buf_size,
                        uint64_t timeout_ms);

#endif // DNET_H
