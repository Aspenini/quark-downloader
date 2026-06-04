{% if flag?(:windows) %}
  require "./api"

  module QuarkGui
    module Win32Ui
      def self.resource_error_message(err : UInt32, stage : String) : String
        exe = Process.executable_path || "quark-downloader-gui.exe"
        <<-MSG
        Could not open the download dialog (#{stage}, Windows error #{err}).

        From: #{exe}

        Fix:
          1. just clean
          2. just build
          3. Run build\\quark-downloader-gui.exe (not an old copy)

        Rebuild with: just clean && just build
        Do not UPX quark-downloader-gui.exe.
        MSG
      end

      def self.load_dialog_template(dialog_id : Int32) : Void*?
        hmodule = module_handle
        name = int_resource(dialog_id)
        type = int_resource(RT_DIALOG)
        lang = ((LANG_NEUTRAL.to_u32 << 10) | SUBLANG_NEUTRAL.to_u32).to_u16

        res_info = LibUser32.FindResourceExW(hmodule, type, name, lang)
        res_info = LibUser32.FindResourceW(hmodule, name, type) if res_info.null?
        return nil if res_info.null?

        res_data = LibUser32.LoadResource(hmodule, res_info)
        return nil if res_data.null?

        template = LibUser32.LockResource(res_data)
        return nil if template.null?

        template
      end
    end
  end
{% end %}
