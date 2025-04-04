#ifndef DLLMD_H
#define DLLMD_H

#include "stdbool.h"

typedef struct dllmd dllmd_t;

extern dllmd_t *dllmd_new(void);
extern void dllmd_free(dllmd_t *ptr);

extern void dllmd_start_daemon(dllmd_t *ptr, const char *addr);
extern bool dllmd_is_running(dllmd_t *ptr);
extern void dllmd_shutdown(dllmd_t *ptr);

#endif // DLLMD_H
