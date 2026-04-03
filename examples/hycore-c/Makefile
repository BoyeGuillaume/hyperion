CC ?= gcc
CXX ?= g++
PROFILE ?= debug
INCLUDE_DIR := ../../hycore/include
CFLAGS ?= -I$(INCLUDE_DIR) -Wall -Wextra
CARGO_FLAGS :=
ifeq ($(PROFILE),release)
CARGO_FLAGS += --release
endif

SO_EXT := .so
ifeq ($(OS),Windows_NT)
	SO_EXT := .dll
else ifeq ($(OS),Darwin)
	SO_EXT := .dylib
endif

SO_PATH := ../../target/$(PROFILE)/libhycore$(SO_EXT)
SO_DIR := $(dir $(SO_PATH))
SO_BASENAME := $(notdir $(SO_PATH))
SO_STEM := $(patsubst lib%$(SO_EXT),%,$(SO_BASENAME))

LDFLAGS := -L$(SO_DIR) -Wl,-rpath,$(SO_DIR) -l$(SO_STEM) -lm

all: sample.out

lib:
	cargo build $(CARGO_FLAGS) --manifest-path ../../hycore/Cargo.toml -F cffi,remote,remote-tls

sample.out: sample.c lib
	echo $(SO_PATH)
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

run: sample.out
	./sample.out

clean:
	rm -f sample.out
