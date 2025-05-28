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
 * @param is_manager if true, the instance will run in manager mode, otherwise
 * in worker mode
 * @return `dnet_p2p_t*` pointer to the `dnet` instance
 */
extern dnet_p2p_t *dnet_p2p_new(const char *instance_name, const char *hostname,
                                const int is_manager);

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
 */
extern int dnet_p2p_stop(dnet_p2p_t *service_ptr,
                         dnet_p2p_handle_t *handle_ptr);

#endif // DNET_P2P_H
