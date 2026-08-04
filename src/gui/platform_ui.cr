require "../download_result"
require "./types"

{% if flag?(:windows) %}
  require "./win32_ui"
{% elsif flag?(:darwin) %}
  require "./macos_ui"
{% else %}
  require "./tk_ui"
{% end %}

module QuarkGui
  module PlatformUi
    def self.show_missing_cli : Nil
      show_error("quark-downloader was not found.\nInstall it next to this program or on PATH.")
    end

    def self.show_error(message : String) : Nil
      {% if flag?(:windows) %}
        Win32Ui.message_box(message, true)
      {% elsif flag?(:darwin) %}
        MacUi.show_error(message)
      {% else %}
        TkUi.show_error(message)
      {% end %}
    end

    def self.show_completion(result : DownloadResult) : Nil
      {% if flag?(:windows) %}
        # Windows shows completion from the progress dialog itself.
      {% elsif flag?(:darwin) %}
        MacUi.show_completion(result)
      {% else %}
        TkUi.show_completion(result)
      {% end %}
    end

    def self.open_folder(path : String) : Nil
      return if path.strip.empty?

      {% if flag?(:windows) %}
        Process.run("explorer", args: [path], error: Process::Redirect::Close)
      {% elsif flag?(:darwin) %}
        Process.run("open", args: [path], error: Process::Redirect::Close)
      {% else %}
        if xdg = Process.find_executable("xdg-open")
          Process.run(xdg, args: [path], error: Process::Redirect::Close)
        end
      {% end %}
    rescue
    end

    def self.collect_main_session(default_output : String, settings : QuarkConfig::Settings) : MainSessionResult
      {% if flag?(:windows) %}
        Win32Ui.collect_main_session(default_output, settings)
      {% elsif flag?(:darwin) %}
        MacUi.collect_main_session(default_output, settings)
      {% else %}
        TkUi.collect_main_session(default_output, settings)
      {% end %}
    end
  end
end
