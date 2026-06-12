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

    def self.show_completion(success : Bool, exit_code : Int32 = 0) : Nil
      {% if flag?(:windows) %}
        # Windows shows completion from the progress dialog itself.
      {% elsif flag?(:darwin) %}
        MacUi.show_completion(success, exit_code)
      {% else %}
        TkUi.show_completion(success, exit_code)
      {% end %}
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
