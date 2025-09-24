#ifndef NETPLANE_H
#define NETPLANE_H

#ifdef __cplusplus
extern "C" {
#endif

void netplane_init_logger();
int netplane_try_generate_crypto_keys(const char *public_filepath,
                                      const char *private_filepath);
int netplane_client_auth(const char *host, const char *link_code,
                         unsigned short auth_port, char **auth_key);
void netplane_client_free_auth(char *auth_key);
int netplane_client_run(const char *tun_dev, const char *host,
                        unsigned short port, const char *transport_type);

#ifdef __cplusplus
}
#endif

#endif // NETPLANE_H
