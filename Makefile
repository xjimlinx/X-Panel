BINARY   = x-panel
PREFIX  ?= /usr/local
BUILD_DIR = target/release
CARGO   ?= cargo

.PHONY: all build install uninstall clean test lint fmt

all: build

build:
	$(CARGO) build --release

install: build
	sudo install -d $(DESTDIR)$(PREFIX)/bin
	sudo install -m 755 $(BUILD_DIR)/$(BINARY) $(DESTDIR)$(PREFIX)/bin/

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/$(BINARY)

clean:
	$(CARGO) clean

test:
	$(CARGO) test

lint:
	$(CARGO) clippy -- -D warnings

fmt:
	$(CARGO) fmt
