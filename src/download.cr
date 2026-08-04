require "./config"
require "./logs"
require "./process_status"
require "./ytdlp_tools"
require "./ffmpeg_tools"
require "./filename_sanitize"
require "./playlist"
require "./destination_tracker"
require "./download_result"
require "./term_color"
require "./version_compare"
{% if flag?(:windows) %}
  require "./win32_hidden_process"
{% end %}

module QuarkDownload
  # Grace period before any yt-dlp output (extraction can be quiet).
  STALL_GRACE = 90.seconds
  # Silence after output has started (playlist item kill / single-video warn).
  STALL_ACTIVE = 75.seconds

  record SingleRunOutcome,
    exit_code : Int32,
    files : Array(String),
    errors : Array(String),
    target_dir : String,
    playlist_error_count : Int32

  def self.default_downloads_dir : String
    {% if flag?(:windows) %}
      return File.join(QuarkConfig.user_home, "Downloads")
    {% else %}
      if xdg = xdg_download_dir?
        return xdg
      end
    {% end %}

    File.join(QuarkConfig.user_home, "Downloads")
  end

  def self.xdg_download_dir? : String?
    home = QuarkConfig.user_home
    config = Path[home] / ".config" / "user-dirs.dirs"
    return nil unless File.exists?(config.to_s)

    File.each_line(config.to_s) do |line|
      next unless line.starts_with?("XDG_DOWNLOAD_DIR=")
      if m = line.match(/^XDG_DOWNLOAD_DIR="(.+)"\s*$/)
        return File.expand_path(m[1].gsub("$HOME", home))
      end
    end

    nil
  end

  def self.default_output_dir : String
    QuarkConfig.load!(quiet: true)
    QuarkConfig.download_dir(default_downloads_dir)
  end

  def self.stall_timeout_from_env(default : Time::Span) : Time::Span
    if raw = ENV["QUARK_STALL_TIMEOUT_SEC"]?
      if secs = raw.to_i?
        return secs.seconds if secs > 0
      end
    end
    default
  end

  def self.run(
    url : String,
    media_type : String,
    format : String = "original",
    output_dir : String? = nil,
    no_pause : Bool = false,
    emit_result : Bool = false,
  ) : Int32
    run_all([url], media_type, format, output_dir, no_pause: no_pause, emit_result: emit_result)
  end

  def self.run_all(
    urls : Array(String),
    media_type : String,
    format : String = "original",
    output_dir : String? = nil,
    no_pause : Bool = false,
    emit_result : Bool = false,
  ) : Int32
    result = execute(urls, media_type, format, output_dir, no_pause: no_pause)
    emit_result_line(result) if emit_result || ENV["QUARK_GUI"]? == "1" || ENV["QUARK_EMIT_RESULT"]? == "1"
    result.exit_code
  end

  def self.execute(
    urls : Array(String),
    media_type : String,
    format : String = "original",
    output_dir : String? = nil,
    no_pause : Bool = false,
  ) : DownloadResult
    {% if flag?(:windows) %}
      if ENV["QUARK_GUI"]? == "1"
        STDOUT.sync = true
        STDERR.sync = true
      end
    {% end %}

    result = DownloadResult.new

    begin
      QuarkConfig.load!(quiet: true)
    rescue ex : QuarkConfig::ConfigError
      result.exit_code = 1
      result.errors << (ex.message || ex.to_s)
      STDERR.puts TermColor.red(ex.message || ex.to_s)
      return result
    end

    QuarkLogs.open_download_log
    result.log_path = QuarkLogs.active_path.try(&.to_s)
    warn_if_root!
    warn_if_unwritable_config!

    begin
      media_type = media_type.downcase
      unless {"audio", "video"}.includes?(media_type)
        return fail_result(result, "Invalid media type: #{media_type.inspect} (expected audio or video)", no_pause)
      end

      format = format.downcase
      format = "original" if format.empty?

      dir = output_dir || QuarkConfig.download_dir(default_downloads_dir)
      output_path = Path[File.expand_path(dir)]
      result.output_dir = output_path.to_s

      ytdlp = begin
        preflight!(urls, media_type, format, output_path)
      rescue ex : YtDlpTools::Error | FfmpegTools::Error | PreflightError
        return fail_result(result, ex.message || ex.to_s, no_pause)
      end

      multi = urls.size > 1
      failed = [] of {String, Int32}

      urls.each_with_index do |url, index|
        QuarkLogs.puts "\n#{TermColor.bold("==> URL #{index + 1} of #{urls.size}")}: #{url}" if multi

        outcome = begin
          run_single(ytdlp, url, media_type, format, output_path)
        rescue ex : YtDlpTools::Error | FfmpegTools::Error
          msg = ex.message || ex.to_s
          if multi
            QuarkLogs.puts TermColor.red(msg)
            SingleRunOutcome.new(1, [] of String, [msg], output_path.to_s, 0)
          else
            return fail_result(result, msg, no_pause)
          end
        end

        result.files.concat(outcome.files)
        result.errors.concat(outcome.errors)
        result.playlist_error_count += outcome.playlist_error_count
        result.output_dir = outcome.target_dir if urls.size == 1

        unless outcome.exit_code == 0
          failed << {url, outcome.exit_code}
          result.failed_urls << url
        end
      end

      result.files.uniq!
      result.errors.uniq!

      if multi
        QuarkLogs.puts
        ok = urls.size - failed.size
        summary = "==> Finished: #{ok} of #{urls.size} succeeded."
        QuarkLogs.puts(failed.empty? ? TermColor.green(summary) : TermColor.yellow(summary))
        failed.each { |(u, _)| QuarkLogs.puts TermColor.red("  failed: #{u}") }
        if failed.any? { |(u, _)| YtDlpTools.youtube_url?(u) }
          QuarkLogs.puts
          QuarkLogs.puts YtDlpTools.youtube_failure_hints
        end
        press_any_key(no_pause)
        result.exit_code = failed.empty? ? 0 : 1
        return result
      end

      if failed.empty?
        QuarkLogs.puts TermColor.green("Done.")
        press_any_key(no_pause)
        result.exit_code = 0
        result
      else
        _, code = failed.first
        message = "Failed with exit code #{code}."
        message += "\n\n#{YtDlpTools.youtube_failure_hints}" if YtDlpTools.youtube_url?(failed.first[0])
        fail_result(result, message, no_pause, code)
      end
    ensure
      QuarkLogs.close
    end
  end

  class PreflightError < Exception; end

  # Fail-fast checks before any long download work.
  def self.preflight!(
    urls : Array(String),
    media_type : String,
    format : String,
    output_path : Path,
  ) : String
    begin
      Dir.mkdir_p(output_path.to_s)
    rescue ex
      raise PreflightError.new("Cannot create output directory:\n  #{output_path}\n#{ex.message}")
    end

    unless dir_writable?(output_path)
      raise PreflightError.new(<<-MSG)
        Output directory is not writable:
          #{output_path}
        Choose another folder (do not use sudo to "fix" permissions).
        MSG
    end

    ytdlp = YtDlpTools.ensure!

    needs_ffmpeg = !{"original", "default.original"}.includes?(format)
    if needs_ffmpeg
      begin
        FfmpegTools.ensure!
      rescue ex : FfmpegTools::Error
        raise PreflightError.new(ex.message || ex.to_s)
      end
    else
      FfmpegTools.detect!
    end

    urls.each do |url|
      YtDlpTools.preflight_youtube!(url)
    end

    if urls.any? { |u| YtDlpTools.youtube_url?(u) }
      if version = YtDlpTools.read_version(ytdlp)
        unless VersionCompare.at_least?(version, YtDlpTools::MIN_YOUTUBE_YTDLP)
          QuarkLogs.puts TermColor.yellow(
            "Warning: yt-dlp #{version} is likely too old for YouTube (want >= #{YtDlpTools::MIN_YOUTUBE_YTDLP})."
          )
        end
      end
    end

    ytdlp
  end

  def self.dir_writable?(dir : Path) : Bool
    path = dir.to_s
    return false unless File.directory?(path)

    probe = File.join(path, ".quark-write-test-#{Process.pid}")
    begin
      File.write(probe, "ok")
      File.delete(probe)
      true
    rescue
      File.delete?(probe) if File.exists?(probe)
      false
    end
  end

  def self.run_single(
    ytdlp : String,
    url : String,
    media_type : String,
    format : String,
    output_path : Path,
  ) : SingleRunOutcome
    settings = QuarkConfig.settings
    playlist = QuarkPlaylist.playlist_url?(url)
    target_dir = output_path

    if playlist && settings.playlist_folders
      if probe = QuarkPlaylist.probe(ytdlp, url, YtDlpTools.extra_args(url))
        folder = FilenameSanitize.sanitize_component(
          probe.title,
          settings.sanitize_filenames,
          settings.filename_spaces.to_policy,
        )
        candidate = output_path / folder
        begin
          Dir.mkdir_p(candidate.to_s)
          target_dir = candidate
          count_note = probe.count ? " (#{probe.count} items)" : ""
          QuarkLogs.puts "Playlist: #{probe.title}#{count_note}"
          QuarkLogs.puts "Saving into: #{TermColor.cyan(target_dir.to_s)}"
        rescue ex
          QuarkLogs.puts TermColor.yellow("Warning: could not create playlist folder #{candidate}: #{ex.message}")
        end
      else
        QuarkLogs.puts TermColor.yellow("Warning: could not read playlist info; downloading without a playlist folder.")
      end
    end

    name_template = settings.strip_video_ids ? "%(title)s.%(ext)s" : "%(title)s [%(id)s].%(ext)s"
    outtmpl = (target_dir / name_template).to_s

    cmd = [ytdlp]
    cmd.concat(playlist ? ["--yes-playlist", "--ignore-errors"] : ["--no-playlist"])
    cmd.concat(["-o", outtmpl])
    cmd.concat(["--socket-timeout", "30", "--retries", "3", "--fragment-retries", "3"])

    if media_type == "audio"
      cmd.concat(["-f", "bestaudio/best"])
      unless {"original", "default.original"}.includes?(format)
        FfmpegTools.append_to_cmd!(cmd)
        cmd.concat(["-x", "--audio-format", format])
      end
    else
      unless {"original", "default.original"}.includes?(format)
        FfmpegTools.append_to_cmd!(cmd)
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

    tracker = DestinationTracker.new
    active_timeout = stall_timeout_from_env(STALL_ACTIVE)
    grace_timeout = stall_timeout_from_env(STALL_GRACE)

    exit_code = if playlist
                  run_playlist(cmd, url, tracker, active_timeout, grace_timeout)
                else
                  monitor = StallMonitor.new(kill_on_stall: false)
                  run_command(cmd + [url], tracker, monitor, active_timeout, grace_timeout)
                end

    final_paths = apply_naming!(tracker, output_path, settings)

    if playlist && tracker.error_count > 0
      QuarkLogs.puts TermColor.yellow("Playlist finished: #{tracker.error_count} item(s) failed.")
    end

    if exit_code == 0 || final_paths.any?
      report_saved_files(final_paths, target_dir)
    end

    SingleRunOutcome.new(
      exit_code,
      final_paths,
      tracker.errors,
      target_dir.to_s,
      tracker.error_count,
    )
  end

  def self.report_saved_files(paths : Array(String), target_dir : Path) : Nil
    QuarkLogs.puts "#{TermColor.bold("Output folder:")} #{TermColor.cyan(target_dir.to_s)}"

    existing = paths.select do |path|
      begin
        !path.ends_with?(".part") && !path.ends_with?(".ytdl") && File.file?(path)
      rescue
        false
      end
    end

    if existing.empty?
      QuarkLogs.puts TermColor.dim("Look for new files under that folder (names may have been sanitized).")
    else
      QuarkLogs.puts TermColor.bold("Saved file(s):")
      existing.each { |p| QuarkLogs.puts "  #{TermColor.cyan(p)}" }
    end
  end

  def self.warn_if_root! : Nil
    {% unless flag?(:windows) %}
      if LibC.getuid == 0
        QuarkLogs.puts TermColor.yellow("Warning: running as root/sudo.")
        QuarkLogs.puts TermColor.yellow("  Config and downloads use root's home (#{QuarkConfig.user_home}), not your user account.")
        QuarkLogs.puts TermColor.yellow("  Re-run without sudo so files land in your Downloads.")
      end
    {% end %}
  end

  def self.warn_if_unwritable_config! : Nil
    {% unless flag?(:windows) %}
      return if LibC.getuid == 0

      path = QuarkConfig.config_dir.to_s
      return unless File.directory?(path)
      return if File.writable?(path)

      QuarkLogs.puts TermColor.yellow("Warning: config directory is not writable:")
      QuarkLogs.puts TermColor.yellow("  #{path}")
      QuarkLogs.puts TermColor.yellow("  If you previously ran with sudo, fix ownership (do not keep using sudo):")
      QuarkLogs.puts TermColor.yellow("  sudo chown -R \"$USER\" #{path}")
    {% end %}
  end

  def self.apply_naming!(tracker : DestinationTracker, output_path : Path, settings : QuarkConfig::Settings) : Array(String)
    policy = settings.filename_spaces.to_policy
    base = File.expand_path(output_path.to_s)
    finals = [] of String

    tracker.paths.each do |path|
      begin
        next if path.ends_with?(".part") || path.ends_with?(".ytdl")

        expanded = File.expand_path(path)
        next unless expanded == base || expanded.starts_with?(base + File::SEPARATOR)
        next unless File.file?(expanded)

        unless settings.sanitize_filenames || !policy.keep?
          finals << expanded
          next
        end

        dir = File.dirname(expanded)
        name = File.basename(expanded)
        new_name = FilenameSanitize.sanitize_filename(
          name,
          settings.sanitize_filenames,
          policy,
        )
        if new_name == name
          finals << expanded
          next
        end

        final = FilenameSanitize.collision_free(dir, new_name)
        unless final
          finals << expanded
          next
        end

        dest = File.join(dir, final)
        File.rename(expanded, dest)
        QuarkLogs.puts "Renamed: #{name} -> #{final}"
        finals << dest
      rescue ex
        QuarkLogs.puts TermColor.yellow("Warning: could not rename #{path}: #{ex.message}")
      end
    end

    finals
  end

  PLAYLIST_ITEM_LINE_RE = /^\[download\] Downloading item (\d+) of (\d+)/
  POSTPROCESS_RE       = /^\[(?:Merger|ExtractAudio|VideoConvertor|VideoRemuxer|Recode|Fixup\w*|Metadata|EmbedSubtitle|EmbedThumbnail|SponsorBlock|ModifyChapters|SplitChapters)\]/
  RESUME_RE            = /^\[download\]|Extracting URL/
  PROGRESS_HINT_RE     = /(\d+(?:\.\d+)?)%|Downloading item|Destination:/

  # Watches a single yt-dlp run for silence. Grace timeout applies before any
  # output; active timeout after. Playlist runs may kill on stall; single
  # video runs only warn (kill_on_stall: false).
  class StallMonitor
    @lock = Mutex.new
    @last = Time.instant
    @started = Time.instant
    @had_output = false
    @suspended = false
    @killed = false
    @finished = false
    @warned = false
    getter current_item : Int32?
    getter total_items : Int32?

    def initialize(
      @offset : Int32 = 0,
      @total_items : Int32? = nil,
      @kill_on_stall : Bool = true,
    )
    end

    def observe(line : String) : String
      @lock.synchronize do
        @last = Time.instant
        @had_output = true
        if m = line.match(PLAYLIST_ITEM_LINE_RE)
          @total_items ||= m[2].to_i + @offset
          abs = m[1].to_i + @offset
          @current_item = abs
          @suspended = false
          return line.sub(PLAYLIST_ITEM_LINE_RE, "[download] Downloading item #{abs} of #{@total_items}")
        end

        if line.matches?(POSTPROCESS_RE)
          @suspended = true
        elsif line.matches?(RESUME_RE) || line.matches?(PROGRESS_HINT_RE)
          @suspended = false
        end
        line
      end
    end

    def stalled?(active_timeout : Time::Span, grace_timeout : Time::Span) : Bool
      @lock.synchronize do
        return false if @suspended || @finished
        timeout = @had_output ? active_timeout : grace_timeout
        anchor = @had_output ? @last : @started
        Time.instant - anchor >= timeout
      end
    end

    def kill_on_stall? : Bool
      @kill_on_stall
    end

    def mark_killed : Nil
      @lock.synchronize { @killed = true }
    end

    def mark_warned : Nil
      @lock.synchronize { @warned = true }
    end

    def warned? : Bool
      @lock.synchronize { @warned }
    end

    def finish : Nil
      @lock.synchronize { @finished = true }
    end

    def killed? : Bool
      @lock.synchronize { @killed }
    end

    def finished? : Bool
      @lock.synchronize { @finished }
    end

    def had_output? : Bool
      @lock.synchronize { @had_output }
    end
  end

  def self.run_playlist(
    opts : Array(String),
    url : String,
    tracker : DestinationTracker,
    active_timeout : Time::Span,
    grace_timeout : Time::Span,
  ) : Int32
    total : Int32? = nil
    start = 1
    exit_code = 0

    loop do
      cmd = opts.dup
      cmd.concat(["--playlist-items", "#{start}:"]) if start > 1
      cmd << url

      monitor = StallMonitor.new(offset: start - 1, total_items: total, kill_on_stall: true)
      exit_code = run_command(cmd, tracker, monitor, active_timeout, grace_timeout)
      total ||= monitor.total_items

      break unless monitor.killed?

      item = monitor.current_item
      unless item
        QuarkLogs.puts TermColor.yellow("\nStopped: no response from the server.")
        break
      end

      secs = active_timeout.total_seconds.to_i
      QuarkLogs.puts TermColor.yellow("\nSkipping item #{item}: no response for #{secs}s.")
      start = item + 1
      break if (t = total) && start > t
    end

    exit_code
  end

  def self.run_command(
    cmd : Array(String),
    tracker : DestinationTracker? = nil,
    monitor : StallMonitor? = nil,
    active_timeout : Time::Span? = nil,
    grace_timeout : Time::Span? = nil,
  ) : Int32
    {% if flag?(:windows) %}
      if ENV["QUARK_GUI"]? == "1"
        return run_command_hidden(cmd, tracker, monitor, active_timeout, grace_timeout)
      end
    {% end %}

    QuarkLogs.puts
    QuarkLogs.puts TermColor.dim("Running:")
    QuarkLogs.puts TermColor.dim(cmd.map { |x| x.includes?(' ') ? %("#{x}") : x }.join(' '))
    QuarkLogs.puts

    process = Process.new(
      command: cmd.first,
      args: cmd[1..]?,
      output: Process::Redirect::Pipe,
      error: Process::Redirect::Pipe,
    )

    if monitor && (active = active_timeout) && (grace = grace_timeout)
      spawn do
        loop do
          sleep 1.second
          break if monitor.finished?
          if monitor.stalled?(active, grace)
            if monitor.kill_on_stall?
              monitor.mark_killed
              process.terminate rescue nil
              break
            elsif !monitor.warned?
              monitor.mark_warned
              QuarkLogs.puts TermColor.yellow("\nWarning: no response for a while; still waiting…")
            end
          end
        end
      end
    end

    relay_process_output(process, tracker, monitor)
    status = process.wait
    monitor.try(&.finish)
    QuarkProcess.exit_code(status, 127)
  rescue File::NotFoundError
    QuarkLogs.puts TermColor.red("Error: #{cmd.first} was not found.")
    127
  end

  def self.relay_process_output(process : Process, tracker : DestinationTracker? = nil, monitor : StallMonitor? = nil) : Nil
    done = Channel(Nil).new(2)

    if stdout = process.output
      spawn do
        relay_lines(stdout, STDOUT, tracker, monitor)
        done.send(nil)
      end
    else
      done.send(nil)
    end

    if stderr = process.error
      spawn do
        relay_lines(stderr, STDERR, tracker, monitor)
        done.send(nil)
      end
    else
      done.send(nil)
    end

    2.times { done.receive }
  end

  def self.relay_lines(input : IO, output : IO, tracker : DestinationTracker? = nil, monitor : StallMonitor? = nil) : Nil
    input.each_line do |line|
      out_line = monitor ? monitor.observe(line) : line
      tracker.try(&.observe(out_line))
      QuarkLogs.puts(out_line, output)
    end
  rescue IO::Error
  end

  {% if flag?(:windows) %}
    def self.run_command_hidden(
      cmd : Array(String),
      tracker : DestinationTracker? = nil,
      monitor : StallMonitor? = nil,
      active_timeout : Time::Span? = nil,
      grace_timeout : Time::Span? = nil,
    ) : Int32
      STDOUT.sync = true
      STDERR.sync = true

      runner = Win32HiddenProcess::Runner.new(cmd.first, cmd[1..]? || [] of String)

      relay = ->(input : IO, output : IO) do
        input.each_line do |line|
          begin
            out_line = monitor ? monitor.observe(line) : line
            tracker.try(&.observe(out_line))
            QuarkLogs.puts(out_line, output)
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

      if monitor && (active = active_timeout) && (grace = grace_timeout)
        Thread.new(name: "cli-ytdlp-watchdog") do
          loop do
            break if runner.wait(1000_u32)
            if monitor.stalled?(active, grace)
              if monitor.kill_on_stall?
                monitor.mark_killed
                runner.terminate
                break
              elsif !monitor.warned?
                monitor.mark_warned
                QuarkLogs.puts "\nWarning: no response for a while; still waiting…"
              end
            end
          end
        end
      end

      status = runner.wait
      monitor.try(&.finish)
      out_done.receive
      err_done.receive
      QuarkProcess.exit_code(status, 127)
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

  def self.fail_result(result : DownloadResult, message : String, no_pause : Bool, code = 1) : DownloadResult
    QuarkLogs.puts TermColor.red(message)
    result.errors << message unless result.errors.includes?(message)
    result.exit_code = code
    press_any_key(no_pause)
    result
  end

  def self.emit_result_line(result : DownloadResult) : Nil
    puts result.to_emit_line
    STDOUT.flush
  rescue IO::Error
  end
end
