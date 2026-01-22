CC ?= gcc
CXX ?= g++
PROFILE ?= debug
INCLUDE_DIR := ../include
CFLAGS ?= -I$(INCLUDE_DIR) -Wall -Wextra
CARGO_FLAGS :=
ifeq ($(PROFILE),release)
CARGO_FLAGS += --release
endif

SO_PATH := ../../target/$(PROFILE)/libhycore_cffi.so
SO_DIR := $(dir $(SO_PATH))
SO_BASENAME := $(notdir $(SO_PATH))
SO_STEM := $(patsubst lib%.so,%,$(SO_BASENAME))

LDFLAGS := -L$(SO_DIR) -Wl,-rpath,$(SO_DIR) -l$(SO_STEM)

all: simple.out

lib:
	cargo build $(CARGO_FLAGS) --manifest-path ../Cargo.toml

simple.out: simple.c lib
	echo $(SO_PATH)
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

clean:
	rm -f simple.out
