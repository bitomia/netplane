ROOT_DIR := $(dir $(realpath $(lastword $(MAKEFILE_LIST))))

ifeq ($(OS),Windows_NT)
	RM = del
else
	RM = rm -rf
endif

.PHONY: bindgen
bindgen:
	cd $(ROOT_DIR)../.. && \
	cbindgen --config $(ROOT_DIR)cbindgen.toml --crate netplane_client --output $(ROOT_DIR)netplane.h

clean:
	-$(RM) -f netplane.h
	-$(RM) -f private.key public.key auth.key
