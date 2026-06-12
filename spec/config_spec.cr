require "spec"
require "../src/config"

describe QuarkConfig do
  it "parses all config fields" do
    path = Path[Dir.tempdir] / "quark-config-#{Time.utc.to_unix_ms}.conf"
    File.write(path.to_s, <<-CONF)
      download_dir = ~/Media
      yt_dlp = path
      ffmpeg = bundled
      gui_download_mode = external_cli
      download_logs = off
      gui_theme = dark
      strip_video_ids = false
      sanitize_filenames = false
      filename_spaces = underscore
      playlist_folders = false
      CONF

    settings = QuarkConfig.parse_file(path, quiet: true)

    settings.download_dir.should eq("~/Media")
    settings.yt_dlp.should eq(QuarkConfig::ToolSource::Path)
    settings.ffmpeg.should eq(QuarkConfig::ToolSource::Bundled)
    settings.gui_download_mode.should eq(QuarkConfig::GuiDownloadMode::ExternalCli)
    settings.download_logs.should be_false
    settings.gui_theme.should eq(QuarkConfig::GuiTheme::Dark)
    settings.strip_video_ids.should be_false
    settings.sanitize_filenames.should be_false
    settings.filename_spaces.should eq(QuarkConfig::FilenameSpaces::Underscore)
    settings.playlist_folders.should be_false
  ensure
    File.delete?(path.to_s) if path
  end

  it "falls back for invalid values" do
    path = Path[Dir.tempdir] / "quark-config-invalid-#{Time.utc.to_unix_ms}.conf"
    File.write(path.to_s, <<-CONF)
      yt_dlp = nope
      ffmpeg = wrong
      gui_download_mode = mystery
      download_logs = maybe
      gui_theme = neon
      strip_video_ids = maybe
      sanitize_filenames = perhaps
      filename_spaces = tabs
      playlist_folders = sometimes
      CONF

    settings = QuarkConfig.parse_file(path, quiet: true)

    settings.yt_dlp.should eq(QuarkConfig::ToolSource::Auto)
    settings.ffmpeg.should eq(QuarkConfig::ToolSource::Auto)
    settings.gui_download_mode.should eq(QuarkConfig::GuiDownloadMode::Progress)
    settings.download_logs.should be_true
    settings.gui_theme.should eq(QuarkConfig::GuiTheme::Light)
    settings.strip_video_ids.should be_true
    settings.sanitize_filenames.should be_true
    settings.filename_spaces.should eq(QuarkConfig::FilenameSpaces::Keep)
    settings.playlist_folders.should be_true
  ensure
    File.delete?(path.to_s) if path
  end

  it "renders the new public settings" do
    settings = QuarkConfig::Settings.new(
      download_dir: "D:/Downloads",
      yt_dlp: QuarkConfig::ToolSource::Bundled,
      ffmpeg: QuarkConfig::ToolSource::Path,
      gui_download_mode: QuarkConfig::GuiDownloadMode::ExternalCli,
      download_logs: false,
      gui_theme: QuarkConfig::GuiTheme::Dark,
      strip_video_ids: false,
      sanitize_filenames: false,
      filename_spaces: QuarkConfig::FilenameSpaces::Dash,
      playlist_folders: false,
    )

    rendered = QuarkConfig.render(settings)

    rendered.includes?("download_dir = D:/Downloads").should be_true
    rendered.includes?("yt_dlp = bundled").should be_true
    rendered.includes?("ffmpeg = path").should be_true
    rendered.includes?("gui_download_mode = external_cli").should be_true
    rendered.includes?("download_logs = false").should be_true
    rendered.includes?("gui_theme = dark").should be_true
    rendered.includes?("strip_video_ids = false").should be_true
    rendered.includes?("sanitize_filenames = false").should be_true
    rendered.includes?("filename_spaces = dash").should be_true
    rendered.includes?("playlist_folders = false").should be_true
  end

  it "appends missing public settings to older config files" do
    path = Path[Dir.tempdir] / "quark-config-migrate-#{Time.utc.to_unix_ms}.conf"
    File.write(path.to_s, <<-CONF)
      download_dir = ~/Downloads
      yt_dlp = auto
      ffmpeg = auto
      CONF

    settings = QuarkConfig.parse_file(path, quiet: true)
    keys = QuarkConfig.parse_file_with_keys(path, quiet: true)[1]
    QuarkConfig.append_missing_defaults!(path, settings, keys)

    migrated = File.read(path.to_s)
    migrated.includes?("download_dir = ~/Downloads").should be_true
    migrated.includes?("gui_download_mode = progress").should be_true
    migrated.includes?("download_logs = true").should be_true
    migrated.includes?("gui_theme = light").should be_true
    migrated.includes?("strip_video_ids = true").should be_true
    migrated.includes?("sanitize_filenames = true").should be_true
    migrated.includes?("filename_spaces = keep").should be_true
    migrated.includes?("playlist_folders = true").should be_true
  ensure
    File.delete?(path.to_s) if path
  end
end
