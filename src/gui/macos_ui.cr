{% if flag?(:darwin) %}
  require "../config"
  require "../logs"
  require "../process_status"
  require "./session_protocol"
  require "./tk_ui"
  require "./types"

  # Native AppKit UI via the Swift helper binary. Falls back to the Tk UI
  # when the helper is missing (e.g. `crystal run` without a Swift build).
  module QuarkGui
    module MacUi
      HELPER_NAME = "quark-downloader-gui-helper"

      def self.helper_path : String?
        beside = (QuarkConfig.app_dir / HELPER_NAME).to_s
        return beside if File::Info.executable?(beside)

        dev = (Path[Dir.current] / "build" / HELPER_NAME).to_s
        return dev if File::Info.executable?(dev)

        nil
      end

      def self.available? : Bool
        !helper_path.nil?
      end

      def self.run_helper(args : Array(String)) : {Int32, String}
        helper = helper_path
        raise "macOS GUI helper not found" unless helper

        output = IO::Memory.new
        status = Process.run(
          helper,
          args: args,
          output: output,
          error: Process::Redirect::Close,
        )
        {QuarkProcess.exit_code(status), output.to_s}
      end

      def self.show_message(kind : String, title : String, body : String) : Nil
        unless available?
          return TkUi.show_message(kind, title, body)
        end

        run_helper(["--message", kind, title, body])
      end

      def self.show_error(message : String) : Nil
        show_message("error", APP_TITLE, message)
      end

      def self.show_completion(success : Bool, _exit_code : Int32 = 0) : Nil
        if success
          show_message("ok", APP_TITLE, "Download Complete!")
        else
          body = "Download failed."
          body += "\n\nLogs: #{QuarkLogs.logs_dir}" if QuarkConfig.download_logs?
          show_message("error", APP_TITLE, body)
        end
      end

      def self.collect_main_session(default_dir : String, settings : QuarkConfig::Settings) : MainSessionResult
        unless available?
          return TkUi.collect_main_session(default_dir, settings)
        end

        code, text = run_helper(SessionProtocol.build_session_args(default_dir, settings))
        return MainSessionResult.new(MainAction::Cancel.new) unless code == 0

        SessionProtocol.parse(text)
      end
    end
  end
{% end %}
