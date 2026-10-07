# Changelog

All notable changes to `bq40z50-rx` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking

- Correct direct/MAC status mappings
  ([OpenDevicePartnership/bq40z50#93][pr-93]):
  - R3/R4/R5: move `emshut()` to bit 6, expose bit 29 as `disconn()`,
    rename `opncell()` to `force()`, and remove `PfAlert::opnc()`.
  - Replace `slpad()` with `vlb()` on R4 and `storagem()` on R5.
  - Remove `SafetyStatus::ptos()` on all revisions; retain
    `SafetyAlert::ptos()`.

### Added

- Add missing protection/shutdown flags on R3/R4/R5, R5 temperature/watchdog
  and TMP468 status, R4/R5 OCV prediction, and R5 low-battery gauging status
  ([OpenDevicePartnership/bq40z50#93][pr-93]).

### Fixed

- Limit buffer writes to the address and caller-provided bytes, without trailing
  padding ([OpenDevicePartnership/bq40z50#67][issue-67]).
- Return `DataTooLarge` before I/O for oversized register/command slices and
  buffer writes ([OpenDevicePartnership/bq40z50#75][issue-75]).
- Correct both calibration-stop commands to `0xF080` on every revision while
  preserving their public method names as aliases of the exit command
  ([OpenDevicePartnership/bq40z50#66][issue-66]).

### Changed

- Document non-atomic data flash writes and caller-managed recovery, with a
  regression test for failure after a committed chunk
  ([OpenDevicePartnership/bq40z50#79][issue-79]).
- Strengthen battery-status and manufacturer-info assertions and use distinct
  payloads for multi-block data flash tests
  ([OpenDevicePartnership/bq40z50#82][issue-82]).

[pr-93]: https://github.com/OpenDevicePartnership/bq40z50/pull/93
[issue-66]: https://github.com/OpenDevicePartnership/bq40z50/issues/66
[issue-67]: https://github.com/OpenDevicePartnership/bq40z50/issues/67
[issue-75]: https://github.com/OpenDevicePartnership/bq40z50/issues/75
[issue-79]: https://github.com/OpenDevicePartnership/bq40z50/issues/79
[issue-82]: https://github.com/OpenDevicePartnership/bq40z50/issues/82
