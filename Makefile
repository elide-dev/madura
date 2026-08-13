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
ELIDE ?= $(shell which elide)
JLINK ?= $(shell which jlink)

# native-image is invoked as an external sub-process (`driverMode = "external"`
# in elide.pkl), so a GraalVM with `native-image` must be reachable. Point
# GRAALVM_HOME at it (the same JDK works as JAVA_HOME at build and run time).
IMAGE := .dev/artifacts/native-image/madura

# Platform metadata shipped beside the binary: a jlink'd minimal image supplies
# lib/modules and lib/ct.sym for the hermetic distribution.
JDKROOT := target/jdkroot

all: build  ## Build all targets.

build: target/dist  ## Build the madura distribution.

test: build  ## Run all tests.
	@echo "Running madura tests..."
	$(RULE)$(ELIDE) test
	$(RULE)$(BUN) test

clean:  ## Clean built targets.
	$(RULE)rm -fr target .dev/artifacts
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
		-c "hyperfine --shell=none --warmup=10 --runs=25 -n 'javac ...' '$$JAVA_HOME/bin/javac -d target ./tests/smoke/simple/Hello.java' -n 'madura check ...' './target/dist/madura check ./tests/smoke/simple/Hello.java' && sleep 2"
	$(RULE)agg --font-size 18 --speed 1.5 madura-check.cast ./docs/check.gif
	$(RULE)rm -fv madura-check.cast
	@echo "Gifs regenerated."

target/dist: $(IMAGE) $(JDKROOT)
	@echo "+ Assembling distribution..."
	$(RULE)./scripts/make-dist.sh

deps: node_modules/ .dev/dependencies  ## Install dependencies.

node_modules/:
	@echo "+ Installing NPM dependencies..."
	$(RULE)$(AUBE) install

.dev/dependencies:
	@echo "+ Installing Maven dependencies..."
	$(RULE)$(ELIDE) install --ecosystems maven

IMAGE_SRCS := $(wildcard src/*.kt) elide.pkl $(wildcard native-image/*)

image: $(IMAGE)  ## Build the native-image binary.

$(IMAGE): $(IMAGE_SRCS)
	@echo "+ Building madura native image..."
	$(RULE)$(ELIDE) build --no-cache --release

jdkroot: $(JDKROOT)  ## Build the minimal jlink'd JDK metadata.

$(JDKROOT):
	@echo "+ Building minimal JDK (jlink)..."
	$(RULE)rm -fr $(JDKROOT)
	$(RULE)$(JLINK) \
		--add-modules java.base,java.compiler,jdk.compiler \
		--strip-debug \
		--no-header-files \
		--no-man-pages \
		--output $(JDKROOT)

help: ## Show this help message.
	@echo "madura:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' Makefile | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-28s\033[0m %s\n", $$1, $$2}'
	@echo ""

.PHONY: all build test clean deps image jdkroot rebuild-gifs help
