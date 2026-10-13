PREFIX ?= $(HOME)/.local
DESTDIR ?=

.PHONY: build test install
build:
	cargo build --release --locked

test:
	cargo test --locked
	cargo fmt --check
	cargo clippy --all-targets --locked -- -D warnings

install: build
	install -Dm755 target/release/workcircuit $(DESTDIR)$(PREFIX)/bin/workcircuit
	install -Dm644 data/io.github.workcircuit.WorkCircuit.desktop $(DESTDIR)$(PREFIX)/share/applications/io.github.workcircuit.WorkCircuit.desktop
	sed 's|^Exec=.*|Exec="$(PREFIX)/bin/workcircuit"|' data/io.github.workcircuit.WorkCircuit.desktop > "$(DESTDIR)$(PREFIX)/share/applications/io.github.workcircuit.WorkCircuit.desktop"
	install -Dm644 data/io.github.workcircuit.WorkCircuit.svg $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/io.github.workcircuit.WorkCircuit.svg
	install -Dm644 data/io.github.workcircuit.WorkCircuit-chevron-symbolic.svg $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/io.github.workcircuit.WorkCircuit-chevron-symbolic.svg
