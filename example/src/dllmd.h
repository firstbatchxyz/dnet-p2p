#ifndef DLLMD_H
#define DLLMD_H

#include "stdbool.h"

typedef struct dllmd_handle dllmd_handle_t;
typedef struct dllmd dllmd_t;

extern dllmd_t *dllmd_new(void);
extern void dllmd_free(dllmd_t *ptr);

extern dllmd_handle_t *dllmd_start(dllmd_t *ptr, const char *addr);
extern void dllmd_stop(dllmd_t *ptr, dllmd_handle_t *handle_ptr);

#endif // DLLMD_H
