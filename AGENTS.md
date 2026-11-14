# Project workflow

- Work on feature branches, not directly on the default branch.
- Make focused git commits at tested checkpoints during implementation.
- Sync the feature branch to the remote periodically and before handoff.
- Open pull requests for review; describe the actual implemented scope and
  validation, including incomplete roadmap milestones.
- Preserve vendored source bytes and their notices. Update the verified WEB map
  and semantics documentation alongside engine behavior changes.
- Run source/coverage verification, formatting, Clippy, and debug/release tests
  on Rust 1.85.0 and current stable for engine changes; use the CI workflow as
  the command reference.

The branch/commit/sync/PR workflow records the user's standing preference for
this and future project work. Keep it in effect across sessions in this repo.
