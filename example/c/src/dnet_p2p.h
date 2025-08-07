#ifndef DNET_P2P_H
#define DNET_P2P_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef struct dnet_p2p_handle dnet_p2p_handle_t;
typedef struct dnet_p2p dnet_p2p_t;

/**
 * @brief Enables logging for dnet, respecting `RUST_LOG` environment variable.
 */
extern void dnet_p2p_enable_logs(void);

/**
 * @brief Create a new dnet instance
 * @param instance_name name of the dnet instance, used for logging
 * @param hostname hostname to bind to, e.g. from `gethostname()`
 * @param address service address (can be empty string)
 * @param is_manager if true, the instance will run in manager mode, otherwise
 * in worker mode
 * @param is_passive if true, the instance will only monitor (not register to mDNS)
 * @return `dnet_p2p_t*` pointer to the `dnet` instance
 */
extern dnet_p2p_t *dnet_p2p_new(const char *instance_name, const char *hostname,
                                const char *address, const int is_manager, 
                                const int is_passive);

/**
 * @brief Free the dnet instance
 * @param service_ptr pointer to the `dnet` instance
 */
extern void dnet_p2p_free(dnet_p2p_t *service_ptr);

/**
 * @brief Start listening on the given address
 * @param service_ptr pointer to the dnet instance
 * @return `dnet_handle_t*` handle for the thread that runs `dnet`
 */
extern dnet_p2p_handle_t *dnet_p2p_start(dnet_p2p_t *service_ptr);

/**
 * @brief Stop the `dnet` service
 * @param service_ptr pointer to the `dnet` service instance
 * @param handle_ptr handle for the thread that is running `dnet` service
 * @return 0 on success, -1 on error
 */
extern int dnet_p2p_stop(dnet_p2p_t *service_ptr,
                         dnet_p2p_handle_t *handle_ptr);

/**
 * @brief Get the properties of the `dnet` service
 *
 * @param service_ptr pointer to the `dnet` service instance
 * @param buf buffer to write the properties to
 * @param buf_size size of the buffer
 * @return 0 on success, -1 on error
 */
extern int dnet_p2p_get_properties(dnet_p2p_t *service_ptr, void *buf,
                                   size_t buf_size);

/**
 * @brief Set the busy status of the service
 * @param service_ptr pointer to the `dnet` service instance
 * @param is_busy true if the service is busy, false otherwise
 */
extern void dnet_p2p_set_is_busy(dnet_p2p_t *service_ptr, bool is_busy);

#endif // DNET_P2P_H
