{% if flag?(:windows) %}
  require "./win32/api"
  require "./win32/resources"
  require "./win32/message_box"
  require "./win32/controls"
  require "./win32/folder_browser"
  require "./win32/main_dialog"
  require "./win32/settings_dialog"
{% end %}
