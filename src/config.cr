module QuarkConfig
  enum ToolSource
    Auto
    Path
    Bundled
  end

  CONFIG_NAME = "quark-downloader.conf"
  APP_NAME    = "quark-downloader"

  @@download_dir : String? = nil
  @@yt_dlp = ToolSource::Auto
  @@ffmpeg = ToolSource::Auto

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
    parse_file(path)
  end

  def self.download_dir(fallback : String) : String
    dir = @@download_dir
    return fallback unless dir && !dir.empty?

    expand_path(dir)
  end

  def self.yt_dlp_source : ToolSource
    @@yt_dlp
  end

  def self.ffmpeg_source : ToolSource
    @@ffmpeg
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
    contents = <<-CONF
# Quark Downloader configuration
# Save and restart to apply changes.

# Default folder offered at the output prompt (~ = your home directory)
download_dir = ~/Downloads

# How to locate yt-dlp and ffmpeg
#   auto    - PATH first, then bundled tools beside the app
#   path    - PATH only
#   bundled - bundled tools beside the app only (may download if missing)
yt_dlp = auto
ffmpeg = auto
CONF

    File.write(path.to_s, contents)
    puts "Created config: #{path}" if announce
  end

  def self.parse_file(path : Path)
    File.each_line(path.to_s) do |line|
      line = line.strip
      next if line.empty? || line.starts_with?('#')
      next unless line.includes?('=')

      key, value = line.split('=', limit: 2).map(&.strip)
      next if key.empty? || value.empty?

      case key.downcase
      when "download_dir"
        @@download_dir = value
      when "yt_dlp"
        @@yt_dlp = parse_tool_source(value, "yt_dlp")
      when "ffmpeg"
        @@ffmpeg = parse_tool_source(value, "ffmpeg")
      else
        puts "Warning: unknown config key #{key.inspect} in #{path}"
      end
    end
  end

  def self.parse_tool_source(value : String, key : String) : ToolSource
    case value.downcase
    when "path"    then ToolSource::Path
    when "bundled" then ToolSource::Bundled
    when "auto"    then ToolSource::Auto
    else
      puts "Warning: invalid #{key} value #{value.inspect}, using auto"
      ToolSource::Auto
    end
  end
end
