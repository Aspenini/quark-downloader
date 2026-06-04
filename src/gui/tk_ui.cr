{% unless flag?(:windows) %}
require "../config"
require "../logs"
require "./types"

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

    def self.show_completion(success : Bool, _exit_code : Int32 = 0) : Nil
      if success
        run_wish(["--message", "ok", APP_TITLE, "Download Complete!"])
      else
        body = "Download failed."
        body += "\n\nLogs: #{QuarkLogs.logs_dir}" if QuarkConfig.download_logs?
        run_wish(["--message", "error", APP_TITLE, body])
      end
    end

    def self.show_error(message : String) : Nil
      run_wish(["--message", "error", APP_TITLE, message])
    end

    def self.collect_main_action(default_dir : String) : MainAction::Type
      code, text = run_wish([default_dir])
      return MainAction::Cancel.new unless code == 0

      lines = text.lines.map(&.strip).reject(&.empty?)
      return MainAction::Cancel.new if lines.empty?

      return MainAction::Settings.new if lines.first == "__OPEN_SETTINGS__"
      return MainAction::Cancel.new unless lines.size >= 4

      url, media_type, format, output_dir = lines[0..3]
      return MainAction::Cancel.new if url.empty? || output_dir.empty?

      MainAction::Download.new(DownloadParams.new(url, media_type, format, output_dir))
    end

    def self.collect_settings_action(settings : QuarkConfig::Settings) : SettingsAction::Type
      code, text = run_wish([
        "--settings",
        settings.download_dir,
        settings.yt_dlp.to_config,
        settings.ffmpeg.to_config,
        settings.gui_download_mode.to_config,
        settings.download_logs.to_s,
      ])
      return SettingsAction::Cancel.new unless code == 0

      lines = text.lines.map(&.strip)
      return SettingsAction::Cancel.new unless lines.size >= 6 && lines[0] == "__SETTINGS__"

      SettingsAction::Save.new(SettingsForm.from_strings(
        lines[1],
        lines[2],
        lines[3],
        lines[4],
        lines[5],
      ))
    end
  end
end
{% end %}
