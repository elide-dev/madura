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
DOCKER ?= $(shell which docker)

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
	$(RULE)$(ELIDE) test dev
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

# `wildcard` does not recurse, and the Kotlin sources sit under their package
# directories, so the file list comes from `find`.
IMAGE_SRCS := $(shell find src -name '*.kt') elide.pkl $(wildcard native-image/*)

image: $(IMAGE)  ## Build the native-image binary.

$(IMAGE): $(IMAGE_SRCS)
	@echo "+ Building madura native image..."
	$(RULE)$(ELIDE) build --no-cache

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

# Local single-arch mirror of what `job.docker.yml` builds. CI assembles a
# multi-arch context from both ELF tarballs; here the context is whatever
# `make build` just produced, so the image can only be built on Linux — the
# Dockerfile's `COPY madura-linux-${TARGETARCH}/` wants a Linux distribution,
# and a darwin one would be a mislabeled lie rather than a broken build.
DOCKER_TAG ?= madura:dev
DOCKER_ARCH ?= $(shell uname -m | sed -e 's/^x86_64$$/amd64/' -e 's/^aarch64$$/arm64/')
DOCKER_CTX := target/docker/madura-linux-$(DOCKER_ARCH)

docker: target/dist  ## Build the container image from the assembled distribution.
	@[ "$$(uname -s)" = "Linux" ] || { \
		echo "docker: the image needs a linux distribution; build it on linux (CI does)"; \
		exit 1; \
	}
	@echo "+ Building container image ($(DOCKER_TAG), linux/$(DOCKER_ARCH))..."
	$(RULE)rm -fr $(DOCKER_CTX)
	$(RULE)mkdir -p $(DOCKER_CTX)
	$(RULE)cp -R target/dist/madura target/dist/lib $(DOCKER_CTX)/
	$(RULE)$(DOCKER) build \
		-f Dockerfile \
		--platform linux/$(DOCKER_ARCH) \
		-t $(DOCKER_TAG) \
		target/docker
	@echo "Image built: $(DOCKER_TAG)"

help: ## Show this help message.
	@echo "madura:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' Makefile | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-28s\033[0m %s\n", $$1, $$2}'
	@echo ""

.PHONY: all build test clean deps docker image jdkroot rebuild-gifs help
