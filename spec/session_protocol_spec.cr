require "spec"
require "../src/gui/session_protocol"

describe QuarkGui::SessionProtocol do
  it "builds session args in protocol order" do
    settings = QuarkConfig::Settings.new(
      download_dir: "~/Videos",
      yt_dlp: QuarkConfig::ToolSource::Bundled,
      ffmpeg: QuarkConfig::ToolSource::Path,
      gui_download_mode: QuarkConfig::GuiDownloadMode::ExternalCli,
      download_logs: false,
      gui_theme: QuarkConfig::GuiTheme::Dark,
      strip_video_ids: false,
      sanitize_filenames: true,
      filename_spaces: QuarkConfig::FilenameSpaces::Underscore,
      playlist_folders: false,
    )

    QuarkGui::SessionProtocol.build_session_args("/tmp/dl", settings).should eq([
      "--session", "/tmp/dl", "~/Videos", "bundled", "path", "external_cli",
      "false", "dark", "false", "true", "underscore", "false",
    ])
  end

  it "parses a multi-URL download with full settings" do
    result = QuarkGui::SessionProtocol.parse(<<-TEXT)
      __SESSION__
      __SETTINGS__
      ~/Videos
      bundled
      path
      external_cli
      false
      dark
      false
      false
      dash
      false
      __DOWNLOAD_MULTI__
      3
      https://example.com/a
      https://example.com/b
      https://example.com/c
      video
      mp4
      /tmp/downloads
      TEXT

    form = result.settings_form.should_not be_nil
    form.gui_theme.should eq("dark")
    form.strip_video_ids.should be_false
    form.sanitize_filenames.should be_false
    form.filename_spaces.should eq("dash")
    form.playlist_folders.should be_false

    download = result.action.as(QuarkGui::MainAction::Download)
    download.params.urls.should eq([
      "https://example.com/a",
      "https://example.com/b",
      "https://example.com/c",
    ])
    download.params.media_type.should eq("video")
    download.params.format.should eq("mp4")
    download.params.output_dir.should eq("/tmp/downloads")
  end

  it "parses the legacy single-URL download block" do
    result = QuarkGui::SessionProtocol.parse(<<-TEXT)
      __SESSION__
      __DOWNLOAD__
      https://example.com/video
      audio
      mp3
      /tmp/downloads
      TEXT

    download = result.action.as(QuarkGui::MainAction::Download)
    download.params.urls.should eq(["https://example.com/video"])
    download.params.url.should eq("https://example.com/video")
  end

  it "defaults missing settings lines (legacy short blocks)" do
    result = QuarkGui::SessionProtocol.parse(<<-TEXT)
      __SESSION__
      __SETTINGS__
      ~/Legacy
      auto
      auto
      progress
      true
      __CANCEL__
      TEXT

    result.action.should be_a(QuarkGui::MainAction::Cancel)
    form = result.settings_form.should_not be_nil
    form.gui_theme.should eq("light")
    form.strip_video_ids.should be_true
    form.sanitize_filenames.should be_true
    form.filename_spaces.should eq("keep")
    form.playlist_folders.should be_true
  end

  it "rejects malformed multi blocks" do
    result = QuarkGui::SessionProtocol.parse(<<-TEXT)
      __SESSION__
      __DOWNLOAD_MULTI__
      2
      https://example.com/a
      video
      mp4
      /tmp/downloads
      TEXT

    result.action.should be_a(QuarkGui::MainAction::Cancel)
  end

  it "cancels on empty or unknown input" do
    QuarkGui::SessionProtocol.parse("").action.should be_a(QuarkGui::MainAction::Cancel)
    QuarkGui::SessionProtocol.parse("garbage").action.should be_a(QuarkGui::MainAction::Cancel)
  end
end
