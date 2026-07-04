require "json"
require "./tool_http"
require "./version_compare"
require "./version"

module ReleaseCheck
  GITHUB_REPO           = "Aspenini/quark-downloader"
  INSTALLER_NAME_PREFIX = "quark-downloader"
  LATEST_URL            = "https://api.github.com/repos/#{GITHUB_REPO}/releases/latest"

  enum Status
    UpToDate
    Ahead
    Behind
    Failed
  end

  record BehindInfo, latest_tag : String, latest_version : String, download_url : String

  def self.installer_download_url(tag_name : String) : String
    version = tag_name.lstrip("v")
    "https://github.com/#{GITHUB_REPO}/releases/download/#{tag_name}/#{INSTALLER_NAME_PREFIX}-#{version}-setup.exe"
  end

  def self.normalize_tag(tag_name : String) : String
    tag_name.lstrip("v")
  end

  def self.status_from_release(release : JSON::Any, installed : String = QuarkVersion::VERSION) : {Status, String?, BehindInfo?}
    tag = release["tag_name"].as_s
    latest = normalize_tag(tag)
    cmp = VersionCompare.compare(installed, latest)

    case cmp
    when 0
      {Status::UpToDate, latest, nil}
    when 1
      {Status::Ahead, latest, nil}
    else
      behind = BehindInfo.new(
        latest_tag: tag,
        latest_version: latest,
        download_url: installer_download_url(tag),
      )
      {Status::Behind, latest, behind}
    end
  end

  def self.check : {Status, String?, BehindInfo?}
    release = fetch_latest_release
    status_from_release(release)
  rescue ex
    {Status::Failed, nil, nil}
  end

  def self.failed_message(ex : Exception) : String
    msg = ex.message
    return ex.class.name if msg.nil? || msg.empty?
    msg
  end

  def self.check_with_error : {Status, String?, BehindInfo?, String?}
    release = fetch_latest_release
    status, latest, behind = status_from_release(release)
    {status, latest, behind, nil}
  rescue ex
    {Status::Failed, nil, nil, failed_message(ex)}
  end

  def self.fetch_latest_release : JSON::Any
    JSON.parse(ToolHttp.fetch_body(LATEST_URL))
  end
end
