# Microsoft Store MSIX Packaging

`scripts/package-windows-msix.ps1` builds the x64 release executable and
creates and verifies an unsigned MSIX package for the OziClock Microsoft Store
identity. The Microsoft Store signs packages that it distributes; this script
does not create or require a developer certificate.

Run from the repository root on Windows with the Windows SDK installed:

```powershell
.\scripts\package-windows-msix.ps1
```

The output is placed under `target/msix/`. The generated `.msixupload` can be
uploaded to Partner Center manually; the `.msix` is used by automated releases. The
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

## Automated Store submissions

After all builds and quality checks succeed and a tag-triggered GitHub Release
is published, the Release workflow calls `store-publish.yml`. The Windows job
downloads the MSIX from that same run and submits it to OziClock's existing
Store listing (`9NBG48TV6NP3`) using Microsoft's Store CLI. GitHub release
success and Store certification are separate: Microsoft reviews the submission
and Partner Center controls when it goes live. The CLI is currently documented
by Microsoft for free products.

One-time setup (never commit credentials):

1. Associate an Entra tenant with Partner Center. Register an Entra application
   and add it under Partner Center user management with the Manager role. See
   [Microsoft's GitHub Actions setup](https://learn.microsoft.com/en-us/windows/apps/publish/msstore-dev-cli/github-actions).
2. In GitHub repository Settings → Secrets and variables → Actions, add
   `PARTNER_CENTER_TENANT_ID`, `PARTNER_CENTER_CLIENT_ID`,
   `PARTNER_CENTER_CLIENT_SECRET`, and `PARTNER_CENTER_SELLER_ID`. The client
   secret is the secret value, not its ID. Renew it before expiration.
3. Push the workflow changes to the default branch. A future tag must advance
   the MSIX major, minor, or patch beyond every previously submitted Store
   version; prerelease suffixes do not change the Store version.

The Store job fails if credentials or API access are missing, the package is
ambiguous, or Partner Center reports a pending submission. This check protects
an existing draft because `msstore publish` can otherwise replace it. Avoid
editing or creating Store submissions while the job is running: the check and
submission are separate operations. If the job fails, inspect Partner Center
before rerunning the failed job; a failed attempt may have created a draft.
Microsoft warns against editing an API-created submission in Partner Center and
then continuing it via the API. Do not retry while another submission is in
certification. The GitHub Release remains available if the Store job fails.

The workflow and credentials have not yet been verified with a real automated
Store submission. The existing manual `.msixupload` process remains available.
