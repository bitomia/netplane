UNAME_S := $(shell uname -s)

LIB_NAME = netplane
RUST_LIB_DIR = ../../../target/release
RUST_LIB_DEBUG_DIR = ../../../target/debug

ifeq ($(UNAME_S),Linux)
    LDFLAGS = -ldl -lpthread -lm
endif
ifeq ($(UNAME_S),Darwin)
    LDFLAGS = -ldl -lpthread -lm
endif
ifeq ($(OS),Windows_NT)
    LDFLAGS = -lws2_32 -luserenv -lbcrypt
endif

CC = gcc
CFLAGS = -Wall -Wextra -std=c99

all: test

debug: test_debug

rust-lib:
	cd .. && cargo build --release --lib

rust-lib-debug:
	cd .. && cargo build --lib

test: test.c rust-lib
	$(CC) $(CFLAGS) -o test test.c -L$(RUST_LIB_DIR) -l$(LIB_NAME) $(LDFLAGS)

test_debug: test.c rust-lib-debug
	$(CC) $(CFLAGS) -g -o test_debug test.c -L$(RUST_LIB_DEBUG_DIR) -l$(LIB_NAME) $(LDFLAGS)

clean:
	rm -f test test_debug

.PHONY: all rust-lib rust-lib-debug run-c run-all clean
