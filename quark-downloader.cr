def user_home : String
  ENV["USERPROFILE"]? || ENV["HOME"]? || "."
end

def default_downloads_dir : String
  {% if flag?(:windows) %}
    return File.join(user_home, "Downloads")
  {% else %}
    if xdg = xdg_download_dir?
      return xdg
    end
  {% end %}

  File.join(user_home, "Downloads")
end

def xdg_download_dir? : String?
  config = Path[user_home] / ".config" / "user-dirs.dirs"
  return nil unless config.exists?

  File.each_line(config) do |line|
    next unless line.starts_with?("XDG_DOWNLOAD_DIR=")
    if m = line.match(/^XDG_DOWNLOAD_DIR="(.+)"\s*$/)
      return File.expand_path(m[1].gsub("$HOME", user_home))
    end
  end

  nil
end

def press_any_key(message = "Press any key to exit...")
  puts
  puts message
  {% if flag?(:windows) %}
    STDIN.raw { |io| io.read_byte }
  {% else %}
    gets
  {% end %}
end

def exit_with_message(message : String, code = 1) : Nil
  puts message
  press_any_key
  exit(code)
end

def prompt_choice(prompt : String, choices : Array(String), default : String? = nil) : String
  choices_lower = choices.map(&.downcase)
  loop do
    if default
      print "#{prompt} (#{choices.join('/')}) [default: #{default}]: "
      value = gets.try(&.strip) || ""
      return default if value.empty?
    else
      print "#{prompt} (#{choices.join('/')}): "
      value = gets.try(&.strip) || ""
    end

    value_lower = value.downcase
    if idx = choices_lower.index(value_lower)
      return choices[idx]
    end

    puts "Invalid choice. Try again."
  end
end

def prompt_nonempty(prompt : String, default : String? = nil) : String
  loop do
    if default
      print "#{prompt} [default: #{default}]: "
      value = gets.try(&.strip) || ""
      return default if value.empty?
    else
      print prompt
      value = gets.try(&.strip) || ""
    end

    return value unless value.empty?

    puts "Value cannot be empty."
  end
end

def ensure_tool_exists(tool_name : String) : Bool
  !Process.find_executable(tool_name).nil?
end

def run_command(cmd : Array(String)) : Int32
  puts "\nRunning:"
  puts cmd.map { |x| x.includes?(' ') ? %("#{x}") : x }.join(' ')
  puts

  status = Process.run(
    command: cmd.first,
    args: cmd[1..]?,
    output: Process::Redirect::Inherit,
    error: Process::Redirect::Inherit,
  )
  status.try(&.exit_code) || 127
rescue File::NotFoundError
  puts "Error: #{cmd.first} was not found."
  127
end

def main
  puts "Quark Downloader"
  puts "----------------"

  unless ensure_tool_exists("yt-dlp")
    exit_with_message <<-MSG
      yt-dlp not found.

      It is either not installed or not in PATH.
      Try installing it with:
        winget install yt-dlp.yt-dlp

      Or with Python:
        py -m pip install -U yt-dlp
      MSG
  end

  url = prompt_nonempty("Enter YouTube URL: ")

  media_type = prompt_choice(
    "Download audio or video?",
    ["audio", "video"],
    default: "video",
  ).downcase

  default_path = File.expand_path(default_downloads_dir)

  output_dir = prompt_nonempty(
    "Enter output directory",
    default: default_path,
  )

  output_path = Path[File.expand_path(output_dir)]

  begin
    Dir.mkdir_p(output_path.to_s)
  rescue ex
    exit_with_message("Error creating output directory:\n#{ex}")
  end

  outtmpl = (output_path / "%(title)s [%(id)s].%(ext)s").to_s

  cmd = ["yt-dlp", "--no-playlist", "-o", outtmpl]

  if media_type == "audio"
    puts "\nAudio formats:"
    puts "  original / default.original  -> no re-encode"
    puts "  mp3, m4a, flac, wav, opus, vorbis"

    print "Choose audio format [default: original]: "
    audio_format = (gets.try(&.strip) || "").downcase
    audio_format = "original" if audio_format.empty?

    cmd.concat(["-f", "bestaudio/best"])

    unless {"original", "default.original"}.includes?(audio_format)
      unless ensure_tool_exists("ffmpeg")
        exit_with_message <<-MSG
          ffmpeg not found.

          ffmpeg is required for audio conversion.
          Install it or choose original format instead.
          MSG
      end

      cmd.concat(["-x", "--audio-format", audio_format])
    end
  else
    puts "\nVideo formats:"
    puts "  original / default.original  -> no re-encode"
    puts "  mp4, mkv, webm"

    print "Choose video format [default: original]: "
    video_format = (gets.try(&.strip) || "").downcase
    video_format = "original" if video_format.empty?

    if {"original", "default.original"}.includes?(video_format)
      cmd.concat([
        "-f", "bv*+ba/b",
        "--merge-output-format", "mkv",
      ])
    else
      unless ensure_tool_exists("ffmpeg")
        exit_with_message <<-MSG
          ffmpeg not found.

          ffmpeg is required for video merging/conversion.
          Install it or choose original format instead.
          MSG
      end

      cmd.concat([
        "-f", "bv*+ba/b",
        "--merge-output-format", video_format,
      ])
    end
  end

  cmd << url

  exit_code = run_command(cmd)

  if exit_code == 0
    puts "Done."
    press_any_key
  else
    exit_with_message("Failed with exit code #{exit_code}.", exit_code)
  end
end

{% unless flag?(:windows) %}
Signal::INT.trap do
  puts "\nCancelled."
  press_any_key
  exit(130)
end
{% end %}

main
