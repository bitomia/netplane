#include <cassert>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <string>

extern "C" {
#include "netplane.h"
}

namespace fs = std::filesystem;

static int failures = 0;

#define EXPECT(cond)                                                           \
    do {                                                                       \
        if (!(cond)) {                                                         \
            std::fprintf(stderr, "FAIL %s:%d  %s\n", __FILE__, __LINE__, #cond); \
            ++failures;                                                        \
        }                                                                      \
    } while (0)

static void test_init_logger() {
    netplane_init_logger(0);
}

static void test_generate_crypto_keys() {
    fs::path dir = fs::temp_directory_path() / "netplane_cpp_test";
    fs::create_directories(dir);
    fs::path pub_path = dir / "public.key";
    fs::path priv_path = dir / "private.key";
    fs::remove(pub_path);
    fs::remove(priv_path);

    int32_t rc = netplane_try_generate_crypto_keys(
        pub_path.c_str(), priv_path.c_str());
    EXPECT(rc == 0);
    EXPECT(fs::exists(pub_path));
    EXPECT(fs::exists(priv_path));
    EXPECT(fs::file_size(pub_path) > 0);
    EXPECT(fs::file_size(priv_path) > 0);

    fs::remove_all(dir);
}

static void test_transport_lifecycle() {
    void *t = netplane_create_transport("127.0.0.1", 51820, "udp");
    EXPECT(t != nullptr);
    if (t != nullptr) {
        netplane_free_transport(t);
    }
}

static void test_transport_invalid_type() {
    void *t = netplane_create_transport("127.0.0.1", 51820, "not-a-transport");
    EXPECT(t == nullptr);
    if (t != nullptr) {
        netplane_free_transport(t);
    }
}

int main() {
    test_init_logger();
    test_generate_crypto_keys();
    test_transport_lifecycle();
    test_transport_invalid_type();

    if (failures == 0) {
        std::printf("OK: all netplane C++ binding tests passed\n");
        return 0;
    }
    std::fprintf(stderr, "FAILED: %d assertion(s)\n", failures);
    return 1;
}
