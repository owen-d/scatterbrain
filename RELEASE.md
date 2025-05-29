# Release Guide for Scatterbrain

This document provides comprehensive instructions for maintainers on how to create releases for the Scatterbrain project.

## Overview

Scatterbrain uses [cargo-dist](https://opensource.axo.dev/cargo-dist/) for automated binary building and GitHub releases. The entire release process is automated through GitHub Actions and triggered by git tags.

## Release Process

### 1. Prepare the Release

1. **Update version in `Cargo.toml`**:
   ```toml
   [package]
   version = "0.2.0"  # Update to new version
   ```

2. **Update version references** (if any):
   - Check README.md for hardcoded version references
   - Update any documentation that mentions specific versions

3. **Test the build locally**:
   ```bash
   # Test regular build
   cargo build --release
   
   # Test all tests pass
   cargo test
   
   # Test cargo-dist build
   cargo dist build --tag v0.2.0
   ```

### 2. Create and Push the Release

1. **Commit version changes**:
   ```bash
   git add Cargo.toml
   git commit -m "chore: bump version to 0.2.0"
   git push origin main
   ```

2. **Create and push the release tag**:
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

3. **Monitor the GitHub Actions workflow**:
   - Go to https://github.com/owen-d/scatterbrain/actions
   - Watch the "Release" workflow triggered by the tag
   - Ensure all build jobs complete successfully

### 3. Verify the Release

Once the GitHub Actions workflow completes:

1. **Check the GitHub release**:
   - Visit https://github.com/owen-d/scatterbrain/releases
   - Verify the new release was created
   - Confirm all expected artifacts are present

2. **Test the installers**:
   ```bash
   # Test shell installer (macOS/Linux)
   curl --proto '=https' --tlsv1.2 -LsSf https://github.com/owen-d/scatterbrain/releases/latest/download/scatterbrain-installer.sh | sh
   
   # Test the installed binary
   scatterbrain --version
   ```

3. **Verify checksums**:
   - Download `sha256.sum` from the release
   - Verify checksums match the individual artifact checksums

## Supported Platforms

The automated release process builds binaries for:

- **macOS**: 
  - `aarch64-apple-darwin` (Apple Silicon)
  - `x86_64-apple-darwin` (Intel)
- **Linux**:
  - `x86_64-unknown-linux-gnu` (x64)
  - `aarch64-unknown-linux-gnu` (ARM64)
- **Windows**:
  - `x86_64-pc-windows-msvc` (x64)

## Generated Artifacts

Each release automatically generates:

1. **Binary archives** (`.tar.xz` for Unix, `.zip` for Windows)
2. **Individual checksums** (`.sha256` files)
3. **Combined checksum file** (`sha256.sum`)
4. **Shell installer** (`scatterbrain-installer.sh`)
5. **PowerShell installer** (`scatterbrain-installer.ps1`)
6. **Source archive** (`source.tar.gz`)

## Troubleshooting

### Build Failures

If the GitHub Actions workflow fails:

1. **Check the workflow logs** in the Actions tab
2. **Test locally** with `cargo dist build --tag vX.Y.Z`
3. **Common issues**:
   - Compilation errors: Fix code and create a new tag
   - Missing dependencies: Update the workflow or Cargo.toml
   - Platform-specific issues: Check cross-compilation setup

### Release Not Created

If the workflow succeeds but no release is created:

1. **Check permissions**: Ensure `GITHUB_TOKEN` has `contents: write`
2. **Verify tag format**: Must match pattern `**[0-9]+.[0-9]+.[0-9]+*`
3. **Check workflow conditions**: Ensure all jobs completed successfully

### Installer Issues

If installers don't work:

1. **Test locally**: Run `cargo dist build` and test generated installer
2. **Check URLs**: Verify installer URLs in README are correct
3. **Platform compatibility**: Ensure target platforms are supported

## Configuration

The release automation is configured in `Cargo.toml`:

```toml
[workspace.metadata.dist]
cargo-dist-version = "0.28.0"
ci = "github"
installers = ["shell", "powershell"]
targets = [
  "aarch64-apple-darwin",
  "x86_64-apple-darwin", 
  "x86_64-unknown-linux-gnu",
  "aarch64-unknown-linux-gnu",
  "x86_64-pc-windows-msvc"
]
pr-run-mode = "plan"
install-updater = false
```

## Emergency Procedures

### Rollback a Release

If a release has critical issues:

1. **Mark as pre-release** in GitHub UI (if just published)
2. **Create hotfix release**:
   ```bash
   # Fix the issue, then:
   git tag v0.2.1
   git push origin v0.2.1
   ```
3. **Update documentation** to point to the fixed version

### Manual Release

If automation fails completely:

1. **Build locally**:
   ```bash
   cargo dist build --tag vX.Y.Z --artifacts=all
   ```
2. **Create GitHub release manually**
3. **Upload artifacts** from `target/distrib/`

## Maintenance

### Updating cargo-dist

To update cargo-dist version:

1. **Update in Cargo.toml**:
   ```toml
   cargo-dist-version = "0.29.0"  # New version
   ```
2. **Regenerate workflow**:
   ```bash
   cargo dist generate-ci
   ```
3. **Test and commit changes**

### Adding New Platforms

To add support for new platforms:

1. **Add target to Cargo.toml**:
   ```toml
   targets = [
     # ... existing targets ...
     "new-target-triple"
   ]
   ```
2. **Regenerate workflow**: `cargo dist generate-ci`
3. **Test build**: `cargo dist build`

## Resources

- [cargo-dist documentation](https://opensource.axo.dev/cargo-dist/)
- [GitHub Actions documentation](https://docs.github.com/en/actions)
- [Rust target platform list](https://doc.rust-lang.org/nightly/rustc/platform-support.html) 