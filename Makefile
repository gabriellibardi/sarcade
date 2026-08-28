SHELL := /bin/sh
.DEFAULT_GOAL := build

TARGET ?= x86_64-unknown-linux-musl
RUST_TOOLCHAIN ?= 1.95.0
CARGO ?= cargo +$(RUST_TOOLCHAIN)
RUSTUP ?= rustup
NPM ?= npm

FRONTEND_DIR := frontend
BACKEND_MANIFEST := backend/Cargo.toml
BINARY := backend/target/$(TARGET)/release/backend

.PHONY: build backend frontend setup check help

build: backend

backend: frontend
	$(CARGO) build --release --target $(TARGET) --manifest-path $(BACKEND_MANIFEST)
	@printf '\nBinário gerado em %s\n' "$(BINARY)"

frontend:
	$(NPM) --prefix $(FRONTEND_DIR) run build

setup:
	$(NPM) --prefix $(FRONTEND_DIR) ci
	$(RUSTUP) target add --toolchain $(RUST_TOOLCHAIN) $(TARGET)

check:
	$(NPM) --prefix $(FRONTEND_DIR) run check
	$(CARGO) test --manifest-path $(BACKEND_MANIFEST)
	$(CARGO) clippy --all-targets --manifest-path $(BACKEND_MANIFEST) -- -D warnings

help:
	@printf '%s\n' \
		'make          Compila o frontend e gera o binário Linux MUSL' \
		'make setup    Instala as dependências e o target MUSL' \
		'make check    Executa as verificações do frontend e do backend' \
		'make frontend Compila somente o frontend'
