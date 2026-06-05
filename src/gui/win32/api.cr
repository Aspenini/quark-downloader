{% if flag?(:windows) %}
  require "../../config"
  require "../types"

  module QuarkGui
    module Win32Ui
      IDD_MAIN               =  101
      IDC_URL                = 1001
      IDC_AUDIO              = 1002
      IDC_VIDEO              = 1003
      IDC_FORMAT             = 1004
      IDC_OUTPUT             = 1005
      IDC_BROWSE             = 1006
      IDC_SETTINGS           = 1009
      IDC_SET_DOWNLOAD_DIR   = 1010
      IDC_SET_BROWSE         = 1011
      IDC_SET_YTDLP          = 1012
      IDC_SET_FFMPEG         = 1013
      IDC_SET_GUI_MODE       = 1014
      IDC_SET_LOGS           = 1015
      IDC_MAIN_URL_LABEL     = 1016
      IDC_MAIN_FORMAT_LABEL  = 1017
      IDC_MAIN_OUTPUT_LABEL  = 1018
      IDC_SET_DOWNLOAD_LABEL = 1019
      IDC_SET_YTDLP_LABEL    = 1020
      IDC_SET_FFMPEG_LABEL   = 1021
      IDC_SET_GUI_MODE_LABEL = 1022
      IDC_SET_SAVE           = 1023
      IDC_SET_CANCEL         = 1024
      IDC_CHECK_UPDATES      = 1025

      SW_HIDE = 0
      SW_SHOW = 5

      MAIN_VIEW_IDS = [
        IDC_MAIN_URL_LABEL, IDC_URL, IDC_AUDIO, IDC_VIDEO,
        IDC_MAIN_FORMAT_LABEL, IDC_FORMAT,
        IDC_MAIN_OUTPUT_LABEL, IDC_OUTPUT, IDC_BROWSE,
        IDC_SETTINGS, 1, 2,
      ]

      SETTINGS_VIEW_IDS = [
        IDC_SET_DOWNLOAD_LABEL, IDC_SET_DOWNLOAD_DIR, IDC_SET_BROWSE,
        IDC_SET_YTDLP_LABEL, IDC_SET_YTDLP,
        IDC_SET_FFMPEG_LABEL, IDC_SET_FFMPEG,
        IDC_SET_GUI_MODE_LABEL, IDC_SET_GUI_MODE, IDC_SET_LOGS,
        IDC_CHECK_UPDATES,
        IDC_SET_SAVE, IDC_SET_CANCEL,
      ]

      AUDIO_FORMATS      = ["original", "mp3", "m4a", "flac", "wav", "opus", "vorbis"]
      VIDEO_FORMATS      = ["original", "mp4", "mkv", "webm"]
      TOOL_SOURCE_VALUES = ["auto", "path", "bundled"]
      GUI_MODE_VALUES    = ["progress", "external_cli"]

      WM_INITDIALOG = 0x0110
      WM_COMMAND    = 0x0111
      WM_KEYDOWN    = 0x0100
      BN_CLICKED    =      0

      VK_RETURN = 0x0D_u64
      VK_ESCAPE = 0x1B_u64

      CB_ADDSTRING    = 0x0143
      CB_GETCURSEL    = 0x0147
      CB_GETLBTEXT    = 0x0148
      CB_SETCURSEL    = 0x014E
      CB_RESETCONTENT = 0x014B

      BFFM_INITIALIZED   =      1_u32
      BFFM_SETSELECTIONW = 0x0467_u32 # WM_USER + 103

      BIF_RETURNONLYFSDIRS = 0x0001
      BIF_NEWDIALOGSTYLE   = 0x0040

      MB_OK              = 0x00000000
      MB_YESNO           = 0x00000004
      MB_ICONERROR       = 0x00000010
      MB_ICONINFORMATION = 0x00000040

      IDYES = 6
      IDNO  = 7

      RT_DIALOG = 5

      LANG_NEUTRAL    = 0x00_u16
      SUBLANG_NEUTRAL = 0x00_u16

      alias WinHWND = Void*
      alias WinBOOL = Int32
      alias DialogCallback = Proc(Void*, UInt32, UInt64, UInt64, Int32)
      alias BrowseCallback = Proc(Void*, UInt32, Int64, Int64, Int32)

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
        fun ShowWindow(hWnd : WinHWND, nCmdShow : Int32) : WinBOOL
        fun SetWindowTextW(hWnd : WinHWND, lpString : UInt16*) : WinBOOL
        fun EnableWindow(hWnd : WinHWND, bEnable : WinBOOL) : WinBOOL
        fun PostMessageW(hWnd : WinHWND, msg : UInt32, wParam : UInt64, lParam : Int64) : WinBOOL
      end

      lib LibShell32
        fun SHBrowseForFolderW(lpbi : Void*) : Void*
        fun SHGetPathFromIDListW(pidl : Void*, pszPath : UInt16*) : WinBOOL
        fun ShellExecuteW(
          hwnd : WinHWND,
          lpOperation : UInt16*,
          lpFile : UInt16*,
          lpParameters : UInt16*,
          lpDirectory : UInt16*,
          nShowCmd : Int32,
        ) : Void*
      end

      lib LibComctl32
        ICC_WIN95_CLASSES = 0x000000FF

        struct INITCOMMONCONTROLSEX
          dwSize : UInt32
          dwICC : UInt32
        end

        fun InitCommonControlsEx(lpInitCtrls : INITCOMMONCONTROLSEX*) : WinBOOL
      end

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

      def self.wide(s : String) : Slice(UInt16)
        s.to_utf16
      end

      def self.ensure_common_controls!
        icc = LibComctl32::INITCOMMONCONTROLSEX.new(dwSize: 8_u32, dwICC: LibComctl32::ICC_WIN95_CLASSES)
        LibComctl32.InitCommonControlsEx(pointerof(icc))
      end

      def self.module_handle : Void*
        LibKernel32.GetModuleHandleW(Pointer(UInt16).null)
      end
    end
  end
{% end %}
