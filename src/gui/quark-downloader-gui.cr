require "./cli_path"
require "./run_cli"

{% if flag?(:windows) %}
require "./win32_ui"
{% else %}
require "./tk_ui"
{% end %}

module QuarkGui
  APP_TITLE = "Quark Downloader"

  def self.run
    cli = resolve_cli
    unless cli
      {% if flag?(:windows) %}
      Win32Ui.message_box("quark-downloader was not found.\nInstall it next to this program or on PATH.", true)
      {% else %}
      TkUi.tk_error("quark-downloader was not found.\nInstall it next to this program or on PATH.")
      {% end %}
      return
    end

    params = {% if flag?(:windows) %} Win32Ui.collect_params(cli) {% else %} TkUi.collect_params(cli) {% end %}
    return unless params

    exit_code = run_download(cli, params)

    {% if flag?(:windows) %}
    Win32Ui.show_result(exit_code == 0, exit_code)
    {% else %}
    TkUi.show_result(exit_code == 0, exit_code)
    {% end %}
  end
end

QuarkGui.run
