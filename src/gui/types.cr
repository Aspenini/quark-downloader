require "../config"
require "../version"

module QuarkGui
  APP_TITLE             = QuarkVersion::APP_NAME
  WINDOW_TITLE          = QuarkVersion.window_title
  SETTINGS_WINDOW_TITLE = QuarkVersion.settings_window_title

  struct DownloadParams
    property urls : Array(String)
    property media_type : String
    property format : String
    property output_dir : String

    def initialize(@urls : Array(String), @media_type, @format, @output_dir)
    end

    def url : String
      urls.first? || ""
    end
  end

  struct SettingsForm
    property download_dir : String
    property yt_dlp : String
    property ffmpeg : String
    property gui_download_mode : String
    property download_logs : Bool
    property gui_theme : String
    property strip_video_ids : Bool
    property sanitize_filenames : Bool
    property filename_spaces : String
    property playlist_folders : Bool

    def initialize(
      @download_dir,
      @yt_dlp,
      @ffmpeg,
      @gui_download_mode,
      @download_logs,
      @gui_theme = QuarkConfig::GuiTheme::Light.to_config,
      @strip_video_ids = true,
      @sanitize_filenames = true,
      @filename_spaces = QuarkConfig::FilenameSpaces::Keep.to_config,
      @playlist_folders = true,
    )
    end

    def self.from_strings(
      download_dir : String,
      yt_dlp : String,
      ffmpeg : String,
      gui_download_mode : String,
      download_logs : String,
      gui_theme : String = QuarkConfig::GuiTheme::Light.to_config,
      strip_video_ids : String = "true",
      sanitize_filenames : String = "true",
      filename_spaces : String = QuarkConfig::FilenameSpaces::Keep.to_config,
      playlist_folders : String = "true",
    ) : SettingsForm
      SettingsForm.new(
        download_dir,
        yt_dlp,
        ffmpeg,
        gui_download_mode,
        QuarkConfig.parse_bool(download_logs, "download_logs", default: true, quiet: true),
        QuarkConfig.parse_gui_theme(gui_theme, quiet: true).to_config,
        QuarkConfig.parse_bool(strip_video_ids, "strip_video_ids", default: true, quiet: true),
        QuarkConfig.parse_bool(sanitize_filenames, "sanitize_filenames", default: true, quiet: true),
        QuarkConfig.parse_filename_spaces(filename_spaces, quiet: true).to_config,
        QuarkConfig.parse_bool(playlist_folders, "playlist_folders", default: true, quiet: true),
      )
    end

    def to_settings : QuarkConfig::Settings
      QuarkConfig::Settings.new(
        download_dir: download_dir,
        yt_dlp: QuarkConfig.parse_tool_source(yt_dlp, "yt_dlp", quiet: true),
        ffmpeg: QuarkConfig.parse_tool_source(ffmpeg, "ffmpeg", quiet: true),
        gui_download_mode: QuarkConfig.parse_gui_download_mode(gui_download_mode, quiet: true),
        download_logs: download_logs,
        gui_theme: QuarkConfig.parse_gui_theme(gui_theme, quiet: true),
        strip_video_ids: strip_video_ids,
        sanitize_filenames: sanitize_filenames,
        filename_spaces: QuarkConfig.parse_filename_spaces(filename_spaces, quiet: true),
        playlist_folders: playlist_folders,
      )
    end
  end

  module MainAction
    struct Download
      getter params : DownloadParams

      def initialize(@params)
      end
    end

    struct Cancel
    end

    alias Type = Download | Cancel
  end

  struct MainSessionResult
    getter action : MainAction::Type
    getter settings_form : SettingsForm?

    def initialize(@action : MainAction::Type, @settings_form : SettingsForm? = nil)
    end
  end
end
