param (
    [Parameter(Mandatory=$false)]
    [string]$Version,
    [switch]$DryRun,
    [string]$FromBranch = "dev"
)

# Redirect to create_release.ps1
if ([string]::IsNullOrEmpty($Version)) {
    & "$PSScriptRoot/create_release.ps1" -DryRun:$DryRun -FromBranch $FromBranch
} else {
    & "$PSScriptRoot/create_release.ps1" -Version $Version -DryRun:$DryRun -FromBranch $FromBranch
}
