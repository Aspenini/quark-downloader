require "json"
require "digest/sha256"
require "./tool_http"
require "./config"

module YtDlpTools
  GITHUB_LATEST_URL = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest"
  CHECK_INTERVAL      = 24.hours
  VERSION_FILE        = ".yt-dlp-version"
  CHECK_AT_FILE       = ".yt-dlp-check-at"

  class Error < Exception; end

  def self.asset_name : String
    {% if flag?(:windows) %}
      "yt-dlp.exe"
    {% elsif flag?(:darwin) %}
      "yt-dlp_macos"
    {% else %}
      "yt-dlp"
    {% end %}
  end

  def self.app_dir : Path
    QuarkConfig.app_dir
  end

  def self.tools_dir : Path
    app_dir / "tools"
  end

  def self.bundled_path : Path
    tools_dir / asset_name
  end

  def self.skip_update? : Bool
    ENV["QUARK_SKIP_YTDLP_UPDATE"]? == "1"
  end

  def self.path_executable : String?
    Process.find_executable("yt-dlp")
  end

  MIN_YOUTUBE_YTDLP = "2025.01.26"

  def self.ensure! : String
    case QuarkConfig.yt_dlp_source
    when .path?
      ensure_path_only!
    when .bundled?
      ensure_bundled!
    else
      ensure_auto!
    end
  end

  def self.ensure_auto! : String
    if path = path_executable
      if version = read_version(path)
        if version_at_least?(version, MIN_YOUTUBE_YTDLP)
          puts "Using yt-dlp from PATH: #{path}"
          warn_youtube_js_runtime
          return path
        end

        puts "yt-dlp on PATH (#{version}) is too old for YouTube; using bundled copy."
      end
    end

    ensure_bundled!
  end

  def self.ensure_path_only! : String
    if path = path_executable
      puts "Using yt-dlp from PATH: #{path}"
      warn_if_stale(path)
      warn_youtube_js_runtime
      return path
    end

    raise Error.new(<<-MSG)
      yt-dlp not found on PATH (quark-downloader.conf: yt_dlp = path).
      Install yt-dlp and add it to PATH, or set yt_dlp = auto or bundled.
      MSG
  end

  def self.ensure_bundled! : String
    Dir.mkdir_p(tools_dir.to_s)

    unless File.exists?(bundled_path.to_s)
      if skip_update?
        raise Error.new(<<-MSG)
          yt-dlp not found in tools/ (quark-downloader.conf: yt_dlp = bundled).
          Place #{asset_name} in tools/ or unset QUARK_SKIP_YTDLP_UPDATE to allow download.
          MSG
      end

      puts "Downloading yt-dlp..."
      download_latest!
      return bundled_path.to_s
    end

    puts "Using yt-dlp from: #{bundled_path}"

    if check_due?
      check_and_update_if_needed
    end

    warn_youtube_js_runtime
    bundled_path.to_s
  end

  def self.raise_not_found
    message = {% if flag?(:darwin) %}
      "yt-dlp not found on PATH.\nInstall with Homebrew: brew install yt-dlp"
    {% elsif flag?(:linux) %}
      <<-MSG
        yt-dlp not found on PATH.
        Distro packages (apt install yt-dlp) are often too old for YouTube.
        Prefer a current build: pipx install yt-dlp   or   pip install -U yt-dlp
        Or set yt_dlp = auto in quark-downloader.conf to download a bundled copy.
        MSG
    {% else %}
      <<-MSG
        yt-dlp not found on PATH.
        Install yt-dlp, add it to PATH, or set yt_dlp = auto in quark-downloader.conf.
        MSG
    {% end %}

    raise Error.new(message)
  end

  def self.youtube_url?(url : String) : Bool
    u = url.downcase
    u.includes?("youtube.com") || u.includes?("youtu.be")
  end

  def self.js_runtime : String?
    return "deno" if Process.find_executable("deno")
    return "node" if Process.find_executable("node")
    nil
  end

  def self.preflight_youtube!(url : String)
    return unless youtube_url?(url)
    return if js_runtime

    raise Error.new(<<-MSG)
      YouTube requires a JavaScript runtime for yt-dlp (EJS).
        • Node.js: sudo apt install nodejs   (or your distro equivalent)
        • Deno: see https://github.com/yt-dlp/yt-dlp/wiki/EJS
      MSG
  end

  def self.extra_args(url : String) : Array(String)
    return [] of String unless youtube_url?(url)

    args = [] of String
    if runtime = js_runtime
      args.concat(["--remote-components", "ejs", "--js-runtimes", runtime])
    end
    args
  end

  def self.youtube_failure_hints : String
    hints = <<-HINT
      YouTube download failed. Common fixes:
        • Let quark-downloader use a bundled yt-dlp (yt_dlp = auto in quark-downloader.conf)
        • Or update PATH: pipx install -U yt-dlp   (or brew upgrade yt-dlp)
      HINT

    unless js_runtime
      hints += <<-HINT

        • Install a JS runtime for YouTube: sudo apt install nodejs
          https://github.com/yt-dlp/yt-dlp/wiki/EJS
      HINT
    end

    hints
  end

  def self.read_version(path : String) : String?
    stdout = IO::Memory.new
    status = Process.run(
      command: path,
      args: ["--version"],
      output: stdout,
      error: Process::Redirect::Close,
    )
    return nil unless status.try(&.success?)

    stdout.to_s.chomp.split(/\s+/).first
  end

  def self.parse_version(v : String) : Tuple(Int32, Int32, Int32)
    parts = v.lstrip("v").split('.').map(&.to_i)
    {
      parts[0]? || 0,
      parts[1]? || 0,
      parts[2]? || 0,
    }
  end

  def self.version_at_least?(installed : String, minimum : String) : Bool
    parse_version(installed) >= parse_version(minimum)
  end

  def self.warn_if_stale(path : String)
    version = read_version(path)
    return unless version
    return if version_at_least?(version, MIN_YOUTUBE_YTDLP)

    puts
    puts "Warning: yt-dlp #{version} is likely too old for YouTube."
    puts "  Update: pipx install -U yt-dlp   (or brew upgrade yt-dlp)"
  end

  def self.warn_youtube_js_runtime
    return if js_runtime

    puts "Warning: No Node.js or Deno on PATH — YouTube may fail until you install one (yt-dlp EJS wiki)."
  end

  def self.check_due? : Bool
    check_file = tools_dir / CHECK_AT_FILE
    return true unless File.exists?(check_file.to_s)

    last = File.read(check_file.to_s).chomp.to_i64?
    return true unless last

    (Time.utc - Time.unix(last)) >= CHECK_INTERVAL
  rescue
    true
  end

  def self.record_check_time
    File.write(tools_dir / CHECK_AT_FILE, Time.utc.to_unix.to_s)
  end

  def self.check_and_update_if_needed
    record_check_time
    release = fetch_latest_release
    latest = release["tag_name"].as_s.lstrip("v")
    installed = installed_version

    if installed && !version_newer?(latest, installed)
      return
    end

    puts "Updating yt-dlp (#{installed || "none"} -> #{latest})..."
    download_release!(release)
  rescue ex
    if File.exists?(bundled_path.to_s)
      puts "yt-dlp update skipped: #{ex.message}"
    else
      raise ex
    end
  end

  def self.download_latest!
    release = fetch_latest_release
    download_release!(release)
  end

  def self.fetch_latest_release : JSON::Any
    JSON.parse(ToolHttp.fetch_body(GITHUB_LATEST_URL))
  end

  def self.download_release!(release : JSON::Any)
    tag = release["tag_name"].as_s
    asset = find_asset(release)
    url = asset["browser_download_url"].as_s
    name = asset["name"].as_s

    tmp = tools_dir / "#{name}.download"
    sums = find_checksums_asset(release)

    puts "Fetching #{name} (#{tag})..."
    ToolHttp.download_file(url, tmp)
    verify_checksum!(release, name, tmp) if sums
    install_binary(tmp, bundled_path)
    File.write(tools_dir / VERSION_FILE, tag.lstrip("v"))
    puts "yt-dlp ready (#{tag})."
  end

  def self.find_asset(release : JSON::Any) : JSON::Any
    assets = release["assets"].as_a
    target = asset_name

    assets.each do |asset|
      return asset if asset["name"].as_s == target
    end

    raise Error.new("Release #{release["tag_name"]} has no asset named #{target}")
  end

  def self.find_checksums_asset(release : JSON::Any) : JSON::Any?
    release["assets"].as_a.find do |asset|
      asset["name"].as_s == "SHA2-256SUMS"
    end
  end

  def self.verify_checksum!(release : JSON::Any, binary_name : String, path : Path)
    sums_asset = find_checksums_asset(release)
    return unless sums_asset

    url = sums_asset["browser_download_url"].as_s
    sums_body = ToolHttp.fetch_body(url)

    expected_hash = nil
    sums_body.split('\n').each do |line|
      if m = line.match(/^([a-f0-9]{64})\s+(\S+)\s*$/)
        entry_name = m[2]
        if entry_name == binary_name || entry_name.ends_with?("/#{binary_name}")
          expected_hash = m[1]
          break
        end
      end
    end

    unless expected_hash
      raise Error.new("SHA2-256SUMS has no entry for #{binary_name}")
    end

    actual = Digest::SHA256.hexdigest(File.read(path.to_s))
    unless actual == expected_hash
      File.delete?(path.to_s) if File.exists?(path.to_s)
      raise Error.new("Checksum mismatch for #{binary_name}")
    end
  end

  def self.install_binary(tmp : Path, dest : Path)
    File.delete?(dest.to_s) if File.exists?(dest.to_s)
    begin
      File.rename(tmp.to_s, dest.to_s)
    rescue
      File.copy(tmp.to_s, dest.to_s)
      File.delete?(tmp.to_s)
    end
    {% unless flag?(:windows) %}
    File.chmod(dest.to_s, 0o755)
    {% end %}
  end

  def self.installed_version : String?
    version_file = tools_dir / VERSION_FILE
    if File.exists?(version_file.to_s)
      v = File.read(version_file.to_s).chomp
      return v unless v.empty?
    end

    return nil unless File.exists?(bundled_path.to_s)

    read_version(bundled_path.to_s)
  end

  def self.version_newer?(latest : String, installed : String) : Bool
    parse_version(latest) > parse_version(installed)
  end
end
