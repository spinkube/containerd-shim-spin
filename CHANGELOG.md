# Changelog

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Updated the minimum required Rust version to 1.81

## [v0.17.0](https://github.com/spinkube/containerd-shim-spin/releases/tag/v0.17.0) - 2024-11-08

### Added

- Added component filtering based on env var `SPIN_COMPONENTS_TO_RETAIN` ([#197](https://github.com/spinkube/containerd-shim-spin/pull/197))
- Improved error hanlding in selective deployment ([#229](https://github.com/spinkube/containerd-shim-spin/pull/229))

### Changed

- Turn off native unwinding from Wasmtime Config to avoid faulty libunwind detection errors ([#215](https://github.com/spinkube/containerd-shim-spin/pull/215))
- Updated the spin version to v3.0.0 ([#230](https://github.com/spinkube/containerd-shim-spin/pull/230))

### Fixed

- FIxed CI errors due to old versions of Go and TinyGo and disk pressure ([#217](https://github.com/spinkube/containerd-shim-spin/pull/217))


## [v0.16.0](https://github.com/spinkube/containerd-shim-spin/releases/tag/v0.16.0) - 2024-10-04

### Added

- Added MQTT trigger and tests ([#175](https://github.com/spinkube/containerd-shim-spin/pull/175))
- Make container environment variables accessible as application variables ([#149](https://github.com/spinkube/containerd-shim-spin/pull/149))
- Added feature to conditionally restart the k0s controller service when present during node installation. ([#167](https://github.com/spinkube/containerd-shim-spin/pull/167))

### Changed

- Updated the minimum required Rust version to 1.79 ([#191](https://github.com/spinkube/containerd-shim-spin/pull/191))
- Refactored the shim code by splitting it into different modules ([#185](https://github.com/spinkube/containerd-shim-spin/pull/185))
- Refactored the Makefile to improve its structure and comments([#171](https://github.com/spinkube/containerd-shim-spin/pull/171))
- Merged two Redis trigger test apps into one ([#176](https://github.com/spinkube/containerd-shim-spin/pull/176))
- Simplified the run command in the documentation ([#184](https://github.com/spinkube/containerd-shim-spin/pull/184))
-  Modified Dependabot settings to group patch-level dependency updates ([#162](https://github.com/spinkube/containerd-shim-spin/pull/162))

### Fixed

- Correct currently supported triggers ([#182](https://github.com/spinkube/containerd-shim-spin/pull/182))
- Fixed an error in `setup-linux.sh` script ([#184](https://github.com/spinkube/containerd-shim-spin/pull/184))
- Updated outdated links to `spinkube.dev` ([#170](https://github.com/spinkube/containerd-shim-spin/pull/170))

---

[Unreleased]: <https://github.com/spinkube/containerd-shim-spin/compare/v0.17.0..HEAD>
[v0.17.0]: https://github.com/spinkube/containerd-shim-spin/compare/v0.16.0...v0.17.0
[v0.16.0]: https://github.com/spinkube/containerd-shim-spin/compare/v0.15.1...v0.16.0
