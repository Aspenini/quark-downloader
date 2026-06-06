module QuarkConfig
  enum ToolSource
    Auto
    Path
    Bundled

    def to_config : String
      case self
      when Auto    then "auto"
      when Path    then "path"
      when Bundled then "bundled"
      else              "auto"
      end
    end
  end

  enum GuiDownloadMode
    Progress
    ExternalCli

    def to_config : String
      case self
      when Progress    then "progress"
      when ExternalCli then "external_cli"
      else                  "progress"
      end
    end
  end

  enum GuiTheme
    Light
    Dark

    def to_config : String
      case self
      when Light then "light"
      when Dark  then "dark"
      else            "light"
      end
    end
  end

  struct Settings
    DEFAULT_DOWNLOAD_DIR = "~/Downloads"

    property download_dir : String
    property yt_dlp : ToolSource
    property ffmpeg : ToolSource
    property gui_download_mode : GuiDownloadMode
    property download_logs : Bool
    property gui_theme : GuiTheme

    def initialize(
      @download_dir : String = DEFAULT_DOWNLOAD_DIR,
      @yt_dlp : ToolSource = ToolSource::Auto,
      @ffmpeg : ToolSource = ToolSource::Auto,
      @gui_download_mode : GuiDownloadMode = GuiDownloadMode::Progress,
      @download_logs : Bool = true,
      @gui_theme : GuiTheme = GuiTheme::Light,
    )
    end
  end

  CONFIG_NAME = "quark-downloader.conf"
  APP_NAME    = "quark-downloader"
  CONFIG_KEYS = ["download_dir", "yt_dlp", "ffmpeg", "gui_download_mode", "download_logs", "gui_theme"]

  @@settings = Settings.new

  def self.app_dir : Path
    if exe = Process.executable_path
      parent = Path[exe].parent
      if parent.to_s.gsub('\\', '/').includes?("/crystal/cache")
        return Path[Dir.current]
      end
      return parent
    end

    Path[Dir.current]
  end

  def self.config_dir : Path
    {% if flag?(:windows) %}
      Path[ENV["APPDATA"]? || user_home] / APP_NAME
    {% else %}
      Path[ENV["XDG_CONFIG_HOME"]? || File.join(user_home, ".config")] / APP_NAME
    {% end %}
  end

  def self.config_path : Path
    config_dir / CONFIG_NAME
  end

  def self.ensure_config_dir!
    Dir.mkdir_p(config_dir.to_s)
  end

  def self.load!(quiet : Bool = false) : Nil
    ensure_config_dir!
    path = config_path
    unless File.exists?(path.to_s)
      create_default!(path, announce: !quiet)
    end
    parsed = parse_file_with_keys(path, quiet: quiet)
    @@settings = parsed[0]
    append_missing_defaults!(path, @@settings, parsed[1])
  end

  def self.settings : Settings
    @@settings
  end

  def self.save!(settings : Settings = @@settings) : Nil
    ensure_config_dir!
    File.write(config_path.to_s, render(settings))
    @@settings = settings
  end

  def self.download_dir(fallback : String) : String
    dir = @@settings.download_dir
    return fallback if dir.empty?

    expand_path(dir)
  end

  def self.download_dir_setting : String
    @@settings.download_dir
  end

  def self.yt_dlp_source : ToolSource
    @@settings.yt_dlp
  end

  def self.ffmpeg_source : ToolSource
    @@settings.ffmpeg
  end

  def self.gui_download_mode : GuiDownloadMode
    @@settings.gui_download_mode
  end

  def self.download_logs? : Bool
    @@settings.download_logs
  end

  def self.gui_theme : GuiTheme
    @@settings.gui_theme
  end

  def self.user_home : String
    ENV["USERPROFILE"]? || ENV["HOME"]? || "."
  end

  def self.expand_path(path : String) : String
    expanded = if path.starts_with?("~/")
                 File.join(user_home, path[2..])
               elsif path == "~"
                 user_home
               else
                 path
               end
    File.expand_path(expanded)
  end

  def self.create_default!(path : Path, announce : Bool = true)
    File.write(path.to_s, render(Settings.new))
    puts "Created config: #{path}" if announce
  end

  def self.parse_file(path : Path, quiet : Bool = false) : Settings
    parse_file_with_keys(path, quiet: quiet)[0]
  end

  def self.parse_file_with_keys(path : Path, quiet : Bool = false) : {Settings, Array(String)}
    settings = Settings.new
    keys = [] of String

    File.each_line(path.to_s) do |line|
      line = line.strip
      next if line.empty? || line.starts_with?('#')
      next unless line.includes?('=')

      key, value = line.split('=', limit: 2).map(&.strip)
      next if key.empty? || value.empty?

      normalized_key = key.downcase
      keys << normalized_key

      case normalized_key
      when "download_dir"
        settings.download_dir = value
      when "yt_dlp"
        settings.yt_dlp = parse_tool_source(value, "yt_dlp", quiet: quiet)
      when "ffmpeg"
        settings.ffmpeg = parse_tool_source(value, "ffmpeg", quiet: quiet)
      when "gui_download_mode"
        settings.gui_download_mode = parse_gui_download_mode(value, quiet: quiet)
      when "download_logs"
        settings.download_logs = parse_bool(value, "download_logs", default: true, quiet: quiet)
      when "gui_theme"
        settings.gui_theme = parse_gui_theme(value, quiet: quiet)
      else
        puts "Warning: unknown config key #{key.inspect} in #{path}" unless quiet
      end
    end

    {settings, keys}
  end

  def self.append_missing_defaults!(path : Path, settings : Settings, existing_keys : Array(String)) : Nil
    missing = CONFIG_KEYS.reject { |key| existing_keys.includes?(key) }
    return if missing.empty?

    File.open(path.to_s, "a") do |file|
      file.puts
      file.puts "# Added by quark-downloader"
      missing.each do |key|
        file.puts("#{key} = #{config_value(settings, key)}")
      end
    end
  end

  def self.config_value(settings : Settings, key : String) : String
    case key
    when "download_dir"      then settings.download_dir
    when "yt_dlp"            then settings.yt_dlp.to_config
    when "ffmpeg"            then settings.ffmpeg.to_config
    when "gui_download_mode" then settings.gui_download_mode.to_config
    when "download_logs"     then settings.download_logs.to_s
    when "gui_theme"         then settings.gui_theme.to_config
    else                          ""
    end
  end

  def self.parse_tool_source(value : String, key : String, quiet : Bool = false) : ToolSource
    case value.downcase
    when "path"    then ToolSource::Path
    when "bundled" then ToolSource::Bundled
    when "auto"    then ToolSource::Auto
    else
      puts "Warning: invalid #{key} value #{value.inspect}, using auto" unless quiet
      ToolSource::Auto
    end
  end

  def self.parse_gui_download_mode(value : String, quiet : Bool = false) : GuiDownloadMode
    case value.downcase
    when "external_cli", "cli", "terminal" then GuiDownloadMode::ExternalCli
    when "progress", "gui"                 then GuiDownloadMode::Progress
    else
      puts "Warning: invalid gui_download_mode value #{value.inspect}, using progress" unless quiet
      GuiDownloadMode::Progress
    end
  end

  def self.parse_gui_theme(value : String, quiet : Bool = false) : GuiTheme
    case value.downcase
    when "dark"  then GuiTheme::Dark
    when "light" then GuiTheme::Light
    else
      puts "Warning: invalid gui_theme value #{value.inspect}, using light" unless quiet
      GuiTheme::Light
    end
  end

  def self.parse_bool(value : String, key : String, default : Bool, quiet : Bool = false) : Bool
    case value.downcase
    when "1", "true", "yes", "on"  then true
    when "0", "false", "no", "off" then false
    else
      puts "Warning: invalid #{key} value #{value.inspect}, using #{default}" unless quiet
      default
    end
  end

  def self.render(settings : Settings) : String
    [
      "# Quark Downloader configuration",
      "# Save and restart to apply changes.",
      "",
      "# Default folder offered at the output prompt (~ = your home directory)",
      "download_dir = #{settings.download_dir}",
      "",
      "# How to locate yt-dlp and ffmpeg",
      "#   auto    - PATH first, then bundled tools beside the app",
      "#   path    - PATH only",
      "#   bundled - bundled tools beside the app only (may download if missing)",
      "yt_dlp = #{settings.yt_dlp.to_config}",
      "ffmpeg = #{settings.ffmpeg.to_config}",
      "",
      "# GUI download behavior",
      "#   progress     - show the GUI progress dialog and completion popup",
      "#   external_cli - open the CLI window after Download and close the GUI",
      "gui_download_mode = #{settings.gui_download_mode.to_config}",
      "",
      "# Create rotated logs for CLI and GUI downloads",
      "download_logs = #{settings.download_logs}",
      "",
      "# GUI appearance",
      "#   light - light controls and window backgrounds",
      "#   dark  - dark controls and window backgrounds where supported",
      "gui_theme = #{settings.gui_theme.to_config}",
      "",
    ].join('\n')
  end
end
