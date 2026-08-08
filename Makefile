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

build: target/dist

test:
	$(BUN) test

clean:
	$(RULE)$(CARGO) clean
	$(RULE)rm -fr target crates/madura_javac/.dev/artifacts
	@echo "Cleaned."

target:
	$(RULE)mkdir target

MADURA_SRCS := $(wildcard crates/madura/src/*.rs) crates/madura/build.rs \
	$(wildcard crates/madura_javac/src/*.rs) crates/madura_javac/build.rs \
	scripts/make-dist.sh Cargo.toml Cargo.lock

# Depends on jdkroot rather than relying on `all` listing them in order: the
# cargo build stages lib/{modules,ct.sym} out of it.
target/dist: target/jdkroot crates/madura_javac/.dev/artifacts/jar/app/app.jar $(MADURA_SRCS)
	@echo "+ Building madura..."
	$(RULE)MADURA_JAVA_HOME=$(PROJECT_ROOT)/target/jdkroot $(CARGO) build --release
	$(RULE)./scripts/make-dist.sh

node_modules/:
	@echo "+ Installing NPM dependencies..."
	$(RULE)$(AUBE) install

crates/madura_javac/.dev/dependencies:
	@echo "+ Installing Maven dependencies..."
	$(RULE)cd crates/madura_javac && $(ELIDE) install --ecosystems maven

JAVAC_IMAGE_SRCS := $(wildcard crates/madura_javac/src/*.kt) \
	crates/madura_javac/elide.pkl \
	$(wildcard crates/madura_javac/native-image/*)

crates/madura_javac/.dev/artifacts/jar/app/app.jar crates/madura_javac/.dev/artifacts/native-image: $(JAVAC_IMAGE_SRCS)
	@echo "+ Building javac image..."
	$(RULE)cd crates/madura_javac && $(ELIDE) build --no-cache --release

target/jdkroot: target crates/madura_javac/.dev/artifacts/jar/app/app.jar
	@echo "+ Building minimal JDK..."
	$(RULE)rm -fr target/jdkroot
	$(RULE)$(JLINK) \
		--module-path ./crates/madura_javac/.dev/artifacts/jar/app/app.jar \
		--add-modules java.base \
		--add-modules java.compiler \
		--add-modules jdk.compiler \
		--strip-debug \
		--no-header-files \
		--no-man-pages \
		--output target/jdkroot

.PHONY: clean
