ID := jltrench.walls
PLUGIN_DIR := $(HOME)/.config/omarchy/plugins/$(ID)
QMLLINT := $(shell command -v qmllint || echo /usr/lib/qt6/bin/qmllint)

.PHONY: build install validate lint test remove clean release

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

## Tag, push and create a GitHub release for the current version.
## Usage: make release VERSION=v0.3.1
release:
	@test -n "$(VERSION)" || (echo "Usage: make release VERSION=vX.Y.Z" && exit 1)
	@test -z "$$(git status --porcelain)" || (echo "Working tree is dirty; commit first" && exit 1)
	@test "$$(git rev-parse --abbrev-ref HEAD)" = "master" || (echo "Release from master only" && exit 1)
	@test -z "$$(git tag -l '$(VERSION)')" || (echo "Tag $(VERSION) already exists" && exit 1)
	git tag -a $(VERSION) -m "Walls $(VERSION)"
	git push origin master
	git push origin $(VERSION)
	gh release create $(VERSION) --title "Walls $(VERSION)" --notes "See CHANGELOG.md for details."
	@echo "Released $(VERSION) - update the marketplace via the verify form with SHA $$(git rev-parse HEAD)"
