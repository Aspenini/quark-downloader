module QuarkGui
  PROGRESS_RE          = /\[download\]\s+(\d+(?:\.\d+)?)%/
  ETA_RE               = /\bETA\s+([0-9]+:[0-9]{2}(?::[0-9]{2})?|--:--|unknown|n\/a)/i
  QUEUE_URL_RE         = /^==> URL (\d+) of (\d+)/
  PLAYLIST_ITEM_RE     = /\[download\] Downloading item (\d+) of (\d+)/
  SETUP_PROGRESS_MAX   =        8.0
  SETUP_PROGRESS_STEP  =       1.25
  STATUS_DISPLAY_MAX   =         72
  INACTIVITY_NOTICE_MS = 15_000_u64

  def self.parse_progress_percent(line : String) : Float64?
    # yt-dlp sometimes emits carriage-return progress without a newline first.
    line.split('\r').reverse_each do |part|
      if m = part.match(PROGRESS_RE)
        return m[1].to_f
      end
    end

    nil
  end

  def self.parse_eta(line : String) : String?
    line.split('\r').reverse_each do |part|
      if m = part.match(ETA_RE)
        return m[1]
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

  def self.display_download_percent(percent : Float64) : Float64
    bounded = percent.clamp(0.0, 100.0)
    SETUP_PROGRESS_MAX + (bounded * (100.0 - SETUP_PROGRESS_MAX) / 100.0)
  end

  def self.next_setup_progress(current : Float64, line : String) : Float64?
    return nil if parse_progress_percent(line)
    return nil unless parse_status_line(line)
    return nil if current >= SETUP_PROGRESS_MAX

    {current + SETUP_PROGRESS_STEP, SETUP_PROGRESS_MAX}.min
  end

  def self.time_left_text(eta : String?) : String
    eta ? "#{eta} left" : "estimating..."
  end

  def self.eta_status_text(eta : String?) : String
    "Time left: #{time_left_text(eta)}"
  end

  def self.inactivity_status(elapsed_ms : UInt64) : String?
    return nil if elapsed_ms < INACTIVITY_NOTICE_MS

    seconds = elapsed_ms // 1_000
    "Waiting for network/server response (#{seconds}s without output)..."
  end

  def self.format_duration(total_seconds : Int64) : String
    total_seconds = 0_i64 if total_seconds < 0
    hours = total_seconds // 3_600
    minutes = (total_seconds % 3_600) // 60
    seconds = total_seconds % 60
    if hours > 0
      "%d:%02d:%02d" % {hours, minutes, seconds}
    else
      "%d:%02d" % {minutes, seconds}
    end
  end

  # Rough remaining-time estimate for a whole playlist, based on the average
  # wall-clock time per completed item so far. Returns nil until at least one
  # item has finished (we need a sample) or when there is nothing left.
  def self.playlist_eta_text(item : Int32?, total : Int32?, elapsed_ms : UInt64) : String?
    return nil unless item && total && total > 1
    completed = item - 1
    return nil if completed < 1 || elapsed_ms == 0

    remaining = total - completed
    return nil if remaining <= 0

    per_item_ms = elapsed_ms.to_f / completed
    eta_seconds = (remaining * per_item_ms / 1_000.0).to_i64
    "Playlist: ~#{format_duration(eta_seconds)} left"
  end

  class ProgressRelay
    @setup_percent = 0.0
    @download_started = false
    @lock = Mutex.new
    @url_text = ""
    @item_text = ""

    def relay(line : String, output : IO) : Nil
      @lock.synchronize do
        if m = line.match(QUEUE_URL_RE)
          @url_text = "URL #{m[1]} of #{m[2]}"
          @item_text = ""
          @download_started = false
          emit_queue(output)
          emit_progress(output, 0.0)
          emit_eta(output, "")
          @setup_percent = 0.0
          return
        end

        if m = line.match(PLAYLIST_ITEM_RE)
          @item_text = "item #{m[1]} of #{m[2]}"
          @download_started = false
          @setup_percent = 0.0
          emit_queue(output)
          emit_eta(output, "")
        end

        eta = QuarkGui.parse_eta(line)

        if percent = QuarkGui.parse_progress_percent(line)
          @download_started = true
          emit_progress(output, QuarkGui.display_download_percent(percent))
          emit_eta(output, eta) if eta
          return
        end

        status = QuarkGui.parse_status_line(line)
        return unless status

        unless @download_started
          if setup_percent = QuarkGui.next_setup_progress(@setup_percent, line)
            @setup_percent = setup_percent
            emit_progress(output, setup_percent)
          end
        end

        emit_status(output, status)
        emit_eta(output, eta) if eta
      end
    end

    private def emit_progress(output : IO, percent : Float64) : Nil
      output.puts("PROGRESS\t#{percent}")
      output.flush
    end

    private def emit_status(output : IO, status : String) : Nil
      output.puts("STATUS\t#{status}")
      output.flush
    end

    private def emit_eta(output : IO, eta : String) : Nil
      output.puts("ETA\t#{eta}")
      output.flush
    end

    private def emit_queue(output : IO) : Nil
      text = [@url_text, @item_text].reject(&.empty?).join(" - ")
      return if text.empty?

      output.puts("QUEUE\t#{text}")
      output.flush
    end
  end
end
