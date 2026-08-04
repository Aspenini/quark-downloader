# Watches yt-dlp output lines for the files it writes, so the naming rules
# can be applied after the download finishes. Also records ERROR: lines for
# the playlist failure summary and GUI completion dialogs.
class DestinationTracker
  DESTINATION_PATTERNS = [
    /^\[download\] Destination: (.+)$/,
    /^\[Merger\] Merging formats into "(.+)"$/,
    /^\[ExtractAudio\] Destination: (.+)$/,
    /^\[VideoConvertor\] Destination: (.+)$/,
    /^\[VideoRemuxer\] Destination: (.+)$/,
    /^\[download\] (.+) has already been downloaded$/,
  ]

  getter error_count = 0

  @paths = [] of String
  @errors = [] of String
  @lock = Mutex.new

  def observe(line : String) : Nil
    # yt-dlp sometimes emits carriage-return progress on the same line.
    line.split('\r').each do |part|
      part = part.strip
      next if part.empty?

      if part.starts_with?("ERROR:")
        @lock.synchronize do
          @error_count += 1
          @errors << part unless @errors.includes?(part)
        end
        next
      end

      DESTINATION_PATTERNS.each do |pattern|
        if m = part.match(pattern)
          @lock.synchronize do
            @paths << m[1] unless @paths.includes?(m[1])
          end
          break
        end
      end
    end
  end

  def paths : Array(String)
    @lock.synchronize { @paths.dup }
  end

  def errors : Array(String)
    @lock.synchronize { @errors.dup }
  end
end
