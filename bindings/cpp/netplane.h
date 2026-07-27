#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct NetplaneHandshakeResult {
  char *netmask;
  char *destination;
  char *ip_addr;
} NetplaneHandshakeResult;

/**
 * Log format selector for the C FFI.
 * 0 = Pretty (default), 1 = Json, 2 = Logfmt. Unknown values fall back to Pretty.
 */
 void netplane_client_init_logger(uint32_t format) ;

/**
 * # Safety
 *
 * All pointer arguments must be valid, non-null, NUL-terminated C strings that
 * remain valid for the duration of the call.
 */

int32_t netplane_client_auth(const char *authkey_path,
                             const char *publickey_path,
                             const char *privatekey_path,
                             const char *host,
                             const char *link_code,
                             uint16_t auth_port)
;

/**
 * # Safety
 *
 * `ptr` must be a pointer previously returned by this library and not yet
 * freed; passing any other value is undefined behavior.
 */
 void netplane_client_free_auth(char *ptr) ;

/**
 * # Safety
 *
 * All pointer arguments must be valid, non-null, NUL-terminated C strings that
 * remain valid for the duration of the call.
 */

int32_t netplane_try_generate_crypto_keys(const char *public_filepath,
                                          const char *private_filepath)
;


int32_t netplane_client_handshake(const char *authkey_path,
                                  const char *public_filepath,
                                  const char *private_filepath,
                                  void *transport,
                                  struct NetplaneHandshakeResult *result)
;

/**
 * # Safety
 *
 * `result` must be a pointer previously returned by this library and not yet
 * freed; passing any other value is undefined behavior.
 */
 void netplane_client_free_handshake(struct NetplaneHandshakeResult *result) ;

/**
 * # Safety
 *
 * `server_addr` and `transport_type` must be valid, non-null, NUL-terminated C
 * strings that remain valid for the duration of the call.
 */

void *netplane_create_transport(const char *server_addr,
                                uint16_t server_port,
                                const char *transport_type)
;

 void netplane_free_transport(void *transport) ;

/**
 * # Safety
 *
 * `transport` and `handshake` must be valid pointers previously returned by
 * this library, and the string pointers must be valid, non-null,
 * NUL-terminated C strings that remain valid for the duration of the call.
 */

int32_t netplane_client_run_fd(int tun_fd,
                               void *transport,
                               struct NetplaneHandshakeResult *handshake,
                               bool loopback_relay,
                               bool no_encryption,
                               const char *public_filepath,
                               const char *private_filepath)
;

 void netplane_client_stop(void) ;

/**
 * # Safety
 *
 * The string pointers must be valid, non-null, NUL-terminated C strings that
 * remain valid for the duration of the call, and `cancel_token_out`, if
 * non-null, must point to writable storage for one pointer.
 */

int32_t netplane_client_run(const char *tun_dev,
                            const char *host,
                            uint16_t port,
                            const char *transport_type,
                            bool loopback_relay,
                            bool no_encryption,
                            const char *authkey_path,
                            const char *public_filepath,
                            const char *private_filepath,
                            void **cancel_token_out)
;
