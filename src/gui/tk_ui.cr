{% unless flag?(:windows) %}
require "../config"

module QuarkGui
  module TkUi
    TCL_SCRIPT = "quark-downloader-gui.tcl"

    def self.wish_path : String?
      Process.find_executable("wish")
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
