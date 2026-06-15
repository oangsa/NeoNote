param(
    [switch]$Debug
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoToml = Join-Path $repoRoot "Cargo.toml"
$installerScript = Join-Path $repoRoot "installer\neonote.nsi"
$themeDir = Join-Path $repoRoot "assets\themes\built-in"
$profile = if ($Debug) { "debug" } else { "release" }
$cargoArgs = @("build")
if (-not $Debug) {
    $cargoArgs += "--release"
}

$versionLine = Select-String -Path $cargoToml -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
if (-not $versionLine) {
    throw "Could not read package version from Cargo.toml."
}

$version = $versionLine.Matches[0].Groups[1].Value
$targetExe = Join-Path $repoRoot "target\$profile\neonote.exe"
$outputDir = Join-Path $repoRoot "target\installer"

New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

Push-Location $repoRoot
try {
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE"
    }

    $makensis = (Get-Command makensis -ErrorAction Stop).Source
    & $makensis `
        "/DAPP_VERSION=$version" `
        "/DSOURCE_EXE=$targetExe" `
        "/DSOURCE_THEME_DIR=$themeDir" `
        "/DOUTPUT_DIR=$outputDir" `
        $installerScript

    if ($LASTEXITCODE -ne 0) {
        throw "makensis failed with exit code $LASTEXITCODE"
    }

    Write-Host "Installer created at $outputDir\NeoNote-Setup-$version.exe"
}
finally {
    Pop-Location
}
