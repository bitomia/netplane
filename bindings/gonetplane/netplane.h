#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct NetplaneHandshakeResult {
  char *netmask;
  char *destination;
  char *ip_addr;
} NetplaneHandshakeResult;

 void netplane_init_logger(uint32_t format) ;


int32_t netplane_client_auth(const char *authkey_path,
                             const char *publickey_path,
                             const char *privatekey_path,
                             const char *host,
                             const char *link_code,
                             uint16_t auth_port)
;

 void netplane_client_free_auth(char *ptr) ;


int32_t netplane_try_generate_crypto_keys(const char *public_filepath,
                                          const char *private_filepath)
;


int32_t netplane_client_handshake(const char *authkey_path,
                                  const char *public_filepath,
                                  const char *private_filepath,
                                  void *transport,
                                  struct NetplaneHandshakeResult *result)
;

 void netplane_client_free_handshake(struct NetplaneHandshakeResult *result) ;


void *netplane_create_transport(const char *server_addr,
                                uint16_t server_port,
                                const char *transport_type)
;

 void netplane_free_transport(void *transport) ;


int32_t netplane_client_run(int tun_fd,
                            void *transport,
                            struct NetplaneHandshakeResult *handshake,
                            bool loopback_relay,
                            bool no_encryption,
                            const char *public_filepath,
                            const char *private_filepath)
;

 void netplane_client_stop(void) ;


int32_t netplane_run(const char *tun_dev,
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

 void netplane_cancel(void *token) ;

 void netplane_free_cancel_token(void *token) ;
