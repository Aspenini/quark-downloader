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
