param (
    [Parameter(Mandatory=$false)]
    [string]$Version,
    [switch]$DryRun,
    [string]$FromBranch = "dev"
)

# --- Step 0: Version Input ----------------------------------------------------

if ([string]::IsNullOrEmpty($Version)) {
    $latestTag = git describe --tags --abbrev=0 2>$null
    if (-not $latestTag) {
        $latestTag = "None (no tags found)"
    }
    Write-Host "Latest git tag: $latestTag" -ForegroundColor Cyan
    $Version = Read-Host "Please enter the release version (e.g. 1.0.2)"
    if ([string]::IsNullOrWhiteSpace($Version)) {
        Write-Error "Error: Release version is mandatory."
        exit 1
    }
}

$plainVersion = $Version.TrimStart('v')

# Validate version format: must be X.Y.Z with X, Y, Z as numbers
if ($plainVersion -notmatch '^\d+\.\d+\.\d+$') {
    Write-Error "Error: Invalid version format '$Version'. Expected X.Y.Z (e.g. 1.0.2) where X, Y, Z are numbers."
    exit 1
}

$tagVersion = "v$plainVersion"
$devTag = "dev/$tagVersion"
$releaseBranch = "release/$tagVersion"

Write-Host ""
Write-Host "=======================================================" -ForegroundColor Cyan
Write-Host "  VibePilot Release: $tagVersion" -ForegroundColor Cyan
Write-Host "=======================================================" -ForegroundColor Cyan
Write-Host ""

# --- Validate preconditions ---------------------------------------------------

$currentBranch = git rev-parse --abbrev-ref HEAD
if ($currentBranch -ne $FromBranch) {
    Write-Error "Error: You must be on the '$FromBranch' branch. Currently on '$currentBranch'."
    exit 1
}

$statusOutput = git status --porcelain
if ($statusOutput) {
    Write-Error "Error: Working tree is not clean. Commit or stash changes first."
    exit 1
}

# Find the latest release tag (vX.Y.Z format only, exclude dev/* tags)
$latestReleaseTag = git tag --sort=-v:refname | Where-Object { $_ -match '^v\d+\.\d+\.\d+$' } | Select-Object -First 1
if (-not $latestReleaseTag) {
    Write-Error "Error: No existing release tag found (vX.Y.Z format). Cannot determine base for squash."
    exit 1
}

Write-Host "  From branch : $FromBranch" -ForegroundColor White
Write-Host "  Base tag     : $latestReleaseTag" -ForegroundColor White
Write-Host "  Release tag  : $tagVersion" -ForegroundColor White
Write-Host "  Dev tag      : $devTag" -ForegroundColor White
Write-Host ""

if ($DryRun) {
    Write-Host "[DRY RUN] Would perform the following:" -ForegroundColor Yellow
    Write-Host "  1. Tag '$FromBranch' as '$devTag'"
    Write-Host "  2. Create branch '$releaseBranch' from '$latestReleaseTag'"
    Write-Host "  3. Squash merge '$FromBranch' into '$releaseBranch' (1 commit)"
    Write-Host "  4. Bump Cargo.toml to $plainVersion"
    Write-Host "  5. Build release binaries"
    Write-Host "  6. Package ZIP in release-zip/"
    Write-Host "  7. Tag release commit as '$tagVersion'"
    Write-Host "  8. Update 'latest' branch to point to '$releaseBranch'"
    Write-Host "  9. Push '$releaseBranch', '$tagVersion', '$devTag', 'latest' to origin"
    Write-Host " 10. Return to '$FromBranch'"
    exit 0
}

# --- Step 1: Tag the dev source commit ----------------------------------------

Write-Host "[1/8] Tagging '$FromBranch' as '$devTag'..." -ForegroundColor Green
git tag -a $devTag -m "Dev snapshot for release $tagVersion"
if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to create dev tag '$devTag'. It may already exist."
    exit 1
}

# --- Step 2: Create release branch from latest tag ----------------------------

Write-Host "[2/8] Creating branch '$releaseBranch' from '$latestReleaseTag'..." -ForegroundColor Green
git checkout -b $releaseBranch $latestReleaseTag
if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to create release branch."
    git tag -d $devTag  # Rollback dev tag
    exit 1
}

# --- Step 3: Import dev content into release branch ---------------------------

Write-Host "[3/8] Importing '$FromBranch' content into '$releaseBranch'..." -ForegroundColor Green

# Copy all files from dev (overwrites everything, no conflicts)
git checkout $FromBranch -- .

# Remove files that were deleted in dev since the last release
$diffOutput = git diff --name-status HEAD $FromBranch
$deletedFiles = $diffOutput | Where-Object { $_ -match '^D' } | ForEach-Object { ($_ -split '\t')[1] }
if ($deletedFiles) {
    Write-Host "  Removing $($deletedFiles.Count) deleted file(s)..."
    foreach ($f in $deletedFiles) {
        if (Test-Path $f) {
            git rm --quiet $f
        }
    }
}

# --- Step 4: Bump version in Cargo.toml ---------------------------------------

Write-Host "[4/8] Bumping Cargo.toml version to $plainVersion..." -ForegroundColor Green
$cargoPath = "vibe_pilot_rust/Cargo.toml"
if (Test-Path $cargoPath) {
    $content = Get-Content $cargoPath
    $bumped = $false
    $newContent = foreach ($line in $content) {
        if (-not $bumped -and $line -match '^version = ".*"') {
            $line = "version = `"$plainVersion`""
            $bumped = $true
        }
        $line
    }
    $newContent | Set-Content $cargoPath
    Write-Host "  Updated $cargoPath to version $plainVersion"
} else {
    Write-Error "Could not find Cargo.toml at $cargoPath"
    exit 1
}

# --- Step 5: Build release binaries -------------------------------------------

Write-Host "[5/8] Building release binaries..." -ForegroundColor Green
Push-Location vibe_pilot_rust
cargo build --release
$buildResult = $LASTEXITCODE
Pop-Location

if ($buildResult -ne 0) {
    Write-Error "Release build failed."
    exit 1
}

# Copy to dist
$distDir = "vibe_pilot_rust/target/release/dist"
if (-not (Test-Path $distDir)) {
    New-Item -ItemType Directory -Path $distDir | Out-Null
}
Copy-Item "vibe_pilot_rust/target/release/vibepilot.exe" "$distDir/vibepilot.exe" -Force
Copy-Item "vibe_pilot_rust/target/release/vibepilot_master.exe" "$distDir/vibepilot_master.exe" -Force

# --- Step 6: Package ZIP -----------------------------------------------------

Write-Host "[6/8] Packaging release ZIP..." -ForegroundColor Green
$releaseZipDir = "vibe_pilot_rust/release-zip"
if (-not (Test-Path $releaseZipDir)) {
    New-Item -ItemType Directory -Path $releaseZipDir | Out-Null
}

$stagingDirName = "vibepilot-$tagVersion-windows-x64"
$stagingPath = "$releaseZipDir/$stagingDirName"

if (Test-Path $stagingPath) {
    Remove-Item -Recurse -Force $stagingPath
}
New-Item -ItemType Directory -Path $stagingPath | Out-Null

# Copy binaries
Copy-Item "vibe_pilot_rust/target/release/vibepilot.exe" "$stagingPath/vibepilot.exe" -Force
Copy-Item "vibe_pilot_rust/target/release/vibepilot_master.exe" "$stagingPath/vibepilot_master.exe" -Force

# Copy backup configurations
if (Test-Path "vibe_pilot_rust/save.enc.bak") {
    Copy-Item "vibe_pilot_rust/save.enc.bak" "$stagingPath/save.enc" -Force
}
if (Test-Path "vibe_pilot_rust/engines.enc.bak") {
    Copy-Item "vibe_pilot_rust/engines.enc.bak" "$stagingPath/engines.enc" -Force
}
if (Test-Path "vibe_pilot_rust/key.enc") {
    Copy-Item "vibe_pilot_rust/key.enc" "$stagingPath/key.enc" -Force
}

# Copy profiles
$stagingProfiles = "$stagingPath/profiles"
New-Item -ItemType Directory -Path $stagingProfiles | Out-Null
if (Test-Path "vibe_pilot_rust/profiles") {
    Get-ChildItem "vibe_pilot_rust/profiles/*.enc.bak" | ForEach-Object {
        $destName = $_.Name.Replace(".enc.bak", ".enc")
        Copy-Item $_.FullName "$stagingProfiles/$destName" -Force
    }
}

# Write README.txt
$readmeContent = @"
# VibePilot $tagVersion

Visual AI Automation Orchestrator for Windows.

## Installation
1. Download and extract the ZIP archive.
2. Run vibepilot.exe (Worker/Client App) or vibepilot_master.exe (Master Grid UI) directly.

## Requirements
- Windows 10/11
- A local LLM server (LM Studio, Ollama, or OpenAI-compatible API)

## License
MIT
Full documentation: https://github.com/Hidr4c/VibePilot
"@
$readmeContent | Out-File -FilePath "$stagingPath/README.txt" -Encoding utf8

# Compress
$zipPath = "$releaseZipDir/$stagingDirName.zip"
if (Test-Path $zipPath) {
    Remove-Item -Force $zipPath
}
Write-Host "  Compressing to $zipPath..."
Compress-Archive -Path "$stagingPath/*" -DestinationPath $zipPath -Force

# --- Step 7: Commit & Tag ----------------------------------------------------

Write-Host "[7/8] Committing squashed release and tagging $tagVersion..." -ForegroundColor Green

# Stage everything (version bump + squash merge changes + Cargo.lock from build)
git add -A
git commit --no-verify -m "Release $tagVersion"
if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to commit squashed release."
    exit 1
}

git tag -a $tagVersion -m "Release $tagVersion"
if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to create release tag."
    exit 1
}

# --- Step 8: Update 'latest' and push ----------------------------------------

Write-Host "[8/8] Updating 'latest' branch and pushing to origin..." -ForegroundColor Green

# Update latest branch
$latestExists = git show-ref --verify --quiet refs/heads/latest 2>$null
$latestBranchExists = ($LASTEXITCODE -eq 0)
if ($latestBranchExists) {
    git checkout latest
    git reset --hard $releaseBranch
} else {
    git checkout -b latest $releaseBranch
}

# Push everything
Write-Host "  Pushing release branch..."
git push origin $releaseBranch
Write-Host "  Pushing release tag..."
git push origin $tagVersion
Write-Host "  Pushing dev tag..."
git push origin $devTag
Write-Host "  Pushing latest branch..."
git push origin latest --force

# --- Return to dev -----------------------------------------------------------

Write-Host ""
git checkout $FromBranch

Write-Host ""
Write-Host "=======================================================" -ForegroundColor Green
Write-Host "  [OK] Release $tagVersion created successfully!" -ForegroundColor Green
Write-Host "=======================================================" -ForegroundColor Green
Write-Host ""
Write-Host "  Summary:" -ForegroundColor White
Write-Host "    Dev tag      : $devTag (on '$FromBranch')" -ForegroundColor Gray
Write-Host "    Release tag  : $tagVersion (on '$releaseBranch')" -ForegroundColor Gray
Write-Host "    Branch       : $releaseBranch (1 squashed commit)" -ForegroundColor Gray
Write-Host "    Latest       : aligned to $releaseBranch" -ForegroundColor Gray
Write-Host "    ZIP          : $zipPath" -ForegroundColor Gray
Write-Host ""
