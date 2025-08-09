UNAME_S := $(shell uname -s)

LIB_NAME = netplane_ffi
RUST_LIB_DIR = ../../target/release
RUST_LIB_DEBUG_DIR = ../../target/debug

ifeq ($(UNAME_S),Linux)
    LIB_EXT = so
    LDFLAGS = -ldl -lpthread -lm
endif
ifeq ($(UNAME_S),Darwin)
    LIB_EXT = dylib
    LDFLAGS = -ldl -lpthread -lm
endif
ifeq ($(OS),Windows_NT)
    LIB_EXT = dll
    LDFLAGS = -lws2_32 -luserenv -lbcrypt
endif

CC = gcc
CXX = g++
CFLAGS = -Wall -Wextra -std=c99
CXXFLAGS = -Wall -Wextra -std=c++11

all: c_example cpp_example

rust-lib:
	cd .. && cargo build --release

rust-lib-debug:
	cd .. && cargo build

# C example
c_example: c_example.c rust-lib
	$(CC) $(CFLAGS) -o c_example c_example.c -L$(RUST_LIB_DIR) -l$(LIB_NAME) $(LDFLAGS)

# C++ example
cpp_example: cpp_example.cpp rust-lib
	$(CXX) $(CXXFLAGS) -o cpp_example cpp_example.cpp -L$(RUST_LIB_DIR) -l$(LIB_NAME) $(LDFLAGS)

# Debug versions
c_example_debug: c_example.c rust-lib-debug
	$(CC) $(CFLAGS) -g -o c_example_debug c_example.c -L$(RUST_LIB_DEBUG_DIR) -l$(LIB_NAME) $(LDFLAGS)

cpp_example_debug: cpp_example.cpp rust-lib-debug
	$(CXX) $(CXXFLAGS) -g -o cpp_example_debug cpp_example.cpp -L$(RUST_LIB_DEBUG_DIR) -l$(LIB_NAME) $(LDFLAGS)

# Run examples
run-c: c_example
	@echo "Running C example:"
	@export LD_LIBRARY_PATH=$(RUST_LIB_DIR):$$LD_LIBRARY_PATH && export DYLD_LIBRARY_PATH=$(RUST_LIB_DIR):$$DYLD_LIBRARY_PATH && ./c_example

run-cpp: cpp_example
	@echo "Running C++ example:"
	@export LD_LIBRARY_PATH=$(RUST_LIB_DIR):$$LD_LIBRARY_PATH && export DYLD_LIBRARY_PATH=$(RUST_LIB_DIR):$$DYLD_LIBRARY_PATH && ./cpp_example

run-all: run-c run-cpp

clean:
	rm -f c_example cpp_example c_example_debug cpp_example_debug

# Help
help:
	@echo "Available targets:"
	@echo "  all           - Build both C and C++ examples"
	@echo "  c_example     - Build C example"
	@echo "  cpp_example   - Build C++ example"
	@echo "  rust-lib      - Build Rust library (release)"
	@echo "  rust-lib-debug- Build Rust library (debug)"
	@echo "  run-c         - Build and run C example"
	@echo "  run-cpp       - Build and run C++ example"
	@echo "  run-all       - Build and run both examples"
	@echo "  clean         - Remove built executables"
	@echo "  help          - Show this help"

.PHONY: all rust-lib rust-lib-debug run-c run-cpp run-all clean help
