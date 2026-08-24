# Define major and minor versions manually here or read from file/env
$major = 1
$minor = 0

# Read last patch from file (or fallback to 0)
$patchFile = "version_patch.txt"
if (Test-Path $patchFile) {
  $patch = [int](Get-Content $patchFile)
} else {
  $patch = 0
}

# Increment patch for this build
$patch++

# Save new patch back to file
Set-Content $patchFile $patch

# Compose version strings
$version = "$major.$minor.$patch"
$assemblyVersion = "$major.$minor.$patch.0"
$fileVersion = $assemblyVersion
$informationalVersion = $version

# Paths and folder names
$tempOutput = "publish\temp"
$baseFolder = "oziclock.$version.win-x64"
$bundledFolder = "$baseFolder-bundled"
$baseOutput = "publish\$baseFolder"
$bundledOutput = "publish\$bundledFolder"
$baseZip = "publish\$baseFolder.zip"
$bundledZip = "publish\$bundledFolder.zip"

# Clean up
Remove-Item -Recurse -Force $tempOutput -ErrorAction SilentlyContinue
Remove-Item $baseZip -ErrorAction SilentlyContinue
Remove-Item $bundledZip -ErrorAction SilentlyContinue

# Publish base (no .NET bundled)
dotnet publish -c Release -r win-x64 `
  -p:Version=$version `
  -p:AssemblyVersion=$assemblyVersion `
  -p:FileVersion=$fileVersion `
  -p:InformationalVersion=$informationalVersion `
  --self-contained false `
  --output $tempOutput

# Rename to final base folder
Move-Item -Force $tempOutput $baseOutput

# Publish bundled (with .NET)
dotnet publish -c Release -r win-x64 `
  -p:Version=$version `
  -p:AssemblyVersion=$assemblyVersion `
  -p:FileVersion=$fileVersion `
  -p:InformationalVersion=$informationalVersion `
  --self-contained true `
  --output $tempOutput

# Rename to final bundled folder
Move-Item -Force $tempOutput $bundledOutput

# Create ZIPs
Compress-Archive -Path "$baseOutput\*" -DestinationPath $baseZip
Compress-Archive -Path "$bundledOutput\*" -DestinationPath $bundledZip

# Publish to GitHub
$tag = "v$version"
$releaseName = "OziClock $version"
$notes = "Release of OziClock $version with both self-contained and framework-dependent builds."

# Create Git tag (optional, only if not already tagged in Git)
git tag $tag
git push origin $tag

# Create GitHub release
gh release create $tag `
  "publish\oziclock.$version.win-x64.zip" `
  "publish\oziclock.$version.win-x64-bundled.zip" `
  --title "$releaseName" `
  --notes "$notes"
