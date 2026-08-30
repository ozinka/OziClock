cask "oziclock" do
  version "2.0.8"
  sha256 "65736abcb7a818d898b9fcc877b82fef3a1192c0e55a2ebcd80f30f098ce6d0d"

  url "https://github.com/ozinka/OziClock/releases/download/v#{version}/OziClock-v#{version}-macos-arm64.tar.gz"
  name "OziClock"
  desc "Desktop world clock for viewing multiple time zones"
  homepage "https://github.com/ozinka/OziClock"

  depends_on arch: :arm64
  depends_on macos: ">= :big_sur"

  app "OziClock.app"

  zap trash: "~/Library/Application Support/OziClock"
end
