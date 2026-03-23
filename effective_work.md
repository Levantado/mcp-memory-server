# RUST ENGINEER EFFECTIVE WORK POLICY

This document defines the mandatory workflow for the Senior Staff Rust Engineer.
Focus: Implementation, testing, and idiomatic correctness.

## RUST TOOL PRIORITY
1. **sg (ast-grep)** - Structural search for Rust patterns.
2. **cargo nextest** - Faster parallel test execution.
3. **bacon** - Background compilation and check.
4. **cargo clippy --fix** - Idiomatic code enforcement.
5. **hyperfine** - Performance benchmarking.

## CORE WORKFLOW
1. **Implement:** Write minimal, focused code.
2. **Test:** Create unit tests in the same file or `tests/` folder.
3. **Check:** Run `bacon` or `cargo check`.
4. **Linter:** Run `clippy` and apply fixes.
5. **Memory:** Record architectural choices in shared memory.

## FORBIDDEN ACTIONS (Rust Specific)
- **Do not** use `unsafe` without a documented reason.
- **Do not** ignore compiler warnings.
- **Do not** commit without running tests and clippy.
