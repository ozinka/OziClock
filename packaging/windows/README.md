# Microsoft Store MSIX Packaging

`scripts/package-windows-msix.ps1` builds the x64 release executable and
creates and verifies an unsigned MSIX package for the OziClock Microsoft Store
identity. The Microsoft Store signs packages that it distributes; this script
does not create or require a developer certificate.

Run from the repository root on Windows with the Windows SDK installed:

```powershell
.\scripts\package-windows-msix.ps1
```

The output is placed under `target/msix/`. Upload the generated `.msixupload`
file to Partner Center; the contained `.msix` is retained for inspection. The
script maps the Cargo version to the required Store MSIX version
`major.minor.patch.0`. Store packages must keep the revision component at zero,
so a Cargo prerelease suffix is intentionally omitted. Publish a later Store
update with a higher major, minor, or patch component. For a tagged release,
pass its version explicitly:

```powershell
.\scripts\package-windows-msix.ps1 -PackageVersion 2.2.2-beta.1
```

Upload the resulting `.msixupload` file in the product's **Packages** section
in Partner Center. The manifest identity is intentionally fixed to the reserved
OziClock Store product; do not use this package for another Store listing.
