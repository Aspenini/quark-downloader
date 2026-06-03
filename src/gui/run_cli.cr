module QuarkGui
  struct DownloadParams
    property url : String
    property media_type : String
    property format : String
    property output_dir : String

    def initialize(@url, @media_type, @format, @output_dir)
    end
  end

  def self.build_cli_args(cli : String, params : DownloadParams) : Array(String)
    [
      cli,
      "--url", params.url,
      "--type", params.media_type,
      "--format", params.format,
      "--output-dir", params.output_dir,
      "--no-pause",
    ]
  end

  def self.default_output_dir(cli : String) : String
    {% if flag?(:windows) %}
      # Avoid spawning the console CLI during GUI startup (visible flash).
      if dir = config_download_dir?
        return expand_path(dir)
      end
      return File.join(ENV["USERPROFILE"]? || ".", "Downloads")
    {% else %}
      output = IO::Memory.new
      status = Process.run(
        cli,
        args: ["--print-default-output-dir"],
        output: output,
        error: Process::Redirect::Close,
      )
      if status.try(&.success?) && !output.to_s.strip.empty?
        return output.to_s.strip
      end

      home = ENV["HOME"]? || "."
      File.join(home, "Downloads")
    {% end %}
  end

  def self.config_download_dir? : String?
    config = Path[ENV["APPDATA"]? || ENV["HOME"]? || "."] / "quark-downloader" / "quark-downloader.conf"
    return nil unless File.exists?(config.to_s)

    File.each_line(config.to_s) do |line|
      line = line.strip
      next if line.empty? || line.starts_with?('#')
      next unless line.includes?('=')

      key, value = line.split('=', limit: 2).map(&.strip)
      next unless key.downcase == "download_dir"
      return value if !value.empty?
    end

    nil
  end

  def self.expand_path(path : String) : String
    home = ENV["USERPROFILE"]? || ENV["HOME"]? || "."
    expanded = if path.starts_with?("~/")
                 File.join(home, path[2..])
               elsif path == "~"
                 home
               else
                 path
               end
    File.expand_path(expanded)
  end

  def self.run_download(cli : String, params : DownloadParams) : Int32
    args = build_cli_args(cli, params)
    command = args.first
    cmd_args = args[1..]? || [] of String

    {% if flag?(:windows) %}
      status = Process.run(
        "cmd",
        args: ["/c", "start", "/wait", "Quark Downloader", command] + cmd_args,
        shell: false,
      )
      status.try(&.exit_code) || 1
    {% else %}
      if terminal = find_terminal
        run_in_terminal(terminal, command, cmd_args)
      else
        status = Process.run(
          command: command,
          args: cmd_args,
          output: Process::Redirect::Inherit,
          error: Process::Redirect::Inherit,
        )
        status.try(&.exit_code) || 1
      end
    {% end %}
  end

  {% unless flag?(:windows) %}
  TERMINAL_CANDIDATES = [
    {"x-terminal-emulator", {"-e"}},
    {"gnome-terminal", {"--wait", "--"}},
    {"konsole", {"-e"}},
    {"xfce4-terminal", {"-e"}},
    {"alacritty", {"-e"}},
    {"foot", {"-e"}},
  ]

  def self.find_terminal : Tuple(String, Array(String))?
    TERMINAL_CANDIDATES.each do |name, prefix|
      if path = Process.find_executable(name)
        return {path, prefix}
      end
    end
    nil
  end

  def self.shell_escape(s : String) : String
    "'" + s.gsub("'", "'\\''") + "'"
  end

  def self.run_in_terminal(terminal : Tuple(String, Array(String)), command : String, args : Array(String)) : Int32
    path, prefix = terminal
    inner = ([command] + args).map { |a| shell_escape(a) }.join(" ")
    inner += "; echo; read -r -p 'Press Enter to close...' _"

    status = Process.run(path, args: prefix + ["sh", "-c", inner])

    status.try(&.exit_code) || 1
  end
  {% end %}

end
