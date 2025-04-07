#ifndef DLLMD_H
#define DLLMD_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef struct dllmd_handle dllmd_handle_t;
typedef struct dllmd dllmd_t;

#define DLLMD_NO_TIMEOUT 0

/**
 * @brief Create a new dllmd instance
 * @return dllmd_t* pointer to the dllmd instance
 */
extern dllmd_t *dllmd_new(void);

/**
 * @brief Free the dllmd instance
 * @param ptr pointer to the dllmd instance
 */
extern void dllmd_free(dllmd_t *ptr);

/**
 * @brief Start listening on the given address
 * @param ptr pointer to the dllmd instance
 * @param addr address to listen on
 * @return dllmd_handle_t* handle for the thread that runs DLLM
 */
extern dllmd_handle_t *dllmd_start(dllmd_t *ptr, const char *addr);

/**
 * @brief Stop the dllmd instance
 * @param ptr pointer to the dllmd instance
 * @param handle_ptr handle for the thread that runs DLLM
 */
extern void dllmd_stop(dllmd_t *ptr, dllmd_handle_t *handle_ptr);

/**
 * @brief Publishes a message to all connected peers
 * @param ptr pointer to the dllmd instance
 * @param data raw bytes
 * @param data_len length of `data`
 * @return non-zero if error
 */
extern int dllmd_publish(dllmd_t *ptr, const char *data, size_t data_len);

/**
 * @brief Receives a message that has been sent to this peer.
 * @param ptr pointer to the dllmd instance
 * @param buf buffer to store the message
 * @param buf_size size of the buffer
 * @param timeout_ms timeout in milliseconds, or 0 for no timeout
 * @return int number of bytes received, or -1 on error
 */
extern int dllmd_receive(dllmd_t *ptr, void *buf, size_t buf_size,
                         uint64_t timeout_ms);

#endif // DLLMD_H
