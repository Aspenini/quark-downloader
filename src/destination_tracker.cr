# Watches yt-dlp output lines for the files it writes, so the naming rules
# can be applied after the download finishes. Also counts ERROR: lines for
# the playlist failure summary.
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
  @lock = Mutex.new

  def observe(line : String) : Nil
    # yt-dlp sometimes emits carriage-return progress on the same line.
    line.split('\r').each do |part|
      part = part.strip
      next if part.empty?

      if part.starts_with?("ERROR:")
        @lock.synchronize { @error_count += 1 }
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
end
