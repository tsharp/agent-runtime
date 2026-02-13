<#
.SYNOPSIS
    Calculates semantic version from git history.

.DESCRIPTION
    Version is determined by:
    1. If on a tag (v*) -> use tag version exactly
    2. If on release/v* branch -> {major}.{minor}.{commits-since-branch-or-tag}
    3. Otherwise -> {latest-tag-version}.{commits-since-tag}-dev

    With -TagOnly mode:
    - Uses pure tag-based versioning (no branch logic)
    - Base version is always 0.1.0 minimum
    - All tags must increment beyond previous tags
    - Non-tagged commits get {latest-tag}.{commits}-dev suffix

.EXAMPLE
    ./scripts/Get-Version.ps1
    
.EXAMPLE
    ./scripts/Get-Version.ps1 -UpdateCargo

.EXAMPLE
    ./scripts/Get-Version.ps1 -TagOnly
#>

param(
    [switch]$UpdateCargo,
    [switch]$Quiet,
    [switch]$TagOnly
)

$ErrorActionPreference = "Stop"

# Minimum base version for tag-only mode
$script:MinBaseVersion = "0.1.0"

function Compare-SemVer {
    param(
        [string]$Version1,
        [string]$Version2
    )
    
    # Strip 'v' prefix and any suffix like -dev, -alpha, etc.
    $v1Clean = $Version1 -replace '^v', '' -replace '-.*$', ''
    $v2Clean = $Version2 -replace '^v', '' -replace '-.*$', ''
    
    $parts1 = $v1Clean -split '\.' | ForEach-Object { [int]$_ }
    $parts2 = $v2Clean -split '\.' | ForEach-Object { [int]$_ }
    
    # Pad arrays to 3 elements
    while ($parts1.Count -lt 3) { $parts1 += 0 }
    while ($parts2.Count -lt 3) { $parts2 += 0 }
    
    for ($i = 0; $i -lt 3; $i++) {
        if ($parts1[$i] -gt $parts2[$i]) { return 1 }
        if ($parts1[$i] -lt $parts2[$i]) { return -1 }
    }
    return 0
}

function Get-GitVersionTagOnly {
    # Check if we're in a git repo
    if (-not (Test-Path ".git")) {
        $gitRoot = git rev-parse --show-toplevel 2>$null
        if (-not $gitRoot) {
            throw "Not in a git repository"
        }
        Push-Location $gitRoot
        $popLocation = $true
    }

    try {
        # Get current branch/ref
        $currentRef = git rev-parse --abbrev-ref HEAD 2>$null
        if ($currentRef -eq "HEAD") {
            $currentRef = git describe --tags --exact-match 2>&1 | Where-Object { $_ -notmatch '^fatal:' }
            if (-not $currentRef) { $currentRef = "HEAD" }
        }

        # Get all version tags sorted
        $allTags = git tag -l "v*" 2>$null | Where-Object { $_ -match '^v?\d+\.\d+(\.\d+)?$' } | Sort-Object { 
            $v = $_ -replace '^v', ''
            $parts = $v -split '\.'
            [int]$parts[0] * 10000 + [int]$parts[1] * 100 + $(if ($parts[2]) { [int]$parts[2] } else { 0 })
        }

        # Check if we're exactly on a tag
        $exactTag = git describe --tags --exact-match 2>&1 | Where-Object { $_ -notmatch '^fatal:' }
        if ($exactTag -and $exactTag -match '^v?(\d+)\.(\d+)\.?(\d*)(.*)$') {
            $major = [int]$Matches[1]
            $minor = [int]$Matches[2]
            $patch = if ($Matches[3]) { [int]$Matches[3] } else { 0 }
            $suffix = $Matches[4]
            
            $version = "$major.$minor.$patch$suffix"
            
            # Validate version is >= minimum
            if ((Compare-SemVer $version $script:MinBaseVersion) -lt 0) {
                throw "Tag version $version is below minimum allowed version $script:MinBaseVersion"
            }
            
            # Validate this tag is greater than all previous tags
            if ($allTags) {
                $tagIndex = [array]::IndexOf($allTags, $exactTag)
                if ($tagIndex -gt 0) {
                    $previousTag = $allTags[$tagIndex - 1]
                    if ((Compare-SemVer $exactTag $previousTag) -le 0) {
                        throw "Tag $exactTag must be greater than previous tag $previousTag"
                    }
                }
            }
            
            return @{
                Version = $version
                IsTag = $true
                IsPrerelease = $version -match '-'
                Branch = $currentRef
                CommitsSinceTag = 0
                LatestTag = $exactTag
            }
        }

        # Get latest tag
        $latestTag = git describe --tags --abbrev=0 2>&1 | Where-Object { $_ -notmatch '^fatal:' }
        $commitsSinceTag = 0
        $baseVersion = $script:MinBaseVersion

        if ($latestTag -and $latestTag -match '^v?(\d+)\.(\d+)\.?(\d*)(.*)$') {
            $major = [int]$Matches[1]
            $minor = [int]$Matches[2]
            $patch = if ($Matches[3]) { [int]$Matches[3] } else { 0 }
            
            $tagVersion = "$major.$minor.$patch"
            
            # Use the greater of tag version or minimum base version
            if ((Compare-SemVer $tagVersion $script:MinBaseVersion) -ge 0) {
                $baseVersion = $tagVersion
            }
            
            $commitsSinceTag = [int](git rev-list --count "$latestTag..HEAD" 2>$null)
        }

        # Calculate version: base + commits since tag as patch increment
        if ($commitsSinceTag -gt 0) {
            $parts = $baseVersion -split '\.'
            $newPatch = [int]$parts[2] + $commitsSinceTag
            $version = "$($parts[0]).$($parts[1]).$newPatch-dev"
        } else {
            $version = "$baseVersion-dev"
        }

        return @{
            Version = $version
            IsTag = $false
            IsPrerelease = $true
            Branch = $currentRef
            CommitsSinceTag = $commitsSinceTag
            LatestTag = $latestTag
        }
    }
    finally {
        if ($popLocation) {
            Pop-Location
        }
    }
}

function Get-GitVersion {
    # Check if we're in a git repo
    if (-not (Test-Path ".git")) {
        $gitRoot = git rev-parse --show-toplevel 2>$null
        if (-not $gitRoot) {
            throw "Not in a git repository"
        }
        Push-Location $gitRoot
        $popLocation = $true
    }

    try {
        # Get current branch/ref
        $currentRef = git rev-parse --abbrev-ref HEAD 2>$null
        if ($currentRef -eq "HEAD") {
            # Detached HEAD - check if we're on a tag
            $currentRef = git describe --tags --exact-match 2>&1 | Where-Object { $_ -notmatch '^fatal:' }
            if (-not $currentRef) { $currentRef = "HEAD" }
        }

        # Check if we're exactly on a tag
        $exactTag = git describe --tags --exact-match 2>&1 | Where-Object { $_ -notmatch '^fatal:' }
        if ($exactTag -and $exactTag -match '^v?(\d+)\.(\d+)\.(\d+.*)$') {
            $tagMajor = [int]$Matches[1]
            $tagMinor = [int]$Matches[2]
            $version = "$($Matches[1]).$($Matches[2]).$($Matches[3])"
            
            # Validate tag matches release branch if on one
            $branchName = git rev-parse --abbrev-ref HEAD 2>$null
            if ($branchName -match '^release/v?(\d+)\.(\d+)$') {
                $branchMajor = [int]$Matches[1]
                $branchMinor = [int]$Matches[2]
                
                if ($tagMajor -ne $branchMajor -or $tagMinor -ne $branchMinor) {
                    throw "Tag version v$tagMajor.$tagMinor.x does not match release branch version v$branchMajor.$branchMinor"
                }
            }
            
            return @{
                Version = $version
                IsTag = $true
                IsPrerelease = $version -match '-'
                Branch = $currentRef
                CommitsSinceTag = 0
            }
        }

        # Get latest tag
        $latestTag = git describe --tags --abbrev=0 2>&1 | Where-Object { $_ -notmatch '^fatal:' }
        $commitsSinceTag = 0
        $baseVersion = "0.1.0"

        if ($latestTag -and $latestTag -match '^v?(\d+)\.(\d+)\.?(\d*)(.*)$') {
            $major = [int]$Matches[1]
            $minor = [int]$Matches[2]
            $patch = if ($Matches[3]) { [int]$Matches[3] } else { 0 }
            $suffix = $Matches[4]
            
            $baseVersion = "$major.$minor.$patch"
            $commitsSinceTag = [int](git rev-list --count "$latestTag..HEAD" 2>$null)
        }

        # Check if on a release branch
        if ($currentRef -match '^release/v?(\d+)\.(\d+)$') {
            $branchMajor = [int]$Matches[1]
            $branchMinor = [int]$Matches[2]
            
            # Validate latest tag matches branch if it exists
            if ($latestTag -and $latestTag -match '^v?(\d+)\.(\d+)') {
                $tagMajor = [int]$Matches[1]
                $tagMinor = [int]$Matches[2]
                
                if ($tagMajor -ne $branchMajor -or $tagMinor -ne $branchMinor) {
                    throw "Latest tag v$tagMajor.$tagMinor.x does not match release branch version v$branchMajor.$branchMinor"
                }
            }
            
            # Count commits since branching from main (or since the branch start)
            $mergeBase = git merge-base HEAD origin/main 2>$null
            if (-not $mergeBase) {
                $mergeBase = git merge-base HEAD main 2>$null
            }
            
            if ($mergeBase) {
                $commitsSinceBranch = [int](git rev-list --count "$mergeBase..HEAD" 2>$null)
            } else {
                $commitsSinceBranch = $commitsSinceTag
            }

            # Use greater of commits since tag or commits since branch
            $patchVersion = [Math]::Max($commitsSinceTag, $commitsSinceBranch)
            
            $version = "$branchMajor.$branchMinor.$patchVersion"
            
            return @{
                Version = $version
                IsTag = $false
                IsPrerelease = $false
                Branch = $currentRef
                CommitsSinceTag = $commitsSinceTag
            }
        }

        # Default: base version + commits + dev suffix
        if ($commitsSinceTag -gt 0) {
            # Bump patch and add dev suffix
            $parts = $baseVersion -split '\.'
            $newPatch = [int]$parts[2] + $commitsSinceTag
            $version = "$($parts[0]).$($parts[1]).$newPatch-dev"
        } else {
            $version = "$baseVersion-dev"
        }

        return @{
            Version = $version
            IsTag = $false
            IsPrerelease = $true
            Branch = $currentRef
            CommitsSinceTag = $commitsSinceTag
        }
    }
    finally {
        if ($popLocation) {
            Pop-Location
        }
    }
}

function Update-CargoVersion {
    param([string]$Version)
    
    # Get the repo root directory
    $repoRoot = git rev-parse --show-toplevel 2>$null
    if (-not $repoRoot) {
        # Fall back to parent of scripts directory
        $repoRoot = Split-Path -Parent $PSScriptRoot
    }
    if (-not $repoRoot) {
        $repoRoot = $PWD.Path
    }
    
    $cargoFiles = @(
        (Join-Path $repoRoot "Cargo.toml"),
        (Join-Path $repoRoot "crates" "model-context-protocol-macros" "Cargo.toml")
    )

    foreach ($file in $cargoFiles) {
        if (Test-Path $file) {
            $content = Get-Content $file -Raw
            # Update package version
            $content = $content -replace '(?m)^version = "[^"]*"', "version = `"$Version`""
            # Update model-context-protocol-macros dependency version (if version exists)
            $content = $content -replace 'model-context-protocol-macros = \{ version = "[^"]*"', "model-context-protocol-macros = { version = `"$Version`""
            # Add version to model-context-protocol-macros dependency (if version doesn't exist)
            $content = $content -replace 'model-context-protocol-macros = \{ path =', "model-context-protocol-macros = { version = `"$Version`", path ="
            [System.IO.File]::WriteAllText($file, $content)
            if (-not $Quiet) {
                Write-Host "Updated $file to version $Version" -ForegroundColor Green
            }
        } else {
            if (-not $Quiet) {
                Write-Host "Warning: $file not found" -ForegroundColor Yellow
            }
        }
    }
}

# Main
if ($TagOnly) {
    $result = Get-GitVersionTagOnly
} else {
    $result = Get-GitVersion
}

if (-not $Quiet) {
    Write-Host ""
    Write-Host "Git Version Info:" -ForegroundColor Cyan
    Write-Host "  Mode:         $(if ($TagOnly) { 'Tag-Only' } else { 'Standard' })" -ForegroundColor Gray
    Write-Host "  Version:      $($result.Version)" -ForegroundColor White
    Write-Host "  Branch:       $($result.Branch)" -ForegroundColor Gray
    Write-Host "  Is Tag:       $($result.IsTag)" -ForegroundColor Gray
    Write-Host "  Is Prerelease: $($result.IsPrerelease)" -ForegroundColor Gray
    Write-Host "  Commits Since Tag: $($result.CommitsSinceTag)" -ForegroundColor Gray
    if ($TagOnly -and $result.LatestTag) {
        Write-Host "  Latest Tag:   $($result.LatestTag)" -ForegroundColor Gray
    }
    Write-Host ""
}

if ($UpdateCargo) {
    Update-CargoVersion -Version $result.Version
}

# Output version for scripts
if ($Quiet) {
    Write-Output $result.Version
    exit 0
} else {
    return $result
}
