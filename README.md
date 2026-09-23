# Folio

Canonical project location: `/Volumes/Main External/Development/Folio/`.

## Active application: folio-native

The desktop-first Flutter and Rust application is in [folio-native](folio-native/).

- [Flutter interface](folio-native/lib/) — application entry point: [main.dart](folio-native/lib/main.dart).
- [Rust engine](folio-native/rust/) — source entry point: [lib.rs](folio-native/rust/src/lib.rs).
- [macOS integration](folio-native/macos/).
- [Product roadmap](folio-native/ROADMAP.md).
- [Execution goal prompt](folio-native/docs/GOAL_PROMPT.md).
- [Build and usage instructions](folio-native/README.md).

Run development commands from `folio-native`. Use this external-drive project for future work; the dated Codex task directory is the original location, not the working project home.

## Earlier prototype: folio-desktop

[folio-desktop](folio-desktop/) contains the earlier Electron prototype. Continue product development in `folio-native`.

Migration note: copied build caches may contain absolute paths to the original directory. Regenerate affected caches when building at the new location; copying files does not verify a new build. Historical validation archives retain their original paths as evidence.

## Repository contents

The repository includes application source, vendored source dependencies and their license files, tests and fixtures, build scripts, roadmap, and validation documentation. Generated builds, downloaded runtimes, dependency caches, and local release/validation archives are excluded from Git and retained on disk.

The primary application is Flutter/Dart plus Rust. Start with the [native project README](folio-native/README.md) for toolchain and build instructions. The source was relocated before repository creation; building from this checkout is a separate verification step.
