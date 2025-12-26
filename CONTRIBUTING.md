# Contributing to Urban Sprawl

Thank you for your interest in contributing! This document provides guidelines for contributing to the project.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/urban_sprawl.git`
3. Create a branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Run checks: `cargo clippy && cargo test`
6. Commit and push to your fork
7. Open a Pull Request

## Development Setup

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build and run
cargo run

# Run with optimizations (faster runtime, slower compile)
cargo run --release
```

## Code Guidelines

### Architecture

- All game logic must be implemented as Bevy Systems
- Use Components for entity data, Resources for global state
- Follow the existing plugin structure in `src/`
- Keep systems small and focused (single responsibility)

### Style

- Run `cargo fmt` before committing
- Run `cargo clippy` and address warnings
- Use descriptive variable and function names
- Document public APIs with doc comments

### Performance

- Avoid allocations in hot paths
- Use `&str` over `String` where possible
- Prefer enums over `Box<dyn Trait>` for polymorphism
- Test with `cargo run --release` for performance work

## Pull Requests

- Keep PRs focused on a single feature or fix
- Update documentation if adding new features
- Add tests for new functionality where applicable
- Ensure `cargo clippy` passes without warnings

## Reporting Issues

When reporting bugs, please include:

- Operating system and GPU
- Rust version (`rustc --version`)
- Steps to reproduce
- Expected vs actual behavior
- Console output or error messages

## Questions?

Open a [Discussion](https://github.com/bneidlinger/urban_sprawl/discussions) for questions or ideas.
