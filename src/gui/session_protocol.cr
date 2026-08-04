require "json"
require "../config"
require "./types"

# Protocol between the Crystal GUI binary and platform UI helpers
# (Tcl on Linux, AppKit helper on macOS). Supports:
#   - JSON v1 (preferred): a single JSON object on stdout
#   - Legacy line format: __SESSION__ / __SETTINGS__ / __DOWNLOAD_MULTI__
module QuarkGui
  module SessionProtocol
    PROTOCOL_VERSION = 1

    def self.build_session_args(default_dir : String, settings : QuarkConfig::Settings) : Array(String)
      # Launch helpers still take positional argv; JSON is only for helper → Crystal.
      ytdlp = {% if flag?(:windows) %} settings.yt_dlp.to_config {% else %} "path" {% end %}
      ffmpeg = {% if flag?(:windows) %} settings.ffmpeg.to_config {% else %} "path" {% end %}
      [
        "--session",
        default_dir,
        settings.download_dir,
        ytdlp,
        ffmpeg,
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
      stripped = text.strip
      return MainSessionResult.new(MainAction::Cancel.new) if stripped.empty?

      if stripped.starts_with?('{')
        return parse_json(stripped)
      end

      parse_legacy(text)
    end

    def self.parse_json(text : String) : MainSessionResult
      data = JSON.parse(text)
      settings_form = parse_settings_json(data["settings"]?)

      action = case data["action"]?.try(&.as_s?)
               when "download"
                 urls = data["urls"]?.try(&.as_a?).try(&.map(&.as_s)) || [] of String
                 media = data["media_type"]?.try(&.as_s?) || "video"
                 format = data["format"]?.try(&.as_s?) || "original"
                 output = data["output_dir"]?.try(&.as_s?) || ""
                 if urls.empty? || output.empty?
                   MainAction::Cancel.new
                 else
                   MainAction::Download.new(DownloadParams.new(urls, media, format, output))
                 end
               when "cancel", "settings_only", nil
                 MainAction::Cancel.new
               else
                 MainAction::Cancel.new
               end

      MainSessionResult.new(action, settings_form)
    rescue
      MainSessionResult.new(MainAction::Cancel.new)
    end

    def self.parse_settings_json(node : JSON::Any?) : SettingsForm?
      return nil unless node
      obj = node.as_h?
      return nil unless obj

      SettingsForm.from_strings(
        obj["download_dir"]?.try(&.as_s?) || "~/Downloads",
        obj["yt_dlp"]?.try(&.as_s?) || "path",
        obj["ffmpeg"]?.try(&.as_s?) || "path",
        obj["gui_download_mode"]?.try(&.as_s?) || "progress",
        obj["download_logs"]?.try(&.raw.to_s) || "true",
        obj["gui_theme"]?.try(&.as_s?) || "light",
        obj["strip_video_ids"]?.try(&.raw.to_s) || "true",
        obj["sanitize_filenames"]?.try(&.raw.to_s) || "true",
        obj["filename_spaces"]?.try(&.as_s?) || "keep",
        obj["playlist_folders"]?.try(&.raw.to_s) || "true",
      )
    rescue
      nil
    end

    # Build a JSON session response for helpers (Tcl / Swift).
    def self.emit_json(
      action : String,
      settings : SettingsForm? = nil,
      urls : Array(String) = [] of String,
      media_type : String = "video",
      format : String = "original",
      output_dir : String = "",
    ) : String
      String.build do |io|
        io << %({"v":#{PROTOCOL_VERSION},"action":#{action.to_json})
        if settings
          io << %(,"settings":{)
          io << %("download_dir":#{settings.download_dir.to_json},)
          io << %("yt_dlp":#{settings.yt_dlp.to_json},)
          io << %("ffmpeg":#{settings.ffmpeg.to_json},)
          io << %("gui_download_mode":#{settings.gui_download_mode.to_json},)
          io << %("download_logs":#{settings.download_logs},)
          io << %("gui_theme":#{settings.gui_theme.to_json},)
          io << %("strip_video_ids":#{settings.strip_video_ids},)
          io << %("sanitize_filenames":#{settings.sanitize_filenames},)
          io << %("filename_spaces":#{settings.filename_spaces.to_json},)
          io << %("playlist_folders":#{settings.playlist_folders})
          io << '}'
        end
        if action == "download"
          io << %(,"urls":#{urls.to_json})
          io << %(,"media_type":#{media_type.to_json})
          io << %(,"format":#{format.to_json})
          io << %(,"output_dir":#{output_dir.to_json})
        end
        io << '}'
      end
    end

    def self.parse_legacy(text : String) : MainSessionResult
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
