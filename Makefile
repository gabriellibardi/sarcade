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
SGLEAM_MANIFEST := sgleam/Cargo.toml
SGLEAM_BINARY := sgleam/target/$(TARGET)/release/sgleam

.PHONY: build backend frontend sgleam-runtime setup check verify-sources help

build: backend

backend: verify-sources frontend sgleam-runtime
	SARCADE_SGLEAM_BINARY="$(abspath $(SGLEAM_BINARY))" \
		$(CARGO) build --locked --release --target $(TARGET) \
		--manifest-path $(BACKEND_MANIFEST) --features embedded-sgleam
	@printf '\nBinário gerado em %s\n' "$(BINARY)"

frontend:
	$(NPM) --prefix $(FRONTEND_DIR) run build

sgleam-runtime: verify-sources
	$(CARGO) build --locked --release --target $(TARGET) \
		--manifest-path $(SGLEAM_MANIFEST) -p sgleam --bin sgleam

setup:
	git submodule update --init --recursive
	$(NPM) --prefix $(FRONTEND_DIR) ci
	$(RUSTUP) target add --toolchain $(RUST_TOOLCHAIN) $(TARGET)

verify-sources:
	./scripts/verify-runtime-pins.sh

check: verify-sources
	$(NPM) --prefix $(FRONTEND_DIR) run check
	$(NPM) --prefix $(FRONTEND_DIR) run lint
	$(CARGO) test --locked --manifest-path $(BACKEND_MANIFEST)
	$(CARGO) clippy --locked --all-targets --manifest-path $(BACKEND_MANIFEST) -- -D warnings

help:
	@printf '%s\n' \
		'make          Compila o frontend e gera o binário Linux MUSL' \
		'make setup    Instala as dependências e o target MUSL' \
		'make check    Executa as verificações do frontend e do backend' \
		'make verify-sources Confere os commits dos runtimes fixados' \
		'make frontend Compila somente o frontend' \
		'make sgleam-runtime Compila o runtime Sgleam MUSL incorporado'
