{% unless flag?(:windows) %}
  require "spec"
  require "../src/gui/tk_ui"

  describe QuarkGui::TkUi do
    it "parses saved settings followed by a download action" do
      result = QuarkGui::TkUi.parse_main_session_response(<<-TEXT)
        __SESSION__
        __SETTINGS__
        ~/Videos
        bundled
        path
        external_cli
        false
        dark
        __DOWNLOAD__
        https://example.com/video
        audio
        mp3
        /tmp/downloads
        TEXT

      form = result.settings_form.should_not be_nil
      form.download_dir.should eq("~/Videos")
      form.yt_dlp.should eq("bundled")
      form.ffmpeg.should eq("path")
      form.gui_download_mode.should eq("external_cli")
      form.download_logs.should be_false
      form.gui_theme.should eq("dark")

      action = result.action
      action.should be_a(QuarkGui::MainAction::Download)
      download = action.as(QuarkGui::MainAction::Download)
      download.params.url.should eq("https://example.com/video")
      download.params.media_type.should eq("audio")
      download.params.format.should eq("mp3")
      download.params.output_dir.should eq("/tmp/downloads")
    end

    it "keeps saved settings when the final action is cancel" do
      result = QuarkGui::TkUi.parse_main_session_response(<<-TEXT)
        __SESSION__
        __SETTINGS__
        ~/Music
        auto
        bundled
        progress
        true
        light
        __CANCEL__
        TEXT

      result.action.should be_a(QuarkGui::MainAction::Cancel)
      form = result.settings_form.should_not be_nil
      form.download_dir.should eq("~/Music")
      form.ffmpeg.should eq("bundled")
      form.download_logs.should be_true
      form.gui_theme.should eq("light")
    end

    it "defaults legacy saved settings to light theme" do
      result = QuarkGui::TkUi.parse_main_session_response(<<-TEXT)
        __SESSION__
        __SETTINGS__
        ~/Legacy
        auto
        auto
        progress
        true
        __CANCEL__
        TEXT

      form = result.settings_form.should_not be_nil
      form.download_dir.should eq("~/Legacy")
      form.gui_theme.should eq("light")
    end
  end
{% end %}
