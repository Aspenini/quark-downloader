require "../release_check"
require "../version"
require "./types"

{% if flag?(:windows) %}
  require "./win32/message_box"
{% else %}
  require "./tk_ui"
{% end %}

module QuarkGui
  module UpdateCheck
    def self.run! : Nil
      status, latest, behind, error = ReleaseCheck.check_with_error

      case status
      when ReleaseCheck::Status::UpToDate
        show_info("You are up to date (#{QuarkVersion::VERSION}).")
      when ReleaseCheck::Status::Ahead
        show_info(
          "You are running #{QuarkVersion::VERSION} (newer than the latest release #{latest}).",
        )
      when ReleaseCheck::Status::Behind
        present_behind(behind.not_nil!)
      when ReleaseCheck::Status::Failed
        show_error("Could not check for updates:\n#{error || "unknown error"}")
      end
    end

    def self.present_behind(info : ReleaseCheck::BehindInfo) : Nil
      {% if flag?(:windows) %}
        message = "A newer version (#{info.latest_version}) is available. " \
                  "You have #{QuarkVersion::VERSION}.\n\n" \
                  "Download the latest installer?"
        Win32Ui.confirm_open_url(message, info.download_url)
      {% else %}
        show_info(
          "A newer version (#{info.latest_version}) is available. " \
          "You have #{QuarkVersion::VERSION}.\n\n" \
          "Update with your package manager (e.g. brew upgrade quark-downloader, yay -Syu, or the AUR).",
        )
      {% end %}
    end

    def self.show_info(message : String) : Nil
      {% if flag?(:windows) %}
        Win32Ui.message_box(message, error: false)
      {% else %}
        TkUi.show_message("ok", APP_TITLE, message)
      {% end %}
    end

    def self.show_error(message : String) : Nil
      {% if flag?(:windows) %}
        Win32Ui.message_box(message, error: true)
      {% else %}
        TkUi.show_message("error", APP_TITLE, message)
      {% end %}
    end
  end
end
