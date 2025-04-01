#include <stdio.h>
#include <stdlib.h>

// Declare the Rust function
extern char *echo(const char *input);

int main() {
  char *result = echo("Hello from C!");
  printf("Echoed: %s\n", result);

  free(result);
  // Normally you'd free or otherwise manage Rust-allocated strings here
  return 0;
}
