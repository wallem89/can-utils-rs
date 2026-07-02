# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Rust Semantic Versioning](https://doc.rust-lang.org/cargo/reference/semver.html).

## [Unreleased]

## [0.5.0] - 2026-07-02
### Added
- CAN dump adds option to filter on one or more identifiers

## [0.4.1] - 2026-05-01
### Changed
- Bump Rust version

## [0.4.0] - 2026-03-21
### Added
- CAN send option to CLI for one message or cyclic message sending

## [0.3.0] - 2026-03-16
### Added
- API to set-up a CAN bus without a CLI

## [0.2.0] - 2026-03-14
### Added
- Check and option to install prerequisites
- Pretty CAN dump

## [0.1.0] - 2026-03-13
### Added
- Initial release with core features:
 - Interactive configuration using [`inquire`]
 - Pretty bitrate selection menus
 - Automatic detection of existing interfaces
 - Option to:
  - replace existing interfaces
  - rename the new interface
  - keep the existing interface
 - Safe execution preview before applying changes
 - Works with standard Linux CAN tools (`ip`, `slcand`)
