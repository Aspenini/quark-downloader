{% if flag?(:windows) %}
require "./gui_logs"
require "./win32_ui"

module QuarkGui
  module Win32Progress
    IDD_PROGRESS          = 102
    IDC_PROGRESS_STATUS   = 1007
    IDC_PROGRESS_BAR      = 1008

    WM_TIMER              = 0x0113_u32
    WM_COMMAND            = 0x0111_u32
    BN_CLICKED            = 0_u32
    WM_APP_DONE           = 0x8001_u32

    PBM_SETRANGE          = 0x0401_u32
    PBM_SETPOS            = 0x0402_u32
    TIMER_ID              = 1_u32
    TIMER_MS              = 100_u32

    PROGRESS_RE = /\[download\]\s+(\d+(?:\.\d+)?)%/
    STATUS_MAX  = 120

    alias WinHWND = Win32Ui::WinHWND
    alias WinBOOL = Win32Ui::WinBOOL

    @@dialog_hwnd : WinHWND? = nil
    @@log_file : File? = nil
    @@cli_process : Process? = nil
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
      fun SetTimer(hWnd : WinHWND, nIDEvent : UIntPtr, uElapse : UInt32, lpTimerFunc : Void*) : UIntPtr
      fun KillTimer(hWnd : WinHWND, nIDEvent : UIntPtr) : WinBOOL
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
      @@cli_process = nil
      @@log_file = nil

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
      log_file, _path = GuiLogs.open_log
      @@log_file = log_file

      env = ENV.to_h.merge({"QUARK_GUI" => "1"})
      cli = Process.new(
        command: @@pending_command,
        args: @@pending_args,
        env: env,
        output: Process::Redirect::Pipe,
        error: Process::Redirect::Pipe,
      )
      @@cli_process = cli

      done = Channel(Nil).new(2)

      if stdout = cli.output
        spawn do
          stdout.each_line { |line| apply_line(line) }
          done.send(nil)
        end
      else
        done.send(nil)
      end

      if stderr = cli.error
        spawn do
          stderr.each_line { |line| apply_line(line) }
          done.send(nil)
        end
      else
        done.send(nil)
      end

      spawn do
        2.times { done.receive }
        status = cli.wait
        @@cli_exit_code = if status.success?
                            0
                          else
                            status.exit_code || 1
                          end
        @@cli_finished = true
        if hdlg = @@dialog_hwnd
          LibUser32.PostMessageW(hdlg, WM_APP_DONE, @@cli_exit_code.to_u64, 0)
        end
      end

      bar = LibUser32.GetDlgItem(hdlg, IDC_PROGRESS_BAR)
      LibUser32.SendMessageW(bar, PBM_SETRANGE, 0, (100_i64 << 16) | 0)
      LibUser32.SetTimer(hdlg, TIMER_ID, TIMER_MS, Pointer(Void).null)
    end

    def self.apply_line(line : String) : Nil
      log_file = @@log_file
      return unless log_file

      log_file.puts(line)
      log_file.flush

      if m = line.match(PROGRESS_RE)
        @@percent = m[1].to_f
        return
      end

      stripped = line.strip
      return if stripped.empty?

      if stripped.size > STATUS_MAX
        stripped = stripped[0, STATUS_MAX - 3] + "..."
      end
      @@status = stripped
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
        Win32Ui.message_box("Download failed.\n\nLogs: #{GuiLogs.logs_dir}", true)
      end

      if cli = @@cli_process
        if cli.exists?
          cli.terminate
          cli.wait
        end
      end

      @@log_file.try &.close
      @@log_file = nil
      @@cli_process = nil
      LibUser32.EndDialog(hdlg, exit_code.to_i64)
    end

    def self.cancel_download(hdlg : WinHWND) : Nil
      @@cancelled = true
      LibUser32.KillTimer(hdlg, TIMER_ID)

      if cli = @@cli_process
        if cli.exists?
          cli.terminate
          cli.wait
        end
      end

      @@log_file.try &.close
      @@log_file = nil
      @@cli_process = nil
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
        start_download(hdlg)
        1
      when WM_TIMER
        update_controls(hdlg)
        0
      when WM_APP_DONE
        finish_download(hdlg, wparam.to_i32)
        1
      when WM_COMMAND
        id = wparam & 0xFFFF
        notify = (wparam >> 16) & 0xFFFF
        if id == 2 && notify == BN_CLICKED # IDCANCEL
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
