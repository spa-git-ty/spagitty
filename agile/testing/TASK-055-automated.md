<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-055 — automated verification

2026-10-06.

- Passed: the packaged native worker handshake/activation/deactivation/exit through the SDK process harness.
- Passed: native Windows production Tauri build creates MSI and NSIS installers; the final source/dependency rebuild passed.
- Inspected WiX and resource trees: target-specific CodeRabbit manifest, native worker, license and README are included beneath extensions/spagitty.coderabbit.
- Passed: independently built hello extension public harness (2 tests plus handshake/activate/deactivate/exit), Windows package creation, and opt-in real host import/run/disable/uninstall test.
- Passed: complete Linux Rust workspace; worker lifecycle/process tests and farm supplemental tests run on both Linux and Windows.
- macOS sidecar/signing configuration retains the existing window configuration; its automated config tests pass.
- Shipping classification includes extensions, SDK, extension tools and protocol schemas.
- Final coverage, license/security checks and build outputs are in extensions-continuation-review.md.

Linux production installers, both macOS architectures/signing, and GUI installation sweeps remain unverified on this Windows host.
