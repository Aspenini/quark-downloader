require "./config"
require "./ytdlp_tools"
require "./ffmpeg_tools"

module QuarkDownload
  def self.user_home : String
    ENV["USERPROFILE"]? || ENV["HOME"]? || "."
  end

  def self.default_downloads_dir : String
    {% if flag?(:windows) %}
      return File.join(user_home, "Downloads")
    {% else %}
      if xdg = xdg_download_dir?
        return xdg
      end
    {% end %}

    File.join(user_home, "Downloads")
  end

  def self.xdg_download_dir? : String?
    config = Path[user_home] / ".config" / "user-dirs.dirs"
    return nil unless File.exists?(config.to_s)

    File.each_line(config.to_s) do |line|
      next unless line.starts_with?("XDG_DOWNLOAD_DIR=")
      if m = line.match(/^XDG_DOWNLOAD_DIR="(.+)"\s*$/)
        return File.expand_path(m[1].gsub("$HOME", user_home))
      end
    end

    nil
  end

  def self.default_output_dir : String
    QuarkConfig.load!(quiet: true)
    QuarkConfig.download_dir(default_downloads_dir)
  end

  def self.run(
    url : String,
    media_type : String,
    format : String = "original",
    output_dir : String? = nil,
    no_pause : Bool = false,
  ) : Int32
    QuarkConfig.load!(quiet: true)

    ytdlp = begin
      YtDlpTools.ensure!
    rescue ex : YtDlpTools::Error
      abort_with(ex.message || ex.to_s, no_pause)
    end

    begin
      YtDlpTools.preflight_youtube!(url)
    rescue ex : YtDlpTools::Error
      abort_with(ex.message || ex.to_s, no_pause)
    end

    FfmpegTools.detect!

    media_type = media_type.downcase
    unless {"audio", "video"}.includes?(media_type)
      abort_with("Invalid media type: #{media_type.inspect} (expected audio or video)", no_pause)
    end

    format = format.downcase
    format = "original" if format.empty?

    dir = output_dir || QuarkConfig.download_dir(default_downloads_dir)
    output_path = Path[File.expand_path(dir)]

    begin
      Dir.mkdir_p(output_path.to_s)
    rescue ex
      abort_with("Error creating output directory:\n#{ex}", no_pause)
    end

    outtmpl = (output_path / "%(title)s [%(id)s].%(ext)s").to_s
    cmd = [ytdlp, "--no-playlist", "-o", outtmpl]

    if media_type == "audio"
      cmd.concat(["-f", "bestaudio/best"])
      unless {"original", "default.original"}.includes?(format)
        append_ffmpeg!(cmd, no_pause)
        cmd.concat(["-x", "--audio-format", format])
      end
    else
      unless {"original", "default.original"}.includes?(format)
        append_ffmpeg!(cmd, no_pause)
        cmd.concat(["-f", "bv*+ba/b", "--merge-output-format", format])
        case format
        when "webm"
          cmd.concat(["--recode-video", "webm"])
        when "mp4"
          cmd.concat(["--remux-video", "mp4"])
        end
      end
    end

    if ENV["QUARK_GUI"]? == "1"
      cmd.concat(["--newline", "--no-color"])
    end

    cmd.concat(YtDlpTools.extra_args(url))
    cmd << url

    exit_code = run_command(cmd)

    if exit_code == 0
      puts "Done."
      press_any_key(no_pause)
    else
      message = "Failed with exit code #{exit_code}."
      message += "\n\n#{YtDlpTools.youtube_failure_hints}" if YtDlpTools.youtube_url?(url)
      abort_with(message, no_pause, exit_code)
    end

    exit_code
  end

  def self.append_ffmpeg!(cmd : Array(String), no_pause : Bool)
    FfmpegTools.append_to_cmd!(cmd)
  rescue ex : FfmpegTools::Error
    abort_with(ex.message || ex.to_s, no_pause)
  end

  def self.run_command(cmd : Array(String)) : Int32
    puts "\nRunning:"
    puts cmd.map { |x| x.includes?(' ') ? %("#{x}") : x }.join(' ')
    puts

    status = Process.run(
      command: cmd.first,
      args: cmd[1..]?,
      output: Process::Redirect::Inherit,
      error: Process::Redirect::Inherit,
    )
    status.try(&.exit_code) || 127
  rescue File::NotFoundError
    puts "Error: #{cmd.first} was not found."
    127
  end

  def self.press_any_key(no_pause : Bool, message = "Press any key to exit...")
    return if no_pause
    {% if flag?(:windows) %}
      puts
      puts message
      begin
        STDIN.raw { |io| io.read_byte }
      rescue IO::Error
        gets
      end
    {% end %}
  end

  def self.abort_with(message : String, no_pause : Bool, code = 1) : Nil
    puts message
    press_any_key(no_pause)
    exit(code)
  end
end
