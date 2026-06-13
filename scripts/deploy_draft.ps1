param (
    [Parameter(Mandatory=$false)]
    [string]$Version = "draft"
)

$tagVersion = "v$($Version.TrimStart('v'))"

Write-Host "Preparing draft deployment for tag: $tagVersion..."

# 1. Compile both binaries in release mode
Write-Host "Compiling Worker and Master in release mode..."
cd vibe_pilot_rust
cargo build --release
cd ..

# 2. Package binaries in dist folder
$distDir = "vibe_pilot_rust/target/release/dist"
if (-not (Test-Path $distDir)) {
    New-Item -ItemType Directory -Path $distDir | Out-Null
}

Copy-Item "vibe_pilot_rust/target/release/vibepilot.exe" "$distDir/vibepilot.exe" -Force
Copy-Item "vibe_pilot_rust/target/release/vibepilot_master.exe" "$distDir/vibepilot_master.exe" -Force

Write-Host "Release binaries compiled successfully in: $distDir"

# 2.5 Create Release ZIP package
$plainVersion = $Version.TrimStart('v')
$releaseZipDir = "vibe_pilot_rust/release-zip"
if (-not (Test-Path $releaseZipDir)) {
    New-Item -ItemType Directory -Path $releaseZipDir | Out-Null
}

$stagingDirName = "vibepilot-v$plainVersion-windows-x64"
$stagingPath = "$releaseZipDir/$stagingDirName"

Write-Host "Packaging release ZIP in $stagingPath..."

# Clean old staging dir if exists
if (Test-Path $stagingPath) {
    Remove-Item -Recurse -Force $stagingPath
}
New-Item -ItemType Directory -Path $stagingPath | Out-Null

# Copy binaries
Copy-Item "vibe_pilot_rust/target/release/vibepilot.exe" "$stagingPath/vibepilot.exe" -Force
Copy-Item "vibe_pilot_rust/target/release/vibepilot_master.exe" "$stagingPath/vibepilot_master.exe" -Force

# Copy and rename backup configurations
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
# VibePilot v$plainVersion

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

# Compress to ZIP
$zipPath = "$releaseZipDir/$stagingDirName.zip"
if (Test-Path $zipPath) {
    Remove-Item -Force $zipPath
}
Write-Host "Compressing to $zipPath..."
Compress-Archive -Path "$stagingPath/*" -DestinationPath $zipPath -Force

# 3. Create Draft Release using GitHub CLI (gh)
if (Get-Command gh -ErrorAction SilentlyContinue) {
    Write-Host "Creating GitHub Draft Release via 'gh' CLI..."
    gh release create $tagVersion --draft --title "Draft Release $tagVersion" "$distDir/vibepilot.exe" "$distDir/vibepilot_master.exe" "$zipPath" --notes "Draft release assets for Worker and Master."
    Write-Host "Draft Release created successfully on GitHub!"
} else {
    Write-Warning "GitHub CLI ('gh') not found in PATH. You can manually upload the binaries from: $distDir"
}
