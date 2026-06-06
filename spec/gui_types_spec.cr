require "spec"
require "../src/gui/types"

describe QuarkGui::SettingsForm do
  it "converts raw platform form values into config settings" do
    form = QuarkGui::SettingsForm.from_strings(
      "~/Videos",
      "bundled",
      "path",
      "external_cli",
      "off",
      "dark",
    )

    settings = form.to_settings

    settings.download_dir.should eq("~/Videos")
    settings.yt_dlp.should eq(QuarkConfig::ToolSource::Bundled)
    settings.ffmpeg.should eq(QuarkConfig::ToolSource::Path)
    settings.gui_download_mode.should eq(QuarkConfig::GuiDownloadMode::ExternalCli)
    settings.download_logs.should be_false
    settings.gui_theme.should eq(QuarkConfig::GuiTheme::Dark)
  end
end
