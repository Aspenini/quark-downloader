{% if flag?(:windows) %}
  require "./api"

  module QuarkGui
    module Win32Ui
      def self.set_dlg_text(hdlg : WinHWND, id : Int32, text : String)
        buf = wide(text)
        LibUser32.SetDlgItemTextW(hdlg, id, buf.to_unsafe)
      end

      def self.get_dlg_text(hdlg : WinHWND, id : Int32) : String
        buf = Array(UInt16).new(32_768, 0_u16)
        len = LibUser32.GetDlgItemTextW(hdlg, id, buf.to_unsafe, buf.size)
        String.from_utf16(Slice.new(buf.to_unsafe, len))
      end

      def self.populate_combo(hdlg : WinHWND, id : Int32, values : Array(String), selected : String)
        combo = LibUser32.GetDlgItem(hdlg, id)
        LibUser32.SendMessageW(combo, CB_RESETCONTENT, 0, 0)
        selected_index = 0
        values.each_with_index do |value, index|
          w = wide(value)
          LibUser32.SendMessageW(combo, CB_ADDSTRING, 0_u64, w.to_unsafe.address.to_i64)
          selected_index = index if value == selected
        end
        LibUser32.SendMessageW(combo, CB_SETCURSEL, selected_index.to_u64, 0)
      end

      def self.set_dialog_title(hdlg : WinHWND, title : String)
        LibUser32.SetWindowTextW(hdlg, wide(title).to_unsafe)
      end

      def self.set_view_visible(hdlg : WinHWND, ids : Array(Int32), visible : Bool)
        cmd = visible ? SW_SHOW : SW_HIDE
        ids.each do |id|
          child = LibUser32.GetDlgItem(hdlg, id)
          next if child.null?

          LibUser32.ShowWindow(child, cmd)
        end
      end

      def self.populate_settings_fields(hdlg : WinHWND, settings : QuarkConfig::Settings)
        set_dlg_text(hdlg, IDC_SET_DOWNLOAD_DIR, settings.download_dir)
        populate_combo(hdlg, IDC_SET_YTDLP, TOOL_SOURCE_VALUES, settings.yt_dlp.to_config)
        populate_combo(hdlg, IDC_SET_FFMPEG, TOOL_SOURCE_VALUES, settings.ffmpeg.to_config)
        populate_combo(hdlg, IDC_SET_GUI_MODE, GUI_MODE_VALUES, settings.gui_download_mode.to_config)
        LibUser32.CheckDlgButton(hdlg, IDC_SET_LOGS, settings.download_logs ? 1 : 0)
        LibUser32.CheckDlgButton(hdlg, IDC_SET_STRIP_IDS, settings.strip_video_ids ? 1 : 0)
        LibUser32.CheckDlgButton(hdlg, IDC_SET_SANITIZE, settings.sanitize_filenames ? 1 : 0)
        populate_combo(hdlg, IDC_SET_SPACES, SPACES_VALUES, settings.filename_spaces.to_config)
        LibUser32.CheckDlgButton(hdlg, IDC_SET_PLAYLIST_FOLDERS, settings.playlist_folders ? 1 : 0)
      end

      # Windows has no theme picker; the caller passes the current theme
      # through so saving settings does not reset it.
      def self.read_settings_form(hdlg : WinHWND, gui_theme : String) : SettingsForm?
        download_dir = get_dlg_text(hdlg, IDC_SET_DOWNLOAD_DIR).strip
        return nil if download_dir.empty?

        SettingsForm.new(
          download_dir,
          combo_text(hdlg, IDC_SET_YTDLP),
          combo_text(hdlg, IDC_SET_FFMPEG),
          combo_text(hdlg, IDC_SET_GUI_MODE),
          LibUser32.IsDlgButtonChecked(hdlg, IDC_SET_LOGS) != 0,
          gui_theme,
          LibUser32.IsDlgButtonChecked(hdlg, IDC_SET_STRIP_IDS) != 0,
          LibUser32.IsDlgButtonChecked(hdlg, IDC_SET_SANITIZE) != 0,
          combo_text(hdlg, IDC_SET_SPACES),
          LibUser32.IsDlgButtonChecked(hdlg, IDC_SET_PLAYLIST_FOLDERS) != 0,
        )
      end

      def self.listbox_items(hdlg : WinHWND, id : Int32) : Array(String)
        list = LibUser32.GetDlgItem(hdlg, id)
        count = LibUser32.SendMessageW(list, LB_GETCOUNT, 0, 0)
        return [] of String if count <= 0

        items = [] of String
        count.times do |index|
          buf = Array(UInt16).new(4096, 0_u16)
          len = LibUser32.SendMessageW(list, LB_GETTEXT, index.to_u64, buf.to_unsafe.address.to_i64)
          next if len < 0

          text = String.from_utf16(Slice.new(buf.to_unsafe, len.to_i32))
          items << text
        end
        items
      end

      def self.combo_text(hdlg : WinHWND, id : Int32) : String
        combo = LibUser32.GetDlgItem(hdlg, id)
        sel = LibUser32.SendMessageW(combo, CB_GETCURSEL, 0, 0)
        return "" if sel < 0

        buf = Array(UInt16).new(256, 0_u16)
        LibUser32.SendMessageW(combo, CB_GETLBTEXT, sel.to_u64, buf.to_unsafe.address.to_i64)
        text, _ = String.from_utf16(buf.to_unsafe)
        text
      end
    end
  end
{% end %}
