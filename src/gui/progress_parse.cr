module QuarkGui
  PROGRESS_RE        = /\[download\]\s+(\d+(?:\.\d+)?)%/
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
end
