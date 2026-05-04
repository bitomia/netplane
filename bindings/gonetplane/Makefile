GO ?= go
ROOT_DIR := $(dir $(realpath $(lastword $(MAKEFILE_LIST))))

ifeq ($(OS),Windows_NT)
	RM = del
else
	RM = rm -rf
endif

.PHONY: all
all:
	$(GO) mod tidy
	$(GO) generate .
	$(GO) build .

.PHONY: test
test:
	cargo build -r -p netplane_client
	$(GO) test -ldflags="-extldflags '-L../../target/release'" -v .

clean:
	-$(RM) -f private.key public.key auth.key
