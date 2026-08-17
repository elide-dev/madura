# Changelog

## [1.2.0](https://github.com/elide-dev/madura/compare/v1.1.0...v1.2.0) (2026-08-17)


### Features

* adopt `--libc=cosmo` ([#10](https://github.com/elide-dev/madura/issues/10)) ([52f1cc5](https://github.com/elide-dev/madura/commit/52f1cc57c7cd44b74b834dd093a7b0532a2adc25))

## [1.1.0](https://github.com/elide-dev/madura/compare/v1.0.1...v1.1.0) (2026-08-11)


### Features

* macos support ([#7](https://github.com/elide-dev/madura/issues/7)) ([20a18d8](https://github.com/elide-dev/madura/commit/20a18d8caf4253ef26a77d33d8048cfa3ec59842))


### Bug Fixes

* **ci:** publish releases from a draft so assets can attach ([#5](https://github.com/elide-dev/madura/issues/5)) ([10ccde4](https://github.com/elide-dev/madura/commit/10ccde47b317e49a9527a9da49201054afc5cc9a))

## [1.0.1](https://github.com/elide-dev/madura/compare/v1.0.0...v1.0.1) (2026-08-09)


### Bug Fixes

* **ci:** let release-please tag the merged release PR ([3975092](https://github.com/elide-dev/madura/commit/3975092aa9d56d178a089bb1b2f6870160a7ddfd))

## [1.0.0](https://github.com/elide-dev/madura/compare/v1.0.0...v1.0.0) (2026-08-09)


### Features

* **dist:** hermetic dist packaging ([fbca758](https://github.com/elide-dev/madura/commit/fbca75865c83d9a834c084801413aea4401bb0a2))
* elide-based local crate build ([4c36812](https://github.com/elide-dev/madura/commit/4c36812603141d874878747614a067992bbfde3d))
* introduce `check` and `compile` (default) modes ([2ff51e7](https://github.com/elide-dev/madura/commit/2ff51e708531fb9a713e081383fdaf6b0877edb8))
* **madura_javac:** invoke the system java compiler with passthrough args ([4f9fdff](https://github.com/elide-dev/madura/commit/4f9fdffefd62cc847f944c25c9893f0aacdaf283))
* **madura:** binary-relative java.home + exact reachability metadata ([1b68b72](https://github.com/elide-dev/madura/commit/1b68b72660ba7c1d243a1496386b65254bc175b5))
* **madura:** javac-compatible bin over madura-javac.so with e2e tests ([7bf1654](https://github.com/elide-dev/madura/commit/7bf165460992f69f11a8b997f709efb06c474412))
* release automation with release-please ([2bcb58b](https://github.com/elide-dev/madura/commit/2bcb58bf8c85144ef66bfd649bcd62affc7ff25d))
* use controlled entrypoint for compile in-proc ([ed79bf5](https://github.com/elide-dev/madura/commit/ed79bf5529add72d4e43a7819ab9495abf786962))


### Bug Fixes

* always prefer the shipped platform image over a baked-in java.home ([29dea1a](https://github.com/elide-dev/madura/commit/29dea1aeaaba849ed0366d3b8f48ca873c4d9296))
* **build:** rebuild the image and dist when their sources change ([fe47e48](https://github.com/elide-dev/madura/commit/fe47e487c4959cf2e1f17aa8e7805744b1f37610))
* **build:** uncached elide builds + mtime-aware staging ([0cd688f](https://github.com/elide-dev/madura/commit/0cd688fdc348c7f94bebf0d7c8516b1219f44e31))
* bump max heap size ([ff0a452](https://github.com/elide-dev/madura/commit/ff0a452dbce02d76e44ea021b4c71d5c01a797d2))
* cfi flags ([ebd8d95](https://github.com/elide-dev/madura/commit/ebd8d953171a2de6ff5693a05a3dbc2e055fb77c))
* don't download rust toolchain components for all targets ([7071db8](https://github.com/elide-dev/madura/commit/7071db83a87d292bca9b1cc07c53ed4a9797803d))
* drop modules from main compile + drop kotlin stdlib ([2d32739](https://github.com/elide-dev/madura/commit/2d32739f4978393714edee656d2b2ce6d1e6bee4))
* git attributes ([0c70c25](https://github.com/elide-dev/madura/commit/0c70c25cf2b9335a195267ca19acd5a7d24450c2))
* git attributes ([e7bf9de](https://github.com/elide-dev/madura/commit/e7bf9de6f0ce13ab255767636458fd5045e59c33))
* gitattributes + langs ([d7902af](https://github.com/elide-dev/madura/commit/d7902af501de22db1c612a7f03121666888c03ef))
* **image:** resolve the jrt platform image path at run time, not build time ([b5299da](https://github.com/elide-dev/madura/commit/b5299da5914c3e20581a54018be5055314e59ac3))
* it's gitattributes ([522609c](https://github.com/elide-dev/madura/commit/522609c1f884d14ee359338877983c8d5a8cd54e))
* lang stats ([3aa8911](https://github.com/elide-dev/madura/commit/3aa89112066ec31a7cd15d46519348751c5992cd))
* native arch compat ([2bcf3a8](https://github.com/elide-dev/madura/commit/2bcf3a829369e2b576de076eb70a939b7e1841e9))
* restore symbols for profiling ([d045cad](https://github.com/elide-dev/madura/commit/d045cadc012f1162ed8eb23e0555f687049b8216))
* support bare cargo commands (no expected env) ([fe20bbd](https://github.com/elide-dev/madura/commit/fe20bbd761c5796c0b881858bb35c653c00c3135))


### Build System

* **madura_javac:** enable JRT filesystem for javac platform metadata ([be5c8bf](https://github.com/elide-dev/madura/commit/be5c8bf5d1f334e6bfdbbb6c6f023b4def417108))
