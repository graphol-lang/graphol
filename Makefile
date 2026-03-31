BIN_NAME := graphol
PREFIX ?= /usr/local
BINDIR := $(PREFIX)/bin

build:
	cargo build --release

install:
	install -Dm755 target/release/$(BIN_NAME) $(BINDIR)/$(BIN_NAME)

uninstall:
	rm -f $(BINDIR)/$(BIN_NAME)
