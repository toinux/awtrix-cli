[CmdletBinding()]
param(
    [string]$Version,
    [string]$InstallDir
)
$ErrorActionPreference = 'Stop'
$repo = 'toinux/awtrix-cli'

if (Get-Command awtrix-cli -ErrorAction SilentlyContinue) {
    $existing = & awtrix-cli --version 2>$null
    if ($LASTEXITCODE -eq 0 -and $existing -match '^awtrix-cli\s') {
        & awtrix-cli --help *> $null
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        Write-Host 'awtrix-cli is already installed; using the existing command.'
        Write-Host $existing
        exit 0
    }
}

if ($Version -and $Version -notmatch '^v[0-9]+\.[0-9]+\.[0-9]+$') {
    throw "Invalid version tag '$Version' (expected vMAJOR.MINOR.PATCH)."
}
$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
if ($env:OS -ne 'Windows_NT' -or $arch -ne [System.Runtime.InteropServices.Architecture]::X64) {
    throw "Unsupported host Windows/$arch. This release supports Windows x86_64 only; use the documented Cargo build path."
}
$asset = 'awtrix-cli-x86_64-pc-windows-msvc.exe'
if (-not $InstallDir) {
    $InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\awtrix-cli\bin'
}

$temp = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString('N'))
$staged = Join-Path $InstallDir ('.awtrix-cli.new.' + [Guid]::NewGuid().ToString('N') + '.exe')
$destination = Join-Path $InstallDir 'awtrix-cli.exe'
New-Item -ItemType Directory -Path $temp | Out-Null
try {
    if (-not $Version) {
        # REST provides the stable latest tag directly, avoiding redirect API differences.
        $latest = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest" -Headers @{ 'User-Agent' = 'awtrix-cli-installer' } -TimeoutSec 30
        $tag = [string]$latest.tag_name
        if ($tag -notmatch '^v[0-9]+\.[0-9]+\.[0-9]+$') {
            throw "Could not resolve a concrete stable release tag (got '$tag')."
        }
        $Version = $tag
    }
    $base = "https://github.com/$repo/releases/download/$Version"
    $binary = Join-Path $temp $asset
    $sums = Join-Path $temp 'SHA256SUMS'
    Invoke-WebRequest -Uri "$base/$asset" -OutFile $binary -UseBasicParsing -TimeoutSec 120
    Invoke-WebRequest -Uri "$base/SHA256SUMS" -OutFile $sums -UseBasicParsing -TimeoutSec 60
    $line = Get-Content -LiteralPath $sums | Where-Object { $_ -match ('^([0-9a-fA-F]{64})\s+\*?' + [regex]::Escape($asset) + '$') } | Select-Object -First 1
    if (-not $line) { throw "SHA256SUMS has no valid entry for $asset." }
    $expected = [regex]::Match($line, '^([0-9a-fA-F]{64})').Groups[1].Value.ToLowerInvariant()
    $actual = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) { throw "SHA-256 verification failed for $asset; existing installation was not changed." }

    # Stage and execute beside the destination before replacing a working install.
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Copy-Item -LiteralPath $binary -Destination $staged -Force
    $reportedVersion = (& $staged --version | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Downloaded binary failed --version; installation was not changed.' }
    $expectedVersion = "awtrix-cli $($Version.TrimStart('v'))"
    if ($reportedVersion -ne $expectedVersion) { throw "Unexpected binary version '$reportedVersion' (expected '$expectedVersion'); installation was not changed." }
    & $staged --help *> $null
    if ($LASTEXITCODE -ne 0) { throw 'Downloaded binary failed --help; installation was not changed.' }
    if (Test-Path -LiteralPath $destination) {
        [IO.File]::Replace($staged, $destination, $null)
    } else {
        [IO.File]::Move($staged, $destination)
    }
    Write-Host "Installed awtrix-cli $Version at $destination"
    Write-Host "Add $InstallDir to PATH for this shell: `$env:PATH = `"$InstallDir;`$env:PATH`""
    Write-Host $reportedVersion
} finally {
    Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $staged -Force -ErrorAction SilentlyContinue
}
