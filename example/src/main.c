#include <stdio.h>
#include <stdlib.h>

extern char *echo(const char *input);

int main() {
  char *result = echo("Hello from C!");
  printf("%s\n", result);

  free(result);
  return 0;
}
