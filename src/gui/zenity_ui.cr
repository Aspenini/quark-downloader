{% unless flag?(:windows) %}
module QuarkGui
  AUDIO_FORMATS = ["original", "mp3", "m4a", "flac", "wav", "opus", "vorbis"]
  VIDEO_FORMATS = ["original", "mp4", "mkv", "webm"]

  def self.zenity_path : String?
    Process.find_executable("zenity")
  end

  def self.zenity_error(message : String) : Nil
    if z = zenity_path
      Process.run(z, args: ["--error", "--text=#{message}", "--title=Quark Downloader"])
    else
      STDERR.puts message
    end
    exit 1
  end

  def self.zenity_run(args : Array(String)) : String?
    z = zenity_path
    return nil unless z

    output = IO::Memory.new
    status = Process.run(z, args: args, output: output, error: Process::Redirect::Pipe)
    return nil unless status.try(&.success?)

    text = output.to_s.strip
    text.empty? ? nil : text
  end

  def self.show_result(success : Bool, exit_code : Int32) : Nil
    z = zenity_path
    return unless z

    if success
      Process.run(z, args: ["--info", "--title=Quark Downloader", "--text=Download finished successfully."])
    else
      text = "Download failed (exit code #{exit_code}).\nCheck the terminal window for details."
      Process.run(z, args: ["--error", "--title=Quark Downloader", "--text=#{text}"])
    end
  end

  def self.collect_params(cli : String) : DownloadParams?
    unless zenity_path
      zenity_error("zenity is required for the GUI.\nInstall zenity (e.g. apt install zenity, brew install zenity).")
    end

    url = zenity_run([
      "--entry", "--title=Quark Downloader",
      "--text=Enter video URL:",
    ])
    return nil unless url

    media_type = zenity_run([
      "--list", "--title=Quark Downloader",
      "--text=Download audio or video?",
      "--column=Type", "--column=Description",
      "video", "Video",
      "audio", "Audio",
      "--height=220", "--width=320",
    ]) || "video"

    formats = media_type == "audio" ? AUDIO_FORMATS : VIDEO_FORMATS
    format_args = ["--list", "--title=Quark Downloader", "--text=Choose format:", "--column=Format"]
    formats.each { |f| format_args << f }
    format_args.concat(["--height=280", "--width=280"])

    format = zenity_run(format_args) || "original"

    default_dir = default_output_dir(cli)
    output_dir = zenity_run([
      "--file-selection", "--directory",
      "--title=Quark Downloader",
      "--filename=#{default_dir}/",
    ])
    return nil unless output_dir

    DownloadParams.new(url, media_type, format, output_dir)
  end
end
{% end %}
