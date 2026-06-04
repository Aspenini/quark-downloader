{% if flag?(:windows) %}
  require "./api"
  require "./controls"

  module QuarkGui
    module Win32Ui
      @@browse_initial = ""
      @@browse_callback : BrowseCallback?

      def self.browse_callback_proc : BrowseCallback
        @@browse_callback ||= BrowseCallback.new do |hwnd, msg, _lparam, _lp_data|
          if msg == BFFM_INITIALIZED && !@@browse_initial.empty?
            path = wide(@@browse_initial)
            LibUser32.SendMessageW(
              hwnd.as(WinHWND),
              BFFM_SETSELECTIONW,
              1,
              path.to_unsafe.address.to_i64,
            )
          end
          0
        end
      end

      def self.browse_folder_for(hdlg : WinHWND, edit_id : Int32, fallback : String, title_text : String) : String?
        current = get_dlg_text(hdlg, edit_id).strip
        @@browse_initial = current.empty? ? fallback : current

        title = wide(title_text)
        display = Pointer(UInt16).malloc(260)
        bi = BROWSEINFOW.new(
          hdlg.as(Void*),
          Pointer(Void).null,
          display,
          title.to_unsafe,
          (BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE).to_u32,
          browse_callback_proc,
          0_i64,
          0,
        )

        pidl = LibShell32.SHBrowseForFolderW(pointerof(bi).as(Void*))
        if pidl.null?
          return nil
        end

        path_buf = Array(UInt16).new(32_768, 0_u16)
        if LibShell32.SHGetPathFromIDListW(pidl, path_buf.to_unsafe) == 0
          return nil
        end

        str, _ = String.from_utf16(path_buf.to_unsafe)
        str
      end
    end
  end
{% end %}
