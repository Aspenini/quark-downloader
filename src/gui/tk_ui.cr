{% unless flag?(:windows) %}
require "../config"

module QuarkGui
  module TkUi
    TCL_SCRIPT = "quark-downloader-gui.tcl"

    {% if flag?(:darwin) %}
    DARWIN_WISH_CANDIDATES = [
      "/opt/homebrew/opt/tcl-tk/bin/wish",
      "/opt/homebrew/opt/tcl-tk/bin/wish9.0",
      "/usr/local/opt/tcl-tk/bin/wish",
      "/usr/local/opt/tcl-tk/bin/wish9.0",
    ]
    {% end %}

    def self.wish_path : String?
      {% if flag?(:darwin) %}
      # macOS ships a broken Tk 8.5 wish at /usr/bin/wish; Homebrew tcl-tk must win.
      if brew = Process.find_executable("brew")
        output = IO::Memory.new
        if Process.run(brew, args: ["--prefix", "tcl-tk"], output: output, error: Process::Redirect::Close).try(&.success?)
          prefix = output.to_s.strip
          unless prefix.empty?
            ["#{prefix}/bin/wish", "#{prefix}/bin/wish9.0"].each do |candidate|
              return candidate if File::Info.executable?(candidate)
            end
          end
        end
      end

      DARWIN_WISH_CANDIDATES.each do |candidate|
        return candidate if File::Info.executable?(candidate)
      end

      path = Process.find_executable("wish")
      return nil if path == "/usr/bin/wish"

      path
      {% else %}
      Process.find_executable("wish")
      {% end %}
    end

    def self.tcl_script_path : Path
      beside_exe = QuarkConfig.app_dir / TCL_SCRIPT
      return beside_exe if File.exists?(beside_exe.to_s)

      Path[__DIR__] / TCL_SCRIPT
    end

    def self.tk_error(message : String) : Nil
      if wish = wish_path
        script = tcl_script_path
        if File.exists?(script.to_s)
          Process.run(
            wish,
            args: [script.to_s, "--message", "error", APP_TITLE, message],
            error: Process::Redirect::Close,
          )
        else
          STDERR.puts message
        end
      else
        STDERR.puts message
      end
      exit 1
    end

    def self.ensure_wish! : String
      unless wish = wish_path
        tk_error(<<-MSG)
          Tk is required for the GUI (wish not found).
            Debian/Ubuntu: sudo apt install tk
            macOS: brew install tcl-tk
          MSG
      end
      wish
    end

    def self.ensure_script! : String
      script = tcl_script_path.to_s
      unless File.exists?(script)
        tk_error("GUI script not found: #{script}")
      end
      script
    end

    def self.run_wish(args : Array(String)) : {Int32, String}
      wish = ensure_wish!
      script = ensure_script!

      output = IO::Memory.new
      status = Process.run(
        wish,
        args: [script] + args,
        output: output,
        error: Process::Redirect::Pipe,
      )
      code = status.try(&.exit_code) || 1
      unless code == 0 || code == 1
        tk_error("Tk GUI failed to start (exit code #{code}).\nOn macOS: brew install tcl-tk")
      end
      {code, output.to_s}
    end

    def self.show_result(success : Bool, exit_code : Int32) : Nil
      kind = success ? "ok" : "error"
      body = if success
               "Download finished successfully."
             else
               "Download failed (exit code #{exit_code}).\nCheck the terminal window for details."
             end
      run_wish(["--message", kind, APP_TITLE, body])
    end

    def self.collect_params(cli : String) : DownloadParams?
      default_dir = QuarkGui.default_output_dir(cli)
      code, text = run_wish([default_dir])

      return nil unless code == 0

      lines = text.lines.map(&.strip).reject(&.empty?)
      return nil unless lines.size >= 4

      url, media_type, format, output_dir = lines[0..3]
      return nil if url.empty? || output_dir.empty?

      DownloadParams.new(url, media_type, format, output_dir)
    end
  end
end
{% end %}
