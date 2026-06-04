{% if flag?(:windows) %}
module QuarkGui
  module Win32Ui
    IDD_MAIN     = 101
    IDC_URL      = 1001
    IDC_AUDIO    = 1002
    IDC_VIDEO    = 1003
    IDC_FORMAT   = 1004
    IDC_OUTPUT   = 1005
    IDC_BROWSE   = 1006

    AUDIO_FORMATS = ["original", "mp3", "m4a", "flac", "wav", "opus", "vorbis"]
    VIDEO_FORMATS = ["original", "mp4", "mkv", "webm"]

    WM_INITDIALOG = 0x0110
    WM_COMMAND    = 0x0111
    WM_KEYDOWN    = 0x0100
    BN_CLICKED    = 0

    VK_RETURN = 0x0D_u64
    VK_ESCAPE = 0x1B_u64

    BFFM_INITIALIZED     = 1_u32
    BFFM_SETSELECTIONW   = 0x0467_u32 # WM_USER + 103

    CB_ADDSTRING  = 0x0143
    CB_SETCURSEL  = 0x014E
    CB_RESETCONTENT = 0x014B

    BIF_RETURNONLYFSDIRS = 0x0001
    BIF_NEWDIALOGSTYLE   = 0x0040

    MB_OK              = 0x00000000
    MB_OKCANCEL        = 0x00000001
    MB_ICONERROR       = 0x00000010
    MB_ICONINFORMATION = 0x00000040

    alias WinHWND = Void*
    alias WinBOOL = Int32

    @@dialog_hwnd : WinHWND? = nil
    @@default_output : String = ""
    @@browse_initial : String = ""
    @@confirmed = false
    @@url = ""
    @@media_type = "video"
    @@format = "original"
    @@output_dir = ""

    alias DialogCallback = Proc(Void*, UInt32, UInt64, UInt64, Int32)
    @@dialog_proc : DialogCallback?

    RT_DIALOG = 5

    LANG_NEUTRAL      = 0x00_u16
    SUBLANG_NEUTRAL   = 0x00_u16

    lib LibKernel32
      fun GetModuleHandleW(lpModuleName : Void*) : Void*
    end

    lib LibUser32
      fun GetLastError : UInt32
      fun FindResourceW(hModule : Void*, lpName : Void*, lpType : Void*) : Void*
      fun FindResourceExW(hModule : Void*, lpType : Void*, lpName : Void*, wLanguage : UInt16) : Void*
      fun LoadResource(hModule : Void*, hResInfo : Void*) : Void*
      fun LockResource(hResData : Void*) : Void*
      fun DialogBoxIndirectParamW(
        hInstance : Void*,
        hDialogTemplate : Void*,
        hWndParent : WinHWND,
        lpDialogFunc : DialogCallback,
        dwInitParam : Int64,
      ) : Int64
      fun EndDialog(hDlg : WinHWND, nResult : Int64) : WinBOOL
      fun GetDlgItem(hDlg : WinHWND, nIDDlgItem : Int32) : WinHWND
      fun SendMessageW(hWnd : WinHWND, msg : UInt32, wParam : UInt64, lParam : Int64) : Int64
      fun SetDlgItemTextW(hDlg : WinHWND, nIDDlgItem : Int32, lpString : UInt16*) : WinBOOL
      fun GetDlgItemTextW(hDlg : WinHWND, nIDDlgItem : Int32, lpString : UInt16*, cchMax : Int32) : UInt32
      fun CheckDlgButton(hDlg : WinHWND, nIDButton : Int32, uCheck : UInt32) : WinBOOL
      fun IsDlgButtonChecked(hDlg : WinHWND, nIDButton : Int32) : UInt32
      fun MessageBoxW(hWnd : WinHWND, lpText : UInt16*, lpCaption : UInt16*, uType : UInt32) : Int32
    end

    lib LibShell32
      fun SHBrowseForFolderW(lpbi : Void*) : Void*
      fun SHGetPathFromIDListW(pidl : Void*, pszPath : UInt16*) : WinBOOL
    end

    lib LibComctl32
      ICC_WIN95_CLASSES = 0x000000FF

      struct INITCOMMONCONTROLSEX
        dwSize : UInt32
        dwICC : UInt32
      end

      fun InitCommonControlsEx(lpInitCtrls : INITCOMMONCONTROLSEX*) : WinBOOL
    end

    alias BrowseCallback = Proc(Void*, UInt32, Int64, Int64, Int32)
    @@browse_callback : BrowseCallback?

    struct BROWSEINFOW
      def initialize(
        @hwndOwner : Void*,
        @pidlRoot : Void*,
        @pszDisplayName : UInt16*,
        @lpszTitle : UInt16*,
        @ulFlags : UInt32,
        @lpfn : BrowseCallback,
        @lParam : Int64,
        @iImage : Int32,
      )
      end
    end

    def self.int_resource(id : Int32) : Void*
      Pointer(Void).new(id.to_u64)
    end

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

    def self.wide(s : String) : Slice(UInt16)
      s.to_utf16
    end

    def self.set_dlg_text(hdlg : WinHWND, id : Int32, text : String)
      buf = wide(text)
      LibUser32.SetDlgItemTextW(hdlg, id, buf.to_unsafe)
    end

    def self.get_dlg_text(hdlg : WinHWND, id : Int32) : String
      buf = Array(UInt16).new(32_768, 0_u16)
      len = LibUser32.GetDlgItemTextW(hdlg, id, buf.to_unsafe, buf.size)
      String.from_utf16(Slice.new(buf.to_unsafe, len))
    end

    def self.populate_formats(hdlg : WinHWND)
      combo = LibUser32.GetDlgItem(hdlg, IDC_FORMAT)
      LibUser32.SendMessageW(combo, CB_RESETCONTENT, 0, 0)
      formats = @@media_type == "audio" ? AUDIO_FORMATS : VIDEO_FORMATS
      formats.each do |f|
        w = wide(f)
        LibUser32.SendMessageW(combo, CB_ADDSTRING, 0_u64, w.to_unsafe.address.to_i64)
      end
      LibUser32.SendMessageW(combo, CB_SETCURSEL, 0, 0)
    end

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

    def self.browse_folder(hdlg : WinHWND) : String?
      current = get_dlg_text(hdlg, IDC_OUTPUT).strip
      @@browse_initial = current.empty? ? @@default_output : current

      title = wide("Select output folder")
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

    def self.try_confirm(hdlg : WinHWND) : WinBOOL
      url = get_dlg_text(hdlg, IDC_URL).strip
      if url.empty?
        message_box("Please enter a video URL.", true)
        return 0
      end

      output = get_dlg_text(hdlg, IDC_OUTPUT).strip
      if output.empty?
        message_box("Please choose an output folder.", true)
        return 0
      end

      combo = LibUser32.GetDlgItem(hdlg, IDC_FORMAT)
      sel = LibUser32.SendMessageW(combo, 0x0147_u32, 0, 0)
      format_buf = Array(UInt16).new(256, 0_u16)
      LibUser32.SendMessageW(combo, 0x0149_u32, sel, format_buf.to_unsafe.address.to_i64)
      format, _ = String.from_utf16(format_buf.to_unsafe)

      @@url = url
      @@output_dir = output
      @@format = format.empty? ? "original" : format
      @@media_type = LibUser32.IsDlgButtonChecked(hdlg, IDC_AUDIO) != 0 ? "audio" : "video"
      @@confirmed = true
      LibUser32.EndDialog(hdlg, 1)
      1
    end

    def self.handle_dialog(
      hdlg : WinHWND,
      msg : UInt32,
      wparam : UInt64,
      lparam : UInt64,
    ) : WinBOOL
      case msg
      when WM_INITDIALOG
        @@dialog_hwnd = hdlg
        LibUser32.CheckDlgButton(hdlg, IDC_VIDEO, 1)
        @@media_type = "video"
        populate_formats(hdlg)
        set_dlg_text(hdlg, IDC_OUTPUT, @@default_output)
        1
      when WM_COMMAND
        id = wparam & 0xFFFF
        notify = (wparam >> 16) & 0xFFFF

        case id
        when IDC_AUDIO
          if notify == BN_CLICKED
            @@media_type = "audio"
            populate_formats(hdlg)
          end
        when IDC_VIDEO
          if notify == BN_CLICKED
            @@media_type = "video"
            populate_formats(hdlg)
          end
        when IDC_BROWSE
          if notify == BN_CLICKED
            if folder = browse_folder(hdlg)
              set_dlg_text(hdlg, IDC_OUTPUT, folder)
            end
          end
        when 1 # IDOK
          if notify == BN_CLICKED
            return try_confirm(hdlg)
          end
        when 2 # IDCANCEL
          if notify == BN_CLICKED
            LibUser32.EndDialog(hdlg, 0)
            return 1
          end
        end
        0
      when WM_KEYDOWN
        case wparam
        when VK_RETURN
          return try_confirm(hdlg)
        when VK_ESCAPE
          LibUser32.EndDialog(hdlg, 0)
          return 1
        end
        0
      else
        0
      end
    end

    def self.message_box(text : String, error : Bool = false)
      flags = error ? (MB_OK | MB_ICONERROR) : (MB_OK | MB_ICONINFORMATION)
      LibUser32.MessageBoxW(
        Pointer(Void).null,
        wide(text).to_unsafe,
        wide(APP_TITLE).to_unsafe,
        flags,
      )
    end

    def self.show_result(success : Bool, exit_code : Int32)
      if success
        message_box("Download finished successfully.")
      else
        message_box("Download failed (exit code #{exit_code}).\nSee the console window for details.", true)
      end
    end

    def self.dialog_proc_callback : DialogCallback
      @@dialog_proc ||= DialogCallback.new do |hdlg, msg, wparam, lparam|
        handle_dialog(hdlg, msg, wparam, lparam)
      end
    end

    def self.ensure_common_controls!
      icc = LibComctl32::INITCOMMONCONTROLSEX.new(dwSize: 8_u32, dwICC: LibComctl32::ICC_WIN95_CLASSES)
      LibComctl32.InitCommonControlsEx(pointerof(icc))
    end

    def self.module_handle : Void*
      LibKernel32.GetModuleHandleW(Pointer(UInt16).null)
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

    def self.collect_params(cli : String) : DownloadParams?
      @@confirmed = false
      @@default_output = QuarkGui.default_output_dir(cli)

      ensure_common_controls!

      template = load_dialog_template(IDD_MAIN)
      unless template
        err = LibUser32.GetLastError()
        message_box(resource_error_message(err, "template not found"), true)
        return nil
      end

      hmodule = module_handle
      result = LibUser32.DialogBoxIndirectParamW(
        hmodule,
        template.not_nil!,
        Pointer(Void).null,
        dialog_proc_callback,
        0,
      )

      if result == -1
        err = LibUser32.GetLastError()
        message_box(resource_error_message(err, "dialog could not be created"), true)
        return nil
      end

      return nil unless @@confirmed

      DownloadParams.new(@@url, @@media_type, @@format, @@output_dir)
    end
  end
end
{% end %}
