require "./config"
require "./logs"
require "./ytdlp_tools"
require "./ffmpeg_tools"
{% if flag?(:windows) %}
  require "./win32_hidden_process"
{% end %}

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
    {% if flag?(:windows) %}
      if ENV["QUARK_GUI"]? == "1"
        STDOUT.sync = true
        STDERR.sync = true
      end
    {% end %}

    QuarkConfig.load!(quiet: true)
    QuarkLogs.open_download_log

    begin
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
        QuarkLogs.puts "Done."
        press_any_key(no_pause)
      else
        message = "Failed with exit code #{exit_code}."
        message += "\n\n#{YtDlpTools.youtube_failure_hints}" if YtDlpTools.youtube_url?(url)
        abort_with(message, no_pause, exit_code)
      end

      exit_code
    ensure
      QuarkLogs.close
    end
  end

  def self.append_ffmpeg!(cmd : Array(String), no_pause : Bool)
    FfmpegTools.append_to_cmd!(cmd)
  rescue ex : FfmpegTools::Error
    abort_with(ex.message || ex.to_s, no_pause)
  end

  def self.run_command(cmd : Array(String)) : Int32
    {% if flag?(:windows) %}
      if ENV["QUARK_GUI"]? == "1"
        return run_command_hidden(cmd)
      end
    {% end %}

    QuarkLogs.puts "\nRunning:"
    QuarkLogs.puts cmd.map { |x| x.includes?(' ') ? %("#{x}") : x }.join(' ')
    QuarkLogs.puts

    process = Process.new(
      command: cmd.first,
      args: cmd[1..]?,
      output: Process::Redirect::Pipe,
      error: Process::Redirect::Pipe,
    )
    relay_process_output(process)
    status = process.wait
    status.try(&.exit_code) || 127
  rescue File::NotFoundError
    QuarkLogs.puts "Error: #{cmd.first} was not found."
    127
  end

  def self.relay_process_output(process : Process) : Nil
    done = Channel(Nil).new(2)

    if stdout = process.output
      spawn do
        relay_lines(stdout, STDOUT)
        done.send(nil)
      end
    else
      done.send(nil)
    end

    if stderr = process.error
      spawn do
        relay_lines(stderr, STDERR)
        done.send(nil)
      end
    else
      done.send(nil)
    end

    2.times { done.receive }
  end

  def self.relay_lines(input : IO, output : IO) : Nil
    input.each_line do |line|
      QuarkLogs.puts(line, output)
    end
  rescue IO::Error
  end

  {% if flag?(:windows) %}
    def self.run_command_hidden(cmd : Array(String)) : Int32
      STDOUT.sync = true
      STDERR.sync = true

      runner = Win32HiddenProcess::Runner.new(cmd.first, cmd[1..]? || [] of String)

      relay = ->(input : IO, output : IO) do
        input.each_line do |line|
          begin
            QuarkLogs.puts(line, output)
          rescue IO::Error
            break
          end
        end
      end

      out_done = Channel(Nil).new(1)
      err_done = Channel(Nil).new(1)

      Thread.new(name: "cli-ytdlp-stdout") do
        begin
          relay.call(runner.stdout, STDOUT)
        ensure
          out_done.send(nil)
        end
      end

      Thread.new(name: "cli-ytdlp-stderr") do
        begin
          relay.call(runner.stderr, STDERR)
        ensure
          err_done.send(nil)
        end
      end

      status = runner.wait
      out_done.receive
      err_done.receive
      status.try(&.exit_code) || 127
    rescue File::NotFoundError
      QuarkLogs.puts "Error: #{cmd.first} was not found."
      127
    end
  {% end %}

  def self.press_any_key(no_pause : Bool, message = "Press any key to exit...")
    return if no_pause
    {% if flag?(:windows) %}
      QuarkLogs.puts
      QuarkLogs.puts message
      begin
        STDIN.raw { |io| io.read_byte }
      rescue IO::Error
        gets
      end
    {% end %}
  end

  def self.abort_with(message : String, no_pause : Bool, code = 1) : Nil
    QuarkLogs.puts message
    press_any_key(no_pause)
    QuarkLogs.close
    exit(code)
  end
end
