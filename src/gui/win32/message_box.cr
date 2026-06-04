{% if flag?(:windows) %}
  require "./api"

  module QuarkGui
    module Win32Ui
      def self.message_box(text : String, error : Bool = false)
        flags = error ? (MB_OK | MB_ICONERROR) : (MB_OK | MB_ICONINFORMATION)
        LibUser32.MessageBoxW(
          Pointer(Void).null,
          wide(text).to_unsafe,
          wide(APP_TITLE).to_unsafe,
          flags,
        )
      end

      def self.confirm_open_url(message : String, url : String) : Bool
        result = LibUser32.MessageBoxW(
          Pointer(Void).null,
          wide(message).to_unsafe,
          wide(APP_TITLE).to_unsafe,
          MB_YESNO | MB_ICONINFORMATION,
        )

        return false unless result == IDYES

        open_url(url)
        true
      end

      def self.open_url(url : String) : Nil
        LibShell32.ShellExecuteW(
          Pointer(Void).null,
          wide("open").to_unsafe,
          wide(url).to_unsafe,
          Pointer(UInt16).null,
          Pointer(UInt16).null,
          SW_SHOW,
        )
      end
    end
  end
{% end %}
