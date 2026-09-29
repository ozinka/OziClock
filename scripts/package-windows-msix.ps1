[CmdletBinding()]
param(
    [string]$PackageVersion
)

$ErrorActionPreference = 'Stop'

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$manifestTemplate = Join-Path $repositoryRoot 'packaging\windows\msix\AppxManifest.xml'
$applicationBinary = Join-Path $repositoryRoot 'target\release\oziclock-desktop.exe'
$iconPath = Join-Path $repositoryRoot 'legacy\dotnet-wpf\Ozi.Clock\Assets\clock.ico'
$stagingDirectory = Join-Path $repositoryRoot 'target\msix\staging'
$packageDirectory = Join-Path $repositoryRoot 'target\msix'
$validationDirectory = Join-Path $repositoryRoot 'target\msix\validation'

function ConvertTo-MsixVersion([string]$version) {
    if ($version -notmatch '^(?<major>\d+)\.(?<minor>\d+)\.(?<patch>\d+)(?:-[^.]+\.(?<revision>\d+))?$') {
        throw "Cannot convert '$version' to an MSIX four-part version. Pass -PackageVersion explicitly."
    }

    return "$($Matches.major).$($Matches.minor).$($Matches.patch).0"
}

function Find-WindowsSdkTool([string]$toolName) {
    $sdkBinRoot = 'C:\Program Files (x86)\Windows Kits\10\bin'
    $tool = Get-ChildItem -LiteralPath $sdkBinRoot -Directory |
        Sort-Object Name -Descending |
        ForEach-Object { Join-Path $_.FullName "x64\$toolName" } |
        Where-Object { Test-Path -LiteralPath $_ } |
        Select-Object -First 1
    if (-not $tool) {
        throw "Windows SDK tool '$toolName' was not found under '$sdkBinRoot'. Install the Windows SDK included with Desktop development with C++."
    }
    return $tool
}

function Write-Logo([string]$destination, [int]$width, [int]$height) {
    $icon = [System.Drawing.Icon]::new($iconPath, 64, 64)
    $source = $icon.ToBitmap()
    $bitmap = [System.Drawing.Bitmap]::new($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.Clear([System.Drawing.Color]::Transparent)
        $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $graphics.DrawImage($source, 0, 0, $width, $height)
        $bitmap.Save($destination, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally {
        $graphics.Dispose()
        $bitmap.Dispose()
        $source.Dispose()
        $icon.Dispose()
    }
}

if (-not $PackageVersion) {
    $cargoManifest = Get-Content (Join-Path $repositoryRoot 'apps\oziclock-desktop\Cargo.toml') -Raw
    if ($cargoManifest -notmatch '(?m)^version\s*=\s*"(?<version>[^"]+)"') {
        throw 'Could not read the desktop package version from Cargo.toml.'
    }
    $PackageVersion = $Matches.version
}
$msixVersion = ConvertTo-MsixVersion $PackageVersion

& cargo build --release -p oziclock-desktop
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

if (-not (Test-Path -LiteralPath $applicationBinary)) {
    throw "Release binary was not produced at '$applicationBinary'."
}

if (Test-Path -LiteralPath $stagingDirectory) {
    Remove-Item -LiteralPath $stagingDirectory -Recurse -Force
}
New-Item -ItemType Directory -Path (Join-Path $stagingDirectory 'Assets') -Force | Out-Null
Copy-Item -LiteralPath $applicationBinary -Destination $stagingDirectory
Copy-Item -LiteralPath (Join-Path $repositoryRoot 'LICENSE') -Destination $stagingDirectory
Copy-Item -LiteralPath (Join-Path $repositoryRoot 'ui\fonts\OFL.txt') -Destination $stagingDirectory

$manifest = (Get-Content -LiteralPath $manifestTemplate -Raw).Replace('__PACKAGE_VERSION__', $msixVersion)
Set-Content -LiteralPath (Join-Path $stagingDirectory 'AppxManifest.xml') -Value $manifest -NoNewline -Encoding utf8

Add-Type -AssemblyName System.Drawing
$assetsDirectory = Join-Path $stagingDirectory 'Assets'
Write-Logo (Join-Path $assetsDirectory 'StoreLogo.png') 50 50
Write-Logo (Join-Path $assetsDirectory 'Square44x44Logo.png') 44 44
Write-Logo (Join-Path $assetsDirectory 'Square71x71Logo.png') 71 71
Write-Logo (Join-Path $assetsDirectory 'Square150x150Logo.png') 150 150
Write-Logo (Join-Path $assetsDirectory 'Square310x310Logo.png') 310 310
Write-Logo (Join-Path $assetsDirectory 'Wide310x150Logo.png') 310 150

$makeAppx = Find-WindowsSdkTool 'makeappx.exe'
$outputPackage = Join-Path $packageDirectory "OziClock-$msixVersion-windows-x64.msix"
$uploadPackage = Join-Path $packageDirectory "OziClock-$msixVersion-windows-x64.msixupload"
$uploadArchive = "$uploadPackage.zip"
New-Item -ItemType Directory -Path $packageDirectory -Force | Out-Null
& $makeAppx pack /d $stagingDirectory /p $outputPackage /o
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}
if (Test-Path -LiteralPath $validationDirectory) {
    Remove-Item -LiteralPath $validationDirectory -Recurse -Force
}
& $makeAppx unpack /p $outputPackage /d $validationDirectory /o
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

if (Test-Path -LiteralPath $uploadPackage) {
    Remove-Item -LiteralPath $uploadPackage -Force
}
if (Test-Path -LiteralPath $uploadArchive) {
    Remove-Item -LiteralPath $uploadArchive -Force
}
Compress-Archive -LiteralPath $outputPackage -DestinationPath $uploadArchive
Move-Item -LiteralPath $uploadArchive -Destination $uploadPackage
Write-Output "Created and verified $outputPackage"
Write-Output "Created Store upload package $uploadPackage"
