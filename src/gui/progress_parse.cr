module QuarkGui
  PROGRESS_RE         = /\[download\]\s+(\d+(?:\.\d+)?)%/
  ETA_RE              = /\bETA\s+([0-9]+:[0-9]{2}(?::[0-9]{2})?|--:--|unknown|n\/a)/i
  SETUP_PROGRESS_MAX  =  8.0
  SETUP_PROGRESS_STEP = 1.25
  STATUS_DISPLAY_MAX  =   72

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

  class ProgressRelay
    @setup_percent = 0.0
    @download_started = false
    @lock = Mutex.new

    def relay(line : String, output : IO) : Nil
      @lock.synchronize do
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
  end
end
