require "../config"
require "./types"

# The line protocol spoken between the Crystal GUI binary and the platform
# UI helpers (Tcl script on Linux, AppKit helper on macOS).
#
# Helper -> Crystal (stdout), after `--session`:
#   __SESSION__
#   [__SETTINGS__ followed by up to 10 lines: download_dir, yt_dlp, ffmpeg,
#    gui_download_mode, download_logs, gui_theme, strip_video_ids,
#    sanitize_filenames, filename_spaces, playlist_folders]
#   __DOWNLOAD_MULTI__ <count> <url...> <media_type> <format> <output_dir>
#   | __DOWNLOAD__ <url> <media_type> <format> <output_dir>   (legacy)
#   | __CANCEL__
module QuarkGui
  module SessionProtocol
    def self.build_session_args(default_dir : String, settings : QuarkConfig::Settings) : Array(String)
      [
        "--session",
        default_dir,
        settings.download_dir,
        settings.yt_dlp.to_config,
        settings.ffmpeg.to_config,
        settings.gui_download_mode.to_config,
        settings.download_logs.to_s,
        settings.gui_theme.to_config,
        settings.strip_video_ids.to_s,
        settings.sanitize_filenames.to_s,
        settings.filename_spaces.to_config,
        settings.playlist_folders.to_s,
      ]
    end

    def self.parse(text : String) : MainSessionResult
      lines = text.lines.map(&.strip)
      return MainSessionResult.new(MainAction::Cancel.new) if lines.empty?
      return MainSessionResult.new(MainAction::Cancel.new) unless lines.first == "__SESSION__"

      action : MainAction::Type = MainAction::Cancel.new
      settings_form : SettingsForm? = nil
      i = 1

      while i < lines.size
        case lines[i]
        when "__SETTINGS__"
          block, i = read_block(lines, i + 1)
          settings_form = parse_settings(block) || settings_form
        when "__DOWNLOAD__"
          block, i = read_block(lines, i + 1)
          if block.size >= 4 && !block[0].empty? && !block[3].empty?
            action = MainAction::Download.new(
              DownloadParams.new([block[0]], block[1], block[2], block[3])
            )
          end
        when "__DOWNLOAD_MULTI__"
          block, i = read_block(lines, i + 1)
          if download = parse_download_multi(block)
            action = download
          end
        when "__CANCEL__"
          action = MainAction::Cancel.new
          i += 1
        else
          i += 1
        end
      end

      MainSessionResult.new(action, settings_form)
    end

    # Collects lines from `start` until the next `__`-prefixed sentinel.
    private def self.read_block(lines : Array(String), start : Int32) : {Array(String), Int32}
      stop = start
      while stop < lines.size && !lines[stop].starts_with?("__")
        stop += 1
      end
      {lines[start...stop], stop}
    end

    private def self.parse_settings(block : Array(String)) : SettingsForm?
      return nil if block.size < 5

      SettingsForm.from_strings(
        block[0],
        block[1],
        block[2],
        block[3],
        block[4],
        block[5]? || QuarkConfig::GuiTheme::Light.to_config,
        block[6]? || "true",
        block[7]? || "true",
        block[8]? || QuarkConfig::FilenameSpaces::Keep.to_config,
        block[9]? || "true",
      )
    end

    private def self.parse_download_multi(block : Array(String)) : MainAction::Download?
      count = block[0]?.try(&.to_i?)
      return nil unless count && count > 0
      return nil unless block.size == count + 4

      urls = block[1, count].reject(&.empty?)
      return nil if urls.empty?

      media_type = block[count + 1]
      format = block[count + 2]
      output_dir = block[count + 3]
      return nil if output_dir.empty?

      MainAction::Download.new(DownloadParams.new(urls, media_type, format, output_dir))
    end
  end
end
