# Changelog

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - YYYY-MM-DD

### Added

- Added MQTT trigger and tests ([#175](https://github.com/spinkube/containerd-shim-spin/pull/175))
- Make container environment bariables accessible as application variables ([#149](https://github.com/spinkube/containerd-shim-spin/pull/149))
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

[Unreleased]: https://github.com/spinkube/containerd-shim-spin/compare/v0.15.1...HEAD