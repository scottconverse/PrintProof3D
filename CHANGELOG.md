# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-05-30

### Added
- Initial workspace structuring with five crates (`core`, `printability`, `adapters`, `sdk`, `cli`).
- Defined core profile models: `PrinterProfile`, `MaterialProfile`, and `ValidationReport`.
- Integrated `schemars` auto-generation in core unit tests to output JSON schemas dynamically.
- Created fixtures library under `/fixtures` containing manifold/non-manifold STLs and boundary-safe/unsafe G-codes.
- Configured workspace pre-push hooks gating compilation and test passes.
