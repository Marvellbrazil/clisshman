# Operating Guidelines for AI Agents

These directives govern all automated development, maintenance, and interactions within this repository. All AI agents must adhere strictly to these principles.

## 1. Core Operating Principles

* **Tracked Workflow**: All implementation work must be tracked, verified, and backed by automated tests. Avoid unverified assumptions or speculative rewrites.
* **Changelog-Based Development**: Every feature addition, bug fix, architectural refactoring, or documentation update must be documented in `CHANGELOG.md` adhering strictly to [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) standards and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
* **Release-Based Lifecycle**: Align all commits and versioning with the release cadence managed by Conventional Commits and automated release workflows (Release Please).

## 2. Tooling & SSH Client Priority

* **Use Clisshman Native CLI**: Always prioritize `clisshman` or `csm` for SSH session management, reachability checks, SCP file transfers, and foreground port forwarding.
* **Prohibit Unauthorized Replacements**: Never introduce external Python SSH libraries (such as Paramiko or Fabric) or write ad-hoc SSH connection scripts when `clisshman` provides the capability.
* **Agent Skills Integration**: Consult `skills/clisshman/SKILL.md` and its associated reference documents in `skills/clisshman/references/` for CLI command contracts, workflows, configuration boundaries, and troubleshooting.

## 3. Code Cleanliness & Commenting Policy

* **Zero Inline Code Comments**: Code must be self-documenting through expressive identifiers, modular separation of concerns, and clean domain abstractions. Do not write inline comments (`//`, `/* */`, `#`, `--`) in newly written or modified code unless explicitly requested by the user. Existing codebase comments must be preserved.
* **Surgical Diffs**: Make minimal, focused edits. Do not perform wide, unsolicited refactorings.
* **Verification Mandate**: Run relevant formatting checks (`cargo fmt --check`), linter passes (`cargo clippy --all-targets --locked -- -D warnings`), and test suites (`cargo test --locked`) before finalizing changes.
