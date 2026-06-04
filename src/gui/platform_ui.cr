require "./types"

{% if flag?(:windows) %}
  require "./win32_ui"
{% else %}
  require "./tk_ui"
{% end %}

module QuarkGui
  module PlatformUi
    def self.show_missing_cli : Nil
      {% if flag?(:windows) %}
        Win32Ui.message_box("quark-downloader was not found.\nInstall it next to this program or on PATH.", true)
      {% else %}
        TkUi.tk_error("quark-downloader was not found.\nInstall it next to this program or on PATH.")
      {% end %}
    end

    def self.show_error(message : String) : Nil
      {% if flag?(:windows) %}
        Win32Ui.message_box(message, true)
      {% else %}
        TkUi.show_error(message)
      {% end %}
    end

    def self.collect_main_action(default_output : String) : MainAction::Type
      {% if flag?(:windows) %}
        Win32Ui.collect_main_action(default_output)
      {% else %}
        TkUi.collect_main_action(default_output)
      {% end %}
    end

    def self.collect_main_session(default_output : String, settings : QuarkConfig::Settings) : MainSessionResult
      {% if flag?(:windows) %}
        Win32Ui.collect_main_session(default_output, settings)
      {% else %}
        TkUi.collect_main_session(default_output, settings)
      {% end %}
    end

    def self.collect_settings_action(settings : QuarkConfig::Settings) : SettingsAction::Type
      {% if flag?(:windows) %}
        Win32Ui.collect_settings_action(settings)
      {% else %}
        TkUi.collect_settings_action(settings)
      {% end %}
    end
  end
end
