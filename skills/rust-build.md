---
name: rust-build
description: Use when the user asks to build, test, check, or fix compile errors in the engine
---

# rust build & test

The engine lives in `engine/`. It builds with cargo (no extra toolchain required).

1. Build: run `cargo build` in `engine/`.
2. Tests: run `cargo test` in `engine/`.
3. Fast check: run `cargo check` in `engine/` — use this for quick iterations.
4. If the build fails:
   - read the first error with the error message, then grep the source for the offending symbol
   - fix one error at a time, rebuild, repeat
5. If a test fails, read the test in `engine/src/*.rs` under `#[cfg(test)]`, understand what it asserts, then fix the code or the test's expectation (never weaken a test silently).
6. The binary is `ayesha-os`. Run it with `cargo run --bin ayesha-os` from `engine/`.
7. Never leave a build broken: end with a passing `cargo build` and `cargo test` unless the user says otherwise.
