# Makefile — Code Buddy release helper
#
# Targets:
#   make build           - debug build for current platform
#   make release         - release build for current platform
#   make build-all       - cross-compile for all 5 targets (requires targets installed)
#   make test            - run all tests
#   make lint            - clippy + format check
#   make package         - build-all + create archives + checksums.txt
#   make publish TAG=v0.1.0 - push a git tag to trigger GitHub Actions release

BINARY     := code-buddy
CARGO      := cargo
TARGETS    := x86_64-unknown-linux-musl \
              aarch64-unknown-linux-musl \
              x86_64-apple-darwin \
              aarch64-apple-darwin \
              x86_64-pc-windows-msvc
DIST_DIR   := dist

.PHONY: build release build-all test lint clean package publish

build:
	$(CARGO) build

release:
	$(CARGO) build --release

test:
	$(CARGO) test --all -- --test-threads=4

lint:
	$(CARGO) clippy --all-targets -- -D warnings
	$(CARGO) fmt --all --check

build-all:
	@for target in $(TARGETS); do \
	    echo "Building $$target…"; \
	    cargo build --release --target $$target; \
	done

package: build-all
	@rm -rf $(DIST_DIR) && mkdir -p $(DIST_DIR)
	@rm -f checksums.txt
	@for target in $(TARGETS); do \
	    bin="target/$$target/release/$(BINARY)"; \
	    ext=""; \
	    if echo "$$target" | grep -q windows; then \
	        ext=".exe"; bin="$$bin.exe"; \
	    fi; \
	    name="$(BINARY)-$$target"; \
	    if echo "$$target" | grep -q windows; then \
	        zip -j "$(DIST_DIR)/$$name.zip" "$$bin" README.md; \
	        sha256sum "$(DIST_DIR)/$$name.zip" >> checksums.txt; \
	    else \
	        strip "$$bin" 2>/dev/null || true; \
	        tar czf "$(DIST_DIR)/$$name.tar.gz" -C "$$(dirname $$bin)" "$(BINARY)"; \
	        sha256sum "$(DIST_DIR)/$$name.tar.gz" >> checksums.txt; \
	    fi; \
	    echo "Packaged $$name"; \
	done
	@cp checksums.txt $(DIST_DIR)/
	@echo "Archives and checksums in $(DIST_DIR)/"

publish:
ifndef TAG
	$(error TAG is required. Usage: make publish TAG=v0.1.0)
endif
	@echo "Tagging $(TAG) and pushing to trigger GitHub Actions release…"
	git tag -a $(TAG) -m "Release $(TAG)"
	git push origin $(TAG)
	@echo "Release workflow started. Check:"
	@echo "  https://github.com/simpletoolsindia/code-buddy/actions"

clean:
	$(CARGO) clean
	rm -rf $(DIST_DIR) checksums.txt
