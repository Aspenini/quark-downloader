require "../config"

module QuarkGui
  APP_TITLE = "Quark Downloader"

  struct DownloadParams
    property url : String
    property media_type : String
    property format : String
    property output_dir : String

    def initialize(@url, @media_type, @format, @output_dir)
    end
  end

  struct SettingsForm
    property download_dir : String
    property yt_dlp : String
    property ffmpeg : String
    property gui_download_mode : String
    property download_logs : Bool

    def initialize(
      @download_dir,
      @yt_dlp,
      @ffmpeg,
      @gui_download_mode,
      @download_logs,
    )
    end

    def self.from_strings(
      download_dir : String,
      yt_dlp : String,
      ffmpeg : String,
      gui_download_mode : String,
      download_logs : String,
    ) : SettingsForm
      SettingsForm.new(
        download_dir,
        yt_dlp,
        ffmpeg,
        gui_download_mode,
        QuarkConfig.parse_bool(download_logs, "download_logs", default: true, quiet: true),
      )
    end

    def to_settings : QuarkConfig::Settings
      QuarkConfig::Settings.new(
        download_dir: download_dir,
        yt_dlp: QuarkConfig.parse_tool_source(yt_dlp, "yt_dlp", quiet: true),
        ffmpeg: QuarkConfig.parse_tool_source(ffmpeg, "ffmpeg", quiet: true),
        gui_download_mode: QuarkConfig.parse_gui_download_mode(gui_download_mode, quiet: true),
        download_logs: download_logs,
      )
    end
  end

  module MainAction
    struct Download
      getter params : DownloadParams

      def initialize(@params)
      end
    end

    struct Settings
    end

    struct Cancel
    end

    alias Type = Download | Settings | Cancel
  end

  struct MainSessionResult
    getter action : MainAction::Type
    getter settings_form : SettingsForm?

    def initialize(@action : MainAction::Type, @settings_form : SettingsForm? = nil)
    end
  end

  module SettingsAction
    struct Save
      getter form : SettingsForm

      def initialize(@form)
      end
    end

    struct Cancel
    end

    alias Type = Save | Cancel
  end
end
