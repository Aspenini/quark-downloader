require "./config"
require "./logs"
require "./process_status"
require "./ytdlp_tools"
require "./ffmpeg_tools"
require "./filename_sanitize"
require "./playlist"
require "./destination_tracker"
{% if flag?(:windows) %}
  require "./win32_hidden_process"
{% end %}

module QuarkDownload
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

  def self.run(
    url : String,
    media_type : String,
    format : String = "original",
    output_dir : String? = nil,
    no_pause : Bool = false,
  ) : Int32
    run_all([url], media_type, format, output_dir, no_pause: no_pause)
  end

  def self.run_all(
    urls : Array(String),
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

      multi = urls.size > 1
      failed = [] of {String, Int32}

      urls.each_with_index do |url, index|
        QuarkLogs.puts "\n==> URL #{index + 1} of #{urls.size}: #{url}" if multi

        code = begin
          run_single(ytdlp, url, media_type, format, output_path)
        rescue ex : YtDlpTools::Error | FfmpegTools::Error
          if multi
            QuarkLogs.puts(ex.message || ex.to_s)
            1
          else
            abort_with(ex.message || ex.to_s, no_pause)
          end
        end

        failed << {url, code} unless code == 0
      end

      if multi
        QuarkLogs.puts
        QuarkLogs.puts "==> Finished: #{urls.size - failed.size} of #{urls.size} succeeded."
        failed.each { |(u, _)| QuarkLogs.puts "  failed: #{u}" }
        if failed.any? { |(u, _)| YtDlpTools.youtube_url?(u) }
          QuarkLogs.puts
          QuarkLogs.puts YtDlpTools.youtube_failure_hints
        end
        press_any_key(no_pause)
        return failed.empty? ? 0 : 1
      end

      if failed.empty?
        QuarkLogs.puts "Done."
        press_any_key(no_pause)
        0
      else
        url, code = failed.first
        message = "Failed with exit code #{code}."
        message += "\n\n#{YtDlpTools.youtube_failure_hints}" if YtDlpTools.youtube_url?(url)
        abort_with(message, no_pause, code)
      end
    ensure
      QuarkLogs.close
    end
  end

  def self.run_single(
    ytdlp : String,
    url : String,
    media_type : String,
    format : String,
    output_path : Path,
  ) : Int32
    YtDlpTools.preflight_youtube!(url)

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
          QuarkLogs.puts "Saving into: #{target_dir}"
        rescue ex
          QuarkLogs.puts "Warning: could not create playlist folder #{candidate}: #{ex.message}"
        end
      else
        QuarkLogs.puts "Warning: could not read playlist info; downloading without a playlist folder."
      end
    end

    name_template = settings.strip_video_ids ? "%(title)s.%(ext)s" : "%(title)s [%(id)s].%(ext)s"
    outtmpl = (target_dir / name_template).to_s

    cmd = [ytdlp]
    cmd.concat(playlist ? ["--yes-playlist", "--ignore-errors"] : ["--no-playlist"])
    cmd.concat(["-o", outtmpl])
    # A stalled connection must not block forever. --socket-timeout turns a silent
    # socket into an error instead of an infinite read; a few quick retries cover
    # transient blips. If yt-dlp still produces no output at all, the stall
    # watchdog in run_command kills it after STALL_TIMEOUT (see run_playlist).
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
    exit_code = if playlist
                  run_playlist(cmd, url, tracker)
                else
                  run_command(cmd + [url], tracker, StallMonitor.new, STALL_TIMEOUT)
                end

    apply_naming!(tracker, output_path, settings)

    if playlist && tracker.error_count > 0
      QuarkLogs.puts "Playlist finished: #{tracker.error_count} item(s) failed."
    end

    exit_code
  end

  # Renames downloaded files according to the download-naming settings.
  # Only touches files yt-dlp reported, that still exist, inside the output
  # directory. Failures are logged and never abort the download.
  def self.apply_naming!(tracker : DestinationTracker, output_path : Path, settings : QuarkConfig::Settings) : Nil
    policy = settings.filename_spaces.to_policy
    return unless settings.sanitize_filenames || !policy.keep?

    base = File.expand_path(output_path.to_s)

    tracker.paths.each do |path|
      begin
        next if path.ends_with?(".part") || path.ends_with?(".ytdl")

        expanded = File.expand_path(path)
        next unless expanded == base || expanded.starts_with?(base + File::SEPARATOR)
        next unless File.file?(expanded)

        dir = File.dirname(expanded)
        name = File.basename(expanded)
        new_name = FilenameSanitize.sanitize_filename(
          name,
          settings.sanitize_filenames,
          policy,
        )
        next if new_name == name

        final = FilenameSanitize.collision_free(dir, new_name)
        next unless final

        File.rename(expanded, File.join(dir, final))
        QuarkLogs.puts "Renamed: #{name} -> #{final}"
      rescue ex
        QuarkLogs.puts "Warning: could not rename #{path}: #{ex.message}"
      end
    end
  end

  # How long yt-dlp may produce no output at all before we treat the current
  # item as stuck, kill it, and move on. yt-dlp prints progress/extraction lines
  # every second or two while it is working, so this much total silence means a
  # wedged connection rather than a slow-but-alive download.
  STALL_TIMEOUT = 30.seconds

  PLAYLIST_ITEM_LINE_RE = /^\[download\] Downloading item (\d+) of (\d+)/
  # ffmpeg post-processing (merge/recode/extract) runs silently and can take far
  # longer than STALL_TIMEOUT; suspend the watchdog while it is in progress.
  POSTPROCESS_RE = /^\[(?:Merger|ExtractAudio|VideoConvertor|VideoRemuxer|Recode|Fixup\w*|Metadata|EmbedSubtitle|EmbedThumbnail|SponsorBlock|ModifyChapters|SplitChapters)\]/
  RESUME_RE      = /^\[download\]|Extracting URL/

  # Watches a single yt-dlp run: tracks the last time it emitted output, the
  # current playlist item, and whether the run is in silent post-processing.
  # When a playlist is restarted past a stuck item, item numbers are rewritten
  # back to absolute (yt-dlp renumbers from 1 after --playlist-start).
  class StallMonitor
    @lock = Mutex.new
    @last = Time.instant
    @suspended = false
    @killed = false
    @finished = false
    getter current_item : Int32?
    getter total_items : Int32?

    def initialize(@offset : Int32 = 0, @total_items : Int32? = nil)
    end

    def observe(line : String) : String
      @lock.synchronize do
        @last = Time.instant
        if m = line.match(PLAYLIST_ITEM_LINE_RE)
          @total_items ||= m[2].to_i + @offset
          abs = m[1].to_i + @offset
          @current_item = abs
          @suspended = false
          return line.sub(PLAYLIST_ITEM_LINE_RE, "[download] Downloading item #{abs} of #{@total_items}")
        end

        if line.matches?(POSTPROCESS_RE)
          @suspended = true
        elsif line.matches?(RESUME_RE)
          @suspended = false
        end
        line
      end
    end

    def stalled?(timeout : Time::Span) : Bool
      @lock.synchronize do
        return false if @suspended || @finished
        Time.instant - @last >= timeout
      end
    end

    def mark_killed : Nil
      @lock.synchronize { @killed = true }
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
  end

  # Runs a playlist, restarting past any item that goes silent for STALL_TIMEOUT.
  # Already-downloaded items are skipped instantly on restart, so resuming from
  # the next item is cheap. A stuck item is abandoned so the rest still downloads.
  def self.run_playlist(opts : Array(String), url : String, tracker : DestinationTracker) : Int32
    total : Int32? = nil
    start = 1
    exit_code = 0

    loop do
      cmd = opts.dup
      # "N:" selects item N through the end; yt-dlp renumbers the slice from 1,
      # which StallMonitor's offset rewrites back to absolute item numbers.
      cmd.concat(["--playlist-items", "#{start}:"]) if start > 1
      cmd << url

      monitor = StallMonitor.new(offset: start - 1, total_items: total)
      exit_code = run_command(cmd, tracker, monitor, STALL_TIMEOUT)
      total ||= monitor.total_items

      break unless monitor.killed?

      item = monitor.current_item
      unless item
        QuarkLogs.puts "\nStopped: no response from the server."
        break
      end

      QuarkLogs.puts "\nSkipping item #{item}: no response for #{STALL_TIMEOUT.total_seconds.to_i}s."
      start = item + 1
      break if (t = total) && start > t
    end

    exit_code
  end

  def self.run_command(
    cmd : Array(String),
    tracker : DestinationTracker? = nil,
    monitor : StallMonitor? = nil,
    stall_timeout : Time::Span? = nil,
  ) : Int32
    {% if flag?(:windows) %}
      if ENV["QUARK_GUI"]? == "1"
        return run_command_hidden(cmd, tracker, monitor, stall_timeout)
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

    if monitor && (timeout = stall_timeout)
      spawn do
        loop do
          sleep 1.second
          break if monitor.finished?
          if monitor.stalled?(timeout)
            monitor.mark_killed
            process.terminate rescue nil
            break
          end
        end
      end
    end

    relay_process_output(process, tracker, monitor)
    status = process.wait
    monitor.try(&.finish)
    QuarkProcess.exit_code(status, 127)
  rescue File::NotFoundError
    QuarkLogs.puts "Error: #{cmd.first} was not found."
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
      stall_timeout : Time::Span? = nil,
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

      if monitor && (timeout = stall_timeout)
        Thread.new(name: "cli-ytdlp-watchdog") do
          loop do
            break if runner.wait(1000_u32) # finished within the second
            if monitor.stalled?(timeout)
              monitor.mark_killed
              runner.terminate
              break
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

  def self.abort_with(message : String, no_pause : Bool, code = 1) : Nil
    QuarkLogs.puts message
    press_any_key(no_pause)
    QuarkLogs.close
    exit(code)
  end
end
