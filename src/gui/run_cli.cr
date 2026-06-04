require "../config"
require "./gui_logs"

{% if flag?(:windows) %}
require "./win32_progress"
{% else %}
require "./tk_ui"
{% end %}

module QuarkGui
  struct DownloadParams
    property url : String
    property media_type : String
    property format : String
    property output_dir : String

    def initialize(@url, @media_type, @format, @output_dir)
    end
  end

  def self.build_cli_args(cli : String, params : DownloadParams) : Array(String)
    [
      cli,
      "--url", params.url,
      "--type", params.media_type,
      "--format", params.format,
      "--output-dir", params.output_dir,
      "--no-pause",
    ]
  end

  def self.default_output_dir(cli : String) : String
    {% if flag?(:windows) %}
      # Avoid spawning the console CLI during GUI startup (visible flash).
      QuarkConfig.load!(quiet: true)
      QuarkConfig.download_dir(File.join(ENV["USERPROFILE"]? || ".", "Downloads"))
    {% else %}
      output = IO::Memory.new
      status = Process.run(
        cli,
        args: ["--print-default-output-dir"],
        output: output,
        error: Process::Redirect::Close,
      )
      if status.try(&.success?) && !output.to_s.strip.empty?
        return output.to_s.strip
      end

      home = ENV["HOME"]? || "."
      File.join(home, "Downloads")
    {% end %}
  end

  def self.run_download(cli : String, params : DownloadParams) : Int32
    args = build_cli_args(cli, params)
    command = args.first
    cmd_args = args[1..]? || [] of String
    run_download_with_progress(command, cmd_args)
  end

  PROGRESS_RE = /\[download\]\s+(\d+(?:\.\d+)?)%/
  STATUS_DISPLAY_MAX = 72

  def self.parse_progress_percent(line : String) : Float64?
    if m = line.match(PROGRESS_RE)
      return m[1].to_f
    end

    # yt-dlp sometimes emits carriage-return progress without a newline first.
    line.split('\r').each do |part|
      if m = part.match(PROGRESS_RE)
        return m[1].to_f
      end
    end

    nil
  end

  def self.parse_status_line(line : String) : String?
    stripped = line.strip
    return nil if stripped.empty?
    return nil if stripped == "Done."
    return nil if stripped.starts_with?("Deleting original file")

    if stripped.size > STATUS_DISPLAY_MAX
      stripped = stripped[0, STATUS_DISPLAY_MAX - 3] + "..."
    end
    stripped
  end

  def self.run_download_with_progress(command : String, cmd_args : Array(String)) : Int32
    {% if flag?(:windows) %}
      Win32Progress.run(command, cmd_args)
    {% else %}
      run_download_with_progress_unix(command, cmd_args)
    {% end %}
  end

  {% unless flag?(:windows) %}
  def self.run_download_with_progress_unix(command : String, cmd_args : Array(String)) : Int32
    log_file, _log_path = GuiLogs.open_log

    wish = TkUi.ensure_wish!
    script = TkUi.ensure_script!

    progress = Process.new(
      wish,
      args: [script, "--progress", GuiLogs.logs_dir.to_s],
      input: Process::Redirect::Pipe,
      output: Process::Redirect::Close,
      error: Process::Redirect::Close,
    )
    prog_in = progress.input.not_nil!

    env = ENV.to_h.merge({"QUARK_GUI" => "1"})
    cli_process = Process.new(
      command: command,
      args: cmd_args,
      env: env,
      output: Process::Redirect::Pipe,
      error: Process::Redirect::Pipe,
    )

    done = Channel(Nil).new(2)

    if stdout = cli_process.output
      spawn do
        stdout.each_line do |line|
          relay_output_line(line, log_file, prog_in)
        end
        done.send(nil)
      end
    else
      done.send(nil)
    end

    if stderr = cli_process.error
      spawn do
        stderr.each_line do |line|
          relay_output_line(line, log_file, prog_in)
        end
        done.send(nil)
      end
    else
      done.send(nil)
    end

    2.times { done.receive }
    status = cli_process.wait
    exit_code = if status.success?
                  0
                else
                  status.exit_code || 1
                end

    prog_in.puts("DONE\t#{exit_code}")
    prog_in.flush
    prog_in.close

    progress.wait
    if cli_process.exists?
      cli_process.terminate
      cli_process.wait
    end
    log_file.close

    TkUi.show_completion(exit_code == 0, exit_code)
    exit_code
  end

  def self.relay_output_line(line : String, log_file : File, prog_in : IO) : Nil
    log_file.puts(line)
    log_file.flush

    if percent = parse_progress_percent(line)
      prog_in.puts("PROGRESS\t#{percent}")
      prog_in.flush
      return
    end

    if status = parse_status_line(line)
      prog_in.puts("STATUS\t#{status}")
      prog_in.flush
    end
  end
  {% end %}

end
