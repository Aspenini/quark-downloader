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
    end
  end
{% end %}
