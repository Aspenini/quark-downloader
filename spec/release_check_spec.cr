require "spec"
require "json"
require "../src/version_compare"
require "../src/release_check"

describe VersionCompare do
  it "compares semver tuples" do
    VersionCompare.compare("0.3.0", "0.4.0").should eq(-1)
    VersionCompare.compare("0.4.0", "0.3.0").should eq(1)
    VersionCompare.compare("0.3.0", "0.3.0").should eq(0)
    VersionCompare.compare("v0.3.0", "0.3.0").should eq(0)
  end

  it "reports newer and at_least" do
    VersionCompare.newer?("0.4.0", "0.3.0").should be_true
    VersionCompare.at_least?("0.3.0", "0.3.0").should be_true
  end
end

describe ReleaseCheck do
  it "builds the predictable Windows installer URL" do
    ReleaseCheck.installer_download_url("v0.4.0").should eq(
      "https://github.com/Aspenini/quark-downloader/releases/download/v0.4.0/quark-downloader-0.4.0-setup.exe",
    )
  end

  it "classifies release status from fixture JSON" do
    release = JSON.parse(<<-JSON)
      {"tag_name":"v0.4.0"}
      JSON

    status, latest, behind = ReleaseCheck.status_from_release(release, "0.3.0")
    status.should eq(ReleaseCheck::Status::Behind)
    latest.should eq("0.4.0")
    info = behind.not_nil!
    info.download_url.should eq(ReleaseCheck.installer_download_url("v0.4.0"))

    status, latest, _ = ReleaseCheck.status_from_release(release, "0.4.0")
    status.should eq(ReleaseCheck::Status::UpToDate)
    latest.should eq("0.4.0")

    status, latest, _ = ReleaseCheck.status_from_release(release, "0.5.0")
    status.should eq(ReleaseCheck::Status::Ahead)
    latest.should eq("0.4.0")
  end
end
