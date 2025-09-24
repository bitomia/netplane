#include <stdio.h>

#include "netplane.h"

int main(int argc, char **argv) {
  netplane_init_logger();
  netplane_try_generate_crypto_keys("public.key", "private.key");

  if (argc != 1) {
    char *auth_key = 0;
    int retval = netplane_client_auth("localhost", argv[1], 8000, &auth_key);
    if (retval != 0) {
      return retval;
    }
  }

  return netplane_client_run("tun0", "localhost", 5050, "udp");
}
