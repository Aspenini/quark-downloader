require "json"

# Shared summary of a download run for CLI, GUI, and logs.
struct DownloadResult
  include JSON::Serializable

  RESULT_PREFIX = "__RESULT__"

  property exit_code : Int32
  property output_dir : String
  property files : Array(String)
  property errors : Array(String)
  property failed_urls : Array(String)
  property log_path : String?
  property playlist_error_count : Int32

  def initialize(
    @exit_code : Int32 = 0,
    @output_dir : String = "",
    @files : Array(String) = [] of String,
    @errors : Array(String) = [] of String,
    @failed_urls : Array(String) = [] of String,
    @log_path : String? = nil,
    @playlist_error_count : Int32 = 0,
  )
  end

  def success? : Bool
    @exit_code == 0 && @failed_urls.empty?
  end

  def to_emit_line : String
    "#{RESULT_PREFIX}#{to_json}"
  end

  def self.parse_emit_line(line : String) : DownloadResult?
    stripped = line.strip
    return nil unless stripped.starts_with?(RESULT_PREFIX)

    from_json(stripped[RESULT_PREFIX.size..])
  rescue
    nil
  end

  # Human-readable body for completion / failure dialogs.
  def dialog_body(max_files : Int32 = 8, max_errors : Int32 = 6) : String
    parts = [] of String

    unless @output_dir.empty?
      parts << "Folder: #{@output_dir}"
    end

    if success?
      if @files.empty?
        parts << "Look for new files in that folder (names may have been sanitized)."
      else
        shown = @files.first(max_files)
        parts << "Saved:"
        shown.each { |f| parts << "  #{f}" }
        if @files.size > max_files
          parts << "  … and #{@files.size - max_files} more"
        end
      end
    else
      if @errors.any?
        parts << "Errors:"
        @errors.last(max_errors).each { |e| parts << "  #{e}" }
      elsif !@failed_urls.empty?
        parts << "Failed URL(s):"
        @failed_urls.first(max_errors).each { |u| parts << "  #{u}" }
      else
        parts << "Download failed (exit code #{@exit_code})."
      end
      if path = @log_path
        parts << ""
        parts << "Log: #{path}"
      end
    end

    if @playlist_error_count > 0
      parts << ""
      parts << "Playlist items failed: #{@playlist_error_count}"
    end

    parts.join('\n')
  end
end
