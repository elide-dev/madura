VERBOSE ?= no

PROJECT_ROOT ?= $(shell echo $$PWD)

ifeq ($(VERBOSE),yes)
RULE ?=
else
RULE ?= @
endif

MISE ?= $(shell which mise)
BUN ?= $(shell which bun)
AUBE ?= $(shell which aube)
JLINK ?= $(shell which jlink)
CARGO ?= $(shell which cargo)
ELIDE ?= $(shell which elide)

all: target/jdkroot target/dist

build: target/dist  ## Build all targets.

test: build  ## Run all tests.
	@echo "Running madura tests..."
	$(RULE)cd crates/madura_javac && $(ELIDE) test
	$(RULE)$(CARGO) nextest run
	$(RULE)$(BUN) test

clean:  ## Clean built targets.
	$(RULE)$(CARGO) clean
	$(RULE)rm -fr target crates/madura_javac/.dev/artifacts
	@echo "Cleaned."

target:
	$(RULE)mkdir target

rebuild-gifs:  ## Rebuild gifs for repo/docs.
	@echo "Rebuilding gifs..."
	$(RULE)asciinema \
		rec ./madura-check.cast \
		--cols 80 \
		--rows 24 \
		--overwrite \
		-c "hyperfine --shell=none --warmup=10 --runs=25 -n 'javac ...' '/usr/lib/jvm/gvm.jdk25/bin/javac -d target ./tests/smoke/simple/Hello.java' -n 'madura check ...' './target/dist/bin/madura check ./tests/smoke/simple/Hello.java' && sleep 2"
	$(RULE)agg --font-size 18 --speed 1.5 madura-check.cast ./docs/check.gif
	$(RULE)rm -fv madura-check.cast
	@echo "Gifs regenerated."

MADURA_SRCS := $(wildcard crates/madura/src/*.rs) crates/madura/build.rs \
	$(wildcard crates/madura_javac/src/*.rs) crates/madura_javac/build.rs \
	scripts/make-dist.sh Cargo.toml Cargo.lock

# Depends on jdkroot rather than relying on `all` listing them in order: the
# cargo build stages lib/{modules,ct.sym} out of it.
target/dist: target/jdkroot crates/madura_javac/.dev/artifacts/jar/app/app.jar $(MADURA_SRCS)
	@echo "+ Building madura..."
	$(RULE)MADURA_JAVA_HOME=$(PROJECT_ROOT)/target/jdkroot $(CARGO) build --release
	$(RULE)./scripts/make-dist.sh

deps: node_modules/ crates/madura_javac/.dev/dependencies  ## Install dependencies.

node_modules/:
	@echo "+ Installing NPM dependencies..."
	$(RULE)$(AUBE) install

crates/madura_javac/.dev/dependencies:
	@echo "+ Installing Maven dependencies..."
	$(RULE)cd crates/madura_javac && $(ELIDE) install --ecosystems maven

JAVAC_IMAGE_SRCS := $(wildcard crates/madura_javac/src/*.kt) \
	crates/madura_javac/elide.pkl \
	$(wildcard crates/madura_javac/native-image/*)

image: crates/madura_javac/.dev/artifacts/native-image  ## Build the shared-library native image.

crates/madura_javac/.dev/artifacts/jar/app/app.jar crates/madura_javac/.dev/artifacts/native-image: $(JAVAC_IMAGE_SRCS)
	@echo "+ Building javac image..."
	$(RULE)cd crates/madura_javac && $(ELIDE) build --no-cache --release


jdkroot: target/jdkroot  ## Build the minimal jlink'd OpenJDK.

target/jdkroot: target
	@echo "+ Building minimal JDK..."
	$(RULE)rm -fr target/jdkroot
	$(RULE)$(JLINK) \
		--add-modules java.base \
		--add-modules java.compiler \
		--add-modules jdk.compiler \
		--strip-debug \
		--no-header-files \
		--no-man-pages \
		--output target/jdkroot

help: ## Show this help message.
	@echo "madura:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' Makefile | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-28s\033[0m %s\n", $$1, $$2}'
	@echo ""

.PHONY: clean help

