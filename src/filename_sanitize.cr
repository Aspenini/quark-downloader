module FilenameSanitize
  enum SpacesPolicy
    Keep
    Underscore
    Dash
    Remove
  end

  # Substitutions applied before Unicode normalization. NFKD would fold the
  # fullwidth forms yt-dlp uses as substitutes (e.g. ／ for /) back into
  # characters that are invalid in filenames.
  PRE_TABLE = {
    '｜' => "-", '：' => "-", '＼' => "-", '／' => "-",
    '？' => "", '＊' => "",
    '＂' => "'", '“' => "'", '”' => "'", '‘' => "'", '’' => "'", '´' => "'", '`' => "'",
    '＜' => "(", '＞' => ")",
    '–' => "-", '—' => "-", '−' => "-", '‐' => "-", '・' => "-",
    '…' => "...",
    '×' => "x",
  }

  WINDOWS_RESERVED_NAMES = %w[
    CON PRN AUX NUL
    COM1 COM2 COM3 COM4 COM5 COM6 COM7 COM8 COM9
    LPT1 LPT2 LPT3 LPT4 LPT5 LPT6 LPT7 LPT8 LPT9
  ]

  MAX_COMPONENT_LENGTH = 180

  # Sanitizes a single path component (a file stem or a directory name).
  #
  # Path separators and control characters are always removed; with
  # `ascii_only` the result is reduced to ASCII (accents transliterated via
  # NFKD, remaining non-ASCII dropped) and Windows-invalid characters are
  # replaced. Spaces are preserved unless `spaces` says otherwise.
  def self.sanitize_component(
    name : String,
    ascii_only : Bool = true,
    spaces : SpacesPolicy = SpacesPolicy::Keep,
  ) : String
    result = name
    result = apply_pre_table(result) if ascii_only
    result = remove_control_chars(result)
    result = result.gsub(/[\/\\]/, "-")

    if ascii_only
      result = replace_windows_invalid(result)
      result = String.build do |io|
        result.unicode_normalize(:nfkd).each_char do |char|
          io << char if char.ord < 0x80
        end
      end
      # NFKD can reintroduce invalid characters (e.g. fraction slash -> /).
      result = replace_windows_invalid(result.gsub(/[\/\\]/, "-"))
    end

    result = result.gsub(/\s+/, " ")
    result = apply_spaces_policy(result, spaces)
    result = result.strip(" .")
    result = result[0, MAX_COMPONENT_LENGTH].strip(" .") if result.size > MAX_COMPONENT_LENGTH
    result = "#{result}_" if WINDOWS_RESERVED_NAMES.includes?(result.upcase)
    result = "download" if result.empty?
    result
  end

  # Sanitizes a filename, leaving the extension untouched.
  def self.sanitize_filename(
    filename : String,
    ascii_only : Bool = true,
    spaces : SpacesPolicy = SpacesPolicy::Keep,
  ) : String
    extension = File.extname(filename)
    stem = extension.empty? ? filename : filename[0, filename.size - extension.size]
    sanitize_component(stem, ascii_only, spaces) + extension
  end

  # Returns `filename` if it is free in `dir`, otherwise the first
  # "name (2).ext" .. "name (99).ext" variant that is. Nil when exhausted.
  def self.collision_free(dir : String, filename : String) : String?
    return filename unless File.exists?(File.join(dir, filename))

    extension = File.extname(filename)
    stem = extension.empty? ? filename : filename[0, filename.size - extension.size]

    (2..99).each do |n|
      candidate = "#{stem} (#{n})#{extension}"
      return candidate unless File.exists?(File.join(dir, candidate))
    end

    nil
  end

  private def self.apply_pre_table(text : String) : String
    text.gsub { |char| PRE_TABLE[char]? || char }
  end

  private def self.remove_control_chars(text : String) : String
    text.gsub { |char| (char.ord <= 0x1F || (0x7F..0x9F).includes?(char.ord)) ? "" : char }
  end

  private def self.replace_windows_invalid(text : String) : String
    text.gsub(/[:|]/, "-").gsub(/[<>"?*]/, "")
  end

  private def self.apply_spaces_policy(text : String, spaces : SpacesPolicy) : String
    case spaces
    in .keep?       then text
    in .underscore? then text.gsub(' ', '_')
    in .dash?       then text.gsub(' ', '-')
    in .remove?     then text.gsub(" ", "")
    end
  end
end
