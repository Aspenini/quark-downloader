{% if flag?(:windows) %}
  require "./controls"
  require "./folder_browser"
  require "./message_box"
  require "./resources"

  module QuarkGui
    module Win32Ui
      @@settings_values = QuarkConfig::Settings.new
      @@settings_action : SettingsAction::Type = SettingsAction::Cancel.new
      @@settings_dialog_proc : DialogCallback?

      def self.collect_settings_action(settings : QuarkConfig::Settings) : SettingsAction::Type
        @@settings_values = settings
        @@settings_action = SettingsAction::Cancel.new

        template = load_dialog_template(IDD_SETTINGS)
        unless template
          err = LibUser32.GetLastError
          message_box(resource_error_message(err, "settings template not found"), true)
          return SettingsAction::Cancel.new
        end

        result = LibUser32.DialogBoxIndirectParamW(
          module_handle,
          template.not_nil!,
          Pointer(Void).null,
          settings_dialog_proc_callback,
          0,
        )

        if result == -1
          err = LibUser32.GetLastError
          message_box(resource_error_message(err, "settings dialog could not be created"), true)
          return SettingsAction::Cancel.new
        end

        @@settings_action
      end

      def self.handle_settings_dialog(
        hdlg : WinHWND,
        msg : UInt32,
        wparam : UInt64,
        lparam : UInt64,
      ) : WinBOOL
        case msg
        when WM_INITDIALOG
          set_dlg_text(hdlg, IDC_SET_DOWNLOAD_DIR, @@settings_values.download_dir)
          populate_combo(hdlg, IDC_SET_YTDLP, TOOL_SOURCE_VALUES, @@settings_values.yt_dlp.to_config)
          populate_combo(hdlg, IDC_SET_FFMPEG, TOOL_SOURCE_VALUES, @@settings_values.ffmpeg.to_config)
          populate_combo(hdlg, IDC_SET_GUI_MODE, GUI_MODE_VALUES, @@settings_values.gui_download_mode.to_config)
          LibUser32.CheckDlgButton(hdlg, IDC_SET_LOGS, @@settings_values.download_logs ? 1 : 0)
          1
        when WM_COMMAND
          id = wparam & 0xFFFF
          notify = (wparam >> 16) & 0xFFFF

          case id
          when IDC_SET_BROWSE
            if notify == BN_CLICKED
              if folder = browse_folder_for(
                   hdlg,
                   IDC_SET_DOWNLOAD_DIR,
                   QuarkConfig.expand_path(@@settings_values.download_dir),
                   "Select default download folder",
                 )
                set_dlg_text(hdlg, IDC_SET_DOWNLOAD_DIR, folder)
              end
            end
          when 1 # IDOK
            if notify == BN_CLICKED
              return save_settings_dialog(hdlg)
            end
          when 2 # IDCANCEL
            if notify == BN_CLICKED
              @@settings_action = SettingsAction::Cancel.new
              LibUser32.EndDialog(hdlg, 0)
              return 1
            end
          end
          0
        when WM_KEYDOWN
          case wparam
          when VK_RETURN
            return save_settings_dialog(hdlg)
          when VK_ESCAPE
            @@settings_action = SettingsAction::Cancel.new
            LibUser32.EndDialog(hdlg, 0)
            return 1
          end
          0
        else
          0
        end
      end

      def self.save_settings_dialog(hdlg : WinHWND) : WinBOOL
        download_dir = get_dlg_text(hdlg, IDC_SET_DOWNLOAD_DIR).strip
        if download_dir.empty?
          message_box("Please choose a default download folder.", true)
          return 0
        end

        form = SettingsForm.new(
          download_dir,
          combo_text(hdlg, IDC_SET_YTDLP),
          combo_text(hdlg, IDC_SET_FFMPEG),
          combo_text(hdlg, IDC_SET_GUI_MODE),
          LibUser32.IsDlgButtonChecked(hdlg, IDC_SET_LOGS) != 0,
        )
        @@settings_action = SettingsAction::Save.new(form)
        LibUser32.EndDialog(hdlg, 1)
        1
      end

      def self.settings_dialog_proc_callback : DialogCallback
        @@settings_dialog_proc ||= DialogCallback.new do |hdlg, msg, wparam, lparam|
          handle_settings_dialog(hdlg, msg, wparam, lparam)
        end
      end
    end
  end
{% end %}
