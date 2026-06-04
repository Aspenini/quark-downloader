{% if flag?(:windows) %}
  require "../win32_hidden_process"
  require "../config"
  require "../logs"
  require "./progress_parse"
  require "./win32_ui"

  module QuarkGui
    module Win32Progress
      IDD_PROGRESS        =  102
      IDC_PROGRESS_STATUS = 1007
      IDC_PROGRESS_BAR    = 1008

      WM_TIMER        = 0x0113_u32
      WM_COMMAND      = 0x0111_u32
      BN_CLICKED      =      0_u32
      WM_APP_DONE     = 0x8001_u32
      WM_APP_PROGRESS = 0x8002_u32

      PBM_SETRANGE = 0x0401_u32
      PBM_SETPOS   = 0x0402_u32
      TIMER_ID     =      1_u32
      TIMER_MS     =    100_u32

      WM_KEYDOWN = 0x0100_u32
      VK_ESCAPE  =   0x1B_u64

      alias WinHWND = Win32Ui::WinHWND
      alias WinBOOL = Win32Ui::WinBOOL

      @@dialog_hwnd : WinHWND? = nil
      @@cli_runner : Win32HiddenProcess::Runner? = nil
      @@cli_finished = false
      @@cli_exit_code = 1
      @@cancelled = false
      @@percent = 0.0
      @@status = "Starting download..."
      @@pending_command = ""
      @@pending_args = [] of String

      alias DialogCallback = Win32Ui::DialogCallback
      @@dialog_proc : DialogCallback?

      lib LibUser32
        fun SetTimer(hWnd : WinHWND, nIDEvent : LibC::UINT_PTR, uElapse : UInt32, lpTimerFunc : Void*) : LibC::UINT_PTR
        fun KillTimer(hWnd : WinHWND, nIDEvent : LibC::UINT_PTR) : WinBOOL
        fun GetDlgItem(hDlg : WinHWND, nIDDlgItem : Int32) : WinHWND
        fun SendMessageW(hWnd : WinHWND, msg : UInt32, wParam : UInt64, lParam : Int64) : Int64
        fun EndDialog(hDlg : WinHWND, nResult : Int64) : WinBOOL
        fun DialogBoxIndirectParamW(
          hInstance : Void*,
          hDialogTemplate : Void*,
          hWndParent : WinHWND,
          lpDialogFunc : DialogCallback,
          dwInitParam : Int64,
        ) : Int64
        fun PostMessageW(hWnd : WinHWND, msg : UInt32, wParam : UInt64, lParam : Int64) : WinBOOL
      end

      def self.run(command : String, cmd_args : Array(String)) : Int32
        @@pending_command = command
        @@pending_args = cmd_args
        @@cli_finished = false
        @@cli_exit_code = 1
        @@cancelled = false
        @@percent = 0.0
        @@status = "Starting download..."
        @@dialog_hwnd = nil
        @@cli_runner = nil

        Win32Ui.ensure_common_controls!

        template = Win32Ui.load_dialog_template(IDD_PROGRESS)
        unless template
          Win32Ui.message_box("Could not load the progress dialog.", true)
          return 1
        end

        hmodule = Win32Ui.module_handle
        result = LibUser32.DialogBoxIndirectParamW(
          hmodule,
          template.not_nil!,
          Pointer(Void).null,
          dialog_proc_callback,
          0,
        )

        if @@cancelled
          1
        elsif result == -1
          1
        else
          @@cli_exit_code
        end
      end

      def self.start_download(hdlg : WinHWND) : Nil
        runner = Win32HiddenProcess::Runner.new(
          @@pending_command,
          @@pending_args,
          env: {"QUARK_GUI" => "1"},
        )
        @@cli_runner = runner

        Thread.new(name: "gui-cli-stdout") do
          runner.stdout.each_line { |line| apply_line(line) }
        rescue ex
          apply_line("Error reading download output: #{ex.message}")
        end

        Thread.new(name: "gui-cli-stderr") do
          runner.stderr.each_line { |line| apply_line(line) }
        rescue ex
          apply_line("Error reading download output: #{ex.message}")
        end

        Thread.new(name: "gui-cli-wait") do
          status = runner.wait
          @@cli_exit_code = if status.success?
                              0
                            else
                              status.exit_code || 1
                            end
          @@cli_finished = true
          LibUser32.PostMessageW(hdlg, WM_APP_DONE, @@cli_exit_code.to_u64, 0)
        rescue ex
          @@cli_exit_code = 1
          @@status = ex.message || "Download failed."
          LibUser32.PostMessageW(hdlg, WM_APP_DONE, 1_u64, 0)
        end

        bar = LibUser32.GetDlgItem(hdlg, IDC_PROGRESS_BAR)
        LibUser32.SendMessageW(bar, PBM_SETRANGE, 0, (100_i64 << 16) | 0)
        LibUser32.SetTimer(hdlg, TIMER_ID, TIMER_MS, Pointer(Void).null)
      rescue ex : IO::Error | RuntimeError
        Win32Ui.message_box("Could not start download:\n#{ex.message}", true)
        LibUser32.PostMessageW(hdlg, WM_APP_DONE, 1_u64, 0)
      end

      def self.apply_line(line : String) : Nil
        if percent = QuarkGui.parse_progress_percent(line)
          @@percent = percent
          if hdlg = @@dialog_hwnd
            LibUser32.PostMessageW(hdlg, WM_APP_PROGRESS, 0, 0)
          end
          return
        end

        if status = QuarkGui.parse_status_line(line)
          @@status = status
        end

        if hdlg = @@dialog_hwnd
          LibUser32.PostMessageW(hdlg, WM_APP_PROGRESS, 0, 0)
        end
      end

      def self.update_controls(hdlg : WinHWND) : Nil
        Win32Ui.set_dlg_text(hdlg, IDC_PROGRESS_STATUS, @@status)
        bar = LibUser32.GetDlgItem(hdlg, IDC_PROGRESS_BAR)
        LibUser32.SendMessageW(bar, PBM_SETPOS, @@percent.to_i32, 0)
      end

      def self.finish_download(hdlg : WinHWND, exit_code : Int32) : Nil
        LibUser32.KillTimer(hdlg, TIMER_ID)

        if exit_code == 0
          @@percent = 100.0
          @@status = "Done."
          update_controls(hdlg)
          Win32Ui.message_box("Download Complete!")
        else
          message = "Download failed."
          message += "\n\nLogs: #{QuarkLogs.logs_dir}" if QuarkConfig.download_logs?
          Win32Ui.message_box(message, true)
        end

        if runner = @@cli_runner
          if runner.exists?
            runner.terminate
            runner.wait
          end
        end

        @@cli_runner = nil
        LibUser32.EndDialog(hdlg, exit_code.to_i64)
      end

      def self.cancel_download(hdlg : WinHWND) : Nil
        @@cancelled = true
        LibUser32.KillTimer(hdlg, TIMER_ID)

        if runner = @@cli_runner
          if runner.exists?
            runner.terminate
            runner.wait
          end
        end

        @@cli_runner = nil
        LibUser32.EndDialog(hdlg, 0)
      end

      def self.handle_dialog(
        hdlg : WinHWND,
        msg : UInt32,
        wparam : UInt64,
        lparam : UInt64,
      ) : WinBOOL
        case msg
        when Win32Ui::WM_INITDIALOG
          @@dialog_hwnd = hdlg
          Win32Ui.set_dialog_title(hdlg, WINDOW_TITLE)
          start_download(hdlg)
          1
        when WM_TIMER
          update_controls(hdlg)
          1
        when WM_APP_DONE
          finish_download(hdlg, wparam.to_i32)
          1
        when WM_APP_PROGRESS
          update_controls(hdlg)
          1
        when WM_COMMAND
          id = wparam & 0xFFFF
          notify = (wparam >> 16) & 0xFFFF
          if id == 2 && notify == BN_CLICKED # IDCANCEL
            cancel_download(hdlg)
            return 1
          end
          0
        when WM_KEYDOWN
          if wparam == VK_ESCAPE
            cancel_download(hdlg)
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
    end
  end
{% end %}
