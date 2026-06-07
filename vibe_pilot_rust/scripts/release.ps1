# Release Script for VibePilot

## Usage

```powershell
# Create a new release from master
.\scripts\release.ps1 -Version "1.1.0"

# Dry run (preview only, no git operations)
.\scripts\release.ps1 -Version "1.1.0" -DryRun

# Release from any branch
.\scripts\release.ps1 -Version "1.1.0" -FromBranch feature-branch
```

## Workflow

1. Creates a `release/v1.1.0` branch from the specified source branch
2. Squashes all commits into a single commit on the release branch
3. Bumps the version in Cargo.toml
4. Creates a git tag
5. Pushes everything and triggers the GitHub Actions release workflow

## Prerequisites

- Git
- Rust toolchain installed
- GitHub CLI (`gh`) installed and authenticated
- Access to push to the repository
