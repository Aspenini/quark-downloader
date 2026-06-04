{% if flag?(:windows) %}
  require "./controls"
  require "./folder_browser"
  require "./message_box"
  require "./resources"

  module QuarkGui
    module Win32Ui
      @@default_output = ""
      @@media_type = "video"
      @@main_action : MainAction::Type = MainAction::Cancel.new
      @@dialog_proc : DialogCallback?
      @@dialog_view = :main
      @@session_settings = QuarkConfig::Settings.new
      @@session_settings_saved = false

      def self.init_main_session(settings : QuarkConfig::Settings)
        @@session_settings = settings
        @@session_settings_saved = false
      end

      def self.session_settings_form : SettingsForm?
        return nil unless @@session_settings_saved

        SettingsForm.new(
          @@session_settings.download_dir,
          @@session_settings.yt_dlp.to_config,
          @@session_settings.ffmpeg.to_config,
          @@session_settings.gui_download_mode.to_config,
          @@session_settings.download_logs,
        )
      end

      def self.show_main_view(hdlg : WinHWND)
        @@dialog_view = :main
        set_dialog_title(hdlg, APP_TITLE)
        set_view_visible(hdlg, SETTINGS_VIEW_IDS, false)
        set_view_visible(hdlg, MAIN_VIEW_IDS, true)
      end

      def self.show_settings_view(hdlg : WinHWND)
        @@dialog_view = :settings
        set_dialog_title(hdlg, "#{APP_TITLE} Settings")
        populate_settings_fields(hdlg, @@session_settings)
        set_view_visible(hdlg, MAIN_VIEW_IDS, false)
        set_view_visible(hdlg, SETTINGS_VIEW_IDS, true)
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

      def self.browse_folder(hdlg : WinHWND) : String?
        browse_folder_for(hdlg, IDC_OUTPUT, @@default_output, "Select output folder")
      end

      def self.browse_settings_folder(hdlg : WinHWND) : String?
        browse_folder_for(
          hdlg,
          IDC_SET_DOWNLOAD_DIR,
          QuarkConfig.expand_path(@@session_settings.download_dir),
          "Select default download folder",
        )
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

        format = combo_text(hdlg, IDC_FORMAT)

        @@media_type = LibUser32.IsDlgButtonChecked(hdlg, IDC_AUDIO) != 0 ? "audio" : "video"
        @@main_action = MainAction::Download.new(DownloadParams.new(
          url,
          @@media_type,
          format.empty? ? "original" : format,
          output,
        ))
        LibUser32.EndDialog(hdlg, 1)
        1
      end

      def self.try_save_settings(hdlg : WinHWND) : WinBOOL
        unless form = read_settings_form(hdlg)
          message_box("Please choose a default download folder.", true)
          return 0
        end

        @@session_settings = form.to_settings
        @@session_settings_saved = true
        show_main_view(hdlg)
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
          LibUser32.CheckDlgButton(hdlg, IDC_VIDEO, 1)
          @@media_type = "video"
          populate_formats(hdlg)
          set_dlg_text(hdlg, IDC_OUTPUT, @@default_output)
          set_dlg_text(hdlg, IDC_SETTINGS, "\u{2699}")
          show_main_view(hdlg)
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
          when IDC_SET_BROWSE
            if notify == BN_CLICKED
              if folder = browse_settings_folder(hdlg)
                set_dlg_text(hdlg, IDC_SET_DOWNLOAD_DIR, folder)
              end
            end
          when IDC_SETTINGS
            if notify == BN_CLICKED
              show_settings_view(hdlg)
              return 1
            end
          when IDC_SET_SAVE
            if notify == BN_CLICKED
              return try_save_settings(hdlg)
            end
          when IDC_SET_CANCEL
            if notify == BN_CLICKED
              show_main_view(hdlg)
              return 1
            end
          when 1 # IDOK
            if notify == BN_CLICKED
              return try_confirm(hdlg) if @@dialog_view == :main
            end
          when 2 # IDCANCEL
            if notify == BN_CLICKED
              if @@dialog_view == :settings
                show_main_view(hdlg)
                return 1
              end

              @@main_action = MainAction::Cancel.new
              LibUser32.EndDialog(hdlg, 0)
              return 1
            end
          end
          0
        when WM_KEYDOWN
          case wparam
          when VK_RETURN
            return try_save_settings(hdlg) if @@dialog_view == :settings

            return try_confirm(hdlg)
          when VK_ESCAPE
            if @@dialog_view == :settings
              show_main_view(hdlg)
              return 1
            end

            @@main_action = MainAction::Cancel.new
            LibUser32.EndDialog(hdlg, 0)
            return 1
          end
          0
        else
          0
        end
      end

      def self.dialog_proc_callback : DialogCallback
        @@dialog_proc ||= DialogCallback.new do |hdlg, msg, wparam, lparam|
          handle_dialog(hdlg, msg, wparam, lparam)
        end
      end

      def self.run_main_dialog(default_output : String) : MainAction::Type
        @@main_action = MainAction::Cancel.new
        @@default_output = default_output

        ensure_common_controls!

        template = load_dialog_template(IDD_MAIN)
        unless template
          err = LibUser32.GetLastError
          message_box(resource_error_message(err, "template not found"), true)
          return MainAction::Cancel.new
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
          err = LibUser32.GetLastError
          message_box(resource_error_message(err, "dialog could not be created"), true)
          return MainAction::Cancel.new
        end

        @@main_action
      end

      def self.collect_main_action(default_output : String) : MainAction::Type
        @@session_settings_saved = false
        run_main_dialog(default_output)
      end

      def self.collect_main_session(default_output : String, settings : QuarkConfig::Settings) : MainSessionResult
        init_main_session(settings)
        action = run_main_dialog(default_output)
        MainSessionResult.new(action, session_settings_form)
      end
    end
  end
{% end %}
