# Homebrew packaging

OziClock is distributed as a Homebrew cask because its release artifact is a macOS application bundle. The cask intentionally uses the upstream GitHub Release archive and does not require an Apple Developer certificate to be produced.

## Test the cask locally

Run these commands on an Apple Silicon Mac from the repository root:

```sh
brew install --cask ./packaging/homebrew/Casks/oziclock.rb
open -a OziClock
brew uninstall --cask oziclock
brew install --cask ./packaging/homebrew/Casks/oziclock.rb
brew uninstall --zap --cask oziclock
```

Releases produced by the current workflow are ad-hoc signed but not notarized. The cask is currently pinned to v2.0.8, which predates that workflow change; update it to the next release before publishing the tap. Current macOS and Homebrew releases may still require explicit user approval in Privacy & Security before the first launch.

## Publish from a personal tap

Create the public repository `ozinka/homebrew-tap`, copy `Casks/oziclock.rb` into its `Casks` directory, and push it. Users can then install OziClock with:

```sh
brew install --cask ozinka/tap/oziclock
```

For every OziClock release, update `version`, calculate the archive checksum with `shasum -a 256`, then run:

```sh
brew style --fix --cask oziclock
brew audit --new --cask oziclock
brew reinstall --cask oziclock
```

The official `homebrew/cask` repository additionally requires its current notability and Gatekeeper policies to be satisfied. The personal tap remains usable independently of acceptance into the official repository.
