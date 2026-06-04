require "./ytdlp_tools"
require "./logs"
{% if flag?(:windows) %}
  require "json"
  require "file_utils"
  require "./tool_http"
{% end %}

module FfmpegTools
  class Error < Exception; end

  def self.executable_name : String
    {% if flag?(:windows) %}
      "ffmpeg.exe"
    {% else %}
      "ffmpeg"
    {% end %}
  end

  def self.ffprobe_name : String
    {% if flag?(:windows) %}
      "ffprobe.exe"
    {% else %}
      "ffprobe"
    {% end %}
  end

  def self.tools_dir : Path
    YtDlpTools.tools_dir
  end

  def self.bundled_path : Path
    tools_dir / executable_name
  end

  def self.ffprobe_path : Path
    tools_dir / ffprobe_name
  end

  def self.skip_download? : Bool
    ENV["QUARK_SKIP_FFMPEG_DOWNLOAD"]? == "1"
  end

  {% if flag?(:windows) %}
    GITHUB_LATEST_URL = "https://api.github.com/repos/BtbN/FFmpeg-Builds/releases/latest"
    VERSION_FILE      = ".ffmpeg-version"
  {% end %}

  @@detected = false

  def self.bundled? : Bool
    {% if flag?(:windows) %}
      File.exists?(bundled_path.to_s)
    {% else %}
      false
    {% end %}
  end

  def self.path_executable : String?
    Process.find_executable("ffmpeg")
  end

  def self.locate : Tuple(Bool, String)?
    source = {% if flag?(:windows) %} QuarkConfig.ffmpeg_source {% else %} QuarkConfig::ToolSource::Auto {% end %}

    if source.path? || source.auto?
      if exe = path_executable
        return {true, exe}
      end
    end

    if {% if flag?(:windows) %} source.bundled? || source.auto? {% else %} false {% end %}
      if bundled?
        return {false, bundled_path.to_s}
      end
    end

    nil
  end

  def self.detect! : Nil
    if location = locate
      from_path, path = location
      if from_path
        QuarkLogs.puts "Using ffmpeg from PATH: #{path}"
      else
        QuarkLogs.puts "Using ffmpeg from: #{path}"
      end
    else
      warn_not_found
    end

    @@detected = true
  end

  # Directory for yt-dlp's --ffmpeg-location (PATH first; on Windows, tools/ then download).
  def self.ensure! : String
    {% if flag?(:windows) %}
      case QuarkConfig.ffmpeg_source
      when .path?
        ensure_path_only!
      when .bundled?
        ensure_bundled!
      else
        if exe = path_executable
          QuarkLogs.puts "Using ffmpeg from PATH: #{exe}" unless @@detected
          return Path[exe].parent.to_s
        end
        ensure_bundled!
      end
    {% else %}
      if exe = path_executable
        QuarkLogs.puts "Using ffmpeg from PATH: #{exe}" unless @@detected
        return Path[exe].parent.to_s
      end

      raise_not_found
    {% end %}
  end

  {% if flag?(:windows) %}
    def self.ensure_path_only! : String
      if exe = path_executable
        QuarkLogs.puts "Using ffmpeg from PATH: #{exe}" unless @@detected
        return Path[exe].parent.to_s
      end

      raise Error.new(<<-MSG)
      ffmpeg not found on PATH (quark-downloader.conf: ffmpeg = path).
      Install ffmpeg and add it to PATH, or set ffmpeg = auto or bundled.
      MSG
    end

    def self.ensure_bundled! : String
      Dir.mkdir_p(tools_dir.to_s)

      if bundled?
        QuarkLogs.puts "Using ffmpeg from: #{bundled_path}" unless @@detected
        return tools_dir.to_s
      end

      unless skip_download?
        QuarkLogs.puts "Downloading ffmpeg..."
        download_latest!
        return tools_dir.to_s
      end

      raise Error.new(<<-MSG)
      ffmpeg not found in tools/ (quark-downloader.conf: ffmpeg = bundled).
      Place ffmpeg.exe in tools/ or allow a network download when converting formats.
      MSG
    end
  {% end %}

  def self.warn_not_found
    QuarkLogs.puts
    QuarkLogs.puts "Warning: ffmpeg not found on PATH."
    {% if flag?(:darwin) %}
      QuarkLogs.puts "  Install with Homebrew: brew install ffmpeg"
    {% elsif flag?(:linux) %}
      QuarkLogs.puts "  Install with your package manager, e.g. apt install ffmpeg"
    {% else %}
      QuarkLogs.puts "  Install ffmpeg, add it to PATH, place binaries in bundled-tools/ and rebuild,"
      QuarkLogs.puts "  or allow a network download when converting formats."
    {% end %}
    QuarkLogs.puts "  Original-format downloads may still work; conversion requires ffmpeg."
  end

  def self.append_to_cmd!(cmd : Array(String))
    cmd.concat(["--ffmpeg-location", ensure!])
  end

  def self.raise_not_found
    message = {% if flag?(:darwin) %}
                "ffmpeg not found on PATH.\nInstall with Homebrew: brew install ffmpeg"
              {% elsif flag?(:linux) %}
                "ffmpeg not found on PATH.\nInstall with your package manager, e.g. apt install ffmpeg"
              {% else %}
                <<-MSG
        ffmpeg not found on PATH.
        Install ffmpeg, add it to PATH, place binaries in bundled-tools/ and rebuild, or allow a network download on next run.
        MSG
              {% end %}

    raise Error.new(message)
  end

  {% if flag?(:windows) %}
  def self.download_latest!
    release = fetch_btbn_release
    asset = find_btbn_asset(release)
    url = asset["browser_download_url"].as_s
    name = asset["name"].as_s
    tag = release["tag_name"].as_s
    QuarkLogs.puts "Fetching #{name}..."
    archive = tools_dir / name
    ToolHttp.download_file(url, archive)
    extract_and_install(archive, tag)
    File.delete?(archive.to_s) if File.exists?(archive.to_s)
  end

  def self.fetch_btbn_release : JSON::Any
    JSON.parse(ToolHttp.fetch_body(GITHUB_LATEST_URL))
  end

  def self.btbn_asset_name : String
    {% if flag?(:aarch64) %}
      "ffmpeg-master-latest-winarm64-gpl.zip"
    {% else %}
      "ffmpeg-master-latest-win64-gpl.zip"
    {% end %}
  end

  def self.find_btbn_asset(release : JSON::Any) : JSON::Any
    target = btbn_asset_name
    release["assets"].as_a.each do |asset|
      return asset if asset["name"].as_s == target
    end
    raise Error.new("FFmpeg release has no asset named #{target}")
  end

  def self.extract_and_install(archive : Path, version_label : String)
    extract_dir = tools_dir / ".ffmpeg-extract"
    FileUtils.rm_rf(extract_dir.to_s) if File.exists?(extract_dir.to_s)
    Dir.mkdir_p(extract_dir.to_s)

    unless extract_archive(archive, extract_dir)
      raise Error.new("Failed to extract #{archive}")
    end

    ffmpeg_src = find_in_tree(extract_dir, executable_name)
    raise Error.new("Extracted archive did not contain #{executable_name}") unless ffmpeg_src

    install_binary(ffmpeg_src, bundled_path)

    if probe_src = find_in_tree(extract_dir, ffprobe_name)
      install_binary(probe_src, ffprobe_path)
    end

    FileUtils.rm_rf(extract_dir.to_s)
    File.write(tools_dir / VERSION_FILE, version_label)
    QuarkLogs.puts "ffmpeg ready (#{version_label})."
  end

  def self.extract_archive(archive : Path, dest : Path) : Bool
    if archive.to_s.ends_with?(".zip")
      status = Process.run(
        "powershell",
        args: [
          "-NoProfile", "-Command",
          "Expand-Archive -LiteralPath '#{archive.to_s}' -DestinationPath '#{dest.to_s}' -Force",
        ],
      )
      return status.try(&.success?) || false
    end

    status = Process.run("tar", args: ["-xf", archive.to_s, "-C", dest.to_s])
    status.try(&.success?) || false
  end

  def self.find_in_tree(dir : Path, filename : String) : Path?
    pattern = File.join(dir.to_s, "**", filename)
    Dir.glob(pattern).each do |match|
      return Path[match]
    end
    nil
  end

  def self.install_binary(src : Path, dest : Path)
    File.delete?(dest.to_s) if File.exists?(dest.to_s)
    File.copy(src.to_s, dest.to_s)
  end
  {% end %}
end
