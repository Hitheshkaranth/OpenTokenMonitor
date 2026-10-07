cask "opentokenmonitor" do
  arch arm: "arm64", intel: "x86_64"

  version "0.4.1"
  sha256 arm:   "43545ac13f7170b3f81ce3ec6eb4effaa185dddce0aa4ac703b8881fb3d8423d",
         intel: "197533381384b5516ed93d46c66e1c9182d1422ffd7d1bc46796fc8c6ecd3953"

  url "https://github.com/Hitheshkaranth/OpenTokenMonitor/releases/download/v#{version}/OpenTokenMonitor_#{version}_#{arch}.dmg"
  name "OpenTokenMonitor"
  desc "Usage limit and cost monitor for Claude Code, Codex and Antigravity"
  homepage "https://github.com/Hitheshkaranth/OpenTokenMonitor"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on macos: ">= :catalina"

  app "OpenTokenMonitor.app"

  zap trash: [
    "~/Library/Application Support/com.opentokenmonitor.desktop",
    "~/Library/Application Support/OpenTokenMonitor",
    "~/Library/Caches/com.opentokenmonitor.desktop",
    "~/Library/LaunchAgents/OpenTokenMonitor.plist",
    "~/Library/Preferences/com.opentokenmonitor.desktop.plist",
    "~/Library/Saved Application State/com.opentokenmonitor.desktop.savedState",
    "~/Library/WebKit/com.opentokenmonitor.desktop",
  ]
end
