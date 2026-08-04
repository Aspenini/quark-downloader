require "option_parser"
require "./config"
require "./download"
require "./term_color"
require "./version"

def prompt_choice(prompt : String, choices : Array(String), default : String? = nil) : String
  choices_lower = choices.map(&.downcase)
  loop do
    if default
      print "#{TermColor.bold(prompt)} (#{choices.join('/')}) [#{TermColor.dim("default: #{default}")}]: "
      value = gets.try(&.strip) || ""
      return default if value.empty?
    else
      print "#{TermColor.bold(prompt)} (#{choices.join('/')}): "
      value = gets.try(&.strip) || ""
    end

    value_lower = value.downcase
    if idx = choices_lower.index(value_lower)
      return choices[idx]
    end

    puts TermColor.red("Invalid choice. Try again.")
  end
end

def prompt_nonempty(prompt : String, default : String? = nil) : String
  loop do
    if default
      print "#{TermColor.bold(prompt)} [#{TermColor.dim("default: #{default}")}]: "
      value = gets.try(&.strip) || ""
      return default if value.empty?
    else
      print "#{TermColor.bold(prompt)}: "
      value = gets.try(&.strip) || ""
    end

    return value unless value.empty?

    puts TermColor.red("Value cannot be empty.")
  end
end

def interactive_main
  QuarkConfig.load!

  puts TermColor.bold(QuarkVersion.window_title)
  puts TermColor.dim("─" * 40)
  puts

  url = prompt_nonempty("Video or playlist URL")

  puts
  media_type = prompt_choice(
    "Download audio or video?",
    ["audio", "video"],
    default: "video",
  ).downcase

  puts
  default_path = QuarkConfig.download_dir(QuarkDownload.default_downloads_dir)
  output_dir = prompt_nonempty(
    "Output directory",
    default: default_path,
  )

  puts
  format = if media_type == "audio"
             puts TermColor.bold("Audio formats")
             puts TermColor.dim("  original  (keep source audio)")
             puts TermColor.dim("  mp3, m4a, flac, wav, opus, vorbis")
             print "Choose format [#{TermColor.dim("default: original")}]: "
             (gets.try(&.strip) || "").downcase
           else
             puts TermColor.bold("Video formats")
             puts TermColor.dim("  original  (keep source video)")
             puts TermColor.dim("  mp4, mkv, webm")
             print "Choose format [#{TermColor.dim("default: original")}]: "
             (gets.try(&.strip) || "").downcase
           end

  format = "original" if format.empty?
  puts

  exit QuarkDownload.run(url, media_type, format, output_dir)
end

{% unless flag?(:windows) %}
  Signal::INT.trap do
    puts "\n#{TermColor.yellow("Cancelled.")}"
    QuarkDownload.press_any_key(false)
    exit(130)
  end
{% end %}

urls = [] of String
media_type = "video"
format = "original"
output_dir = nil
no_pause = false
print_default_dir = false
emit_result = false

OptionParser.parse do |parser|
  parser.banner = "Usage: quark-downloader [options]\n\nInteractive when run with no options."

  parser.on("--url URL", "Video or playlist URL to download (repeatable)") { |v| urls << v }
  parser.on("--batch-file FILE", "File with one URL per line (# comments ignored)") { |v|
    unless File.exists?(v)
      abort TermColor.red("Batch file not found: #{v}")
    end
    File.each_line(v) do |line|
      line = line.strip
      next if line.empty? || line.starts_with?('#')
      urls << line
    end
  }
  parser.on("--type TYPE", "audio or video (default: video)") { |v| media_type = v }
  parser.on("--format FORMAT", "Output format (default: original)") { |v| format = v }
  parser.on("--output-dir DIR", "Output directory") { |v| output_dir = v }
  parser.on("--no-pause", "Do not wait for a key press before exiting (Windows)") { no_pause = true }
  parser.on("--print-default-output-dir", "Print default output directory and exit") { print_default_dir = true }
  parser.on("--emit-result-json", "Print a final __RESULT__ JSON line for GUI/tools") { emit_result = true }
  parser.on("-h", "--help", "Show help") {
    puts parser
    exit
  }
end

if print_default_dir
  puts QuarkDownload.default_output_dir
  exit 0
end

if urls.empty?
  interactive_main
else
  exit QuarkDownload.run_all(urls, media_type, format, output_dir, no_pause: no_pause, emit_result: emit_result)
end
