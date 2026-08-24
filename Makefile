ID := jltrench.walls
PLUGIN_DIR := $(HOME)/.config/omarchy/plugins/$(ID)
QMLLINT := $(shell command -v qmllint || echo /usr/lib/qt6/bin/qmllint)

.PHONY: build install validate lint test remove clean

build:
	cargo build --release --manifest-path rust/Cargo.toml

## Build the binary and sync everything into the Omarchy plugin folder.
install: build
	@mkdir -p $(PLUGIN_DIR)/bin
	cp manifest.json BarWidget.qml Panel.qml ResultGrid.qml SavedGrid.qml icon.svg README.md LICENSE $(PLUGIN_DIR)/
	cp rust/target/release/walls $(PLUGIN_DIR)/bin/
	@echo "Installed $(ID) -> $(PLUGIN_DIR)"

validate: 
	omarchy plugin validate $(PLUGIN_DIR)

lint:
	$(QMLLINT) -I "$${OMARCHY_PATH}/shell" BarWidget.qml Panel.qml ResultGrid.qml || true

test:
	cargo test --manifest-path rust/Cargo.toml

remove:
	omarchy plugin remove $(ID) --yes 2>/dev/null || rm -rf $(PLUGIN_DIR)

clean:
	cargo clean --manifest-path rust/Cargo.toml
