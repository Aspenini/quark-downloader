{% if flag?(:windows) %}
  module QuarkGui
    module Win32Spawn
      CREATE_NO_WINDOW = 0x08000000_u32

      def self.run_cmd_start_wait(title : String, command : String, args : Array(String)) : Int32
        spawn_args = ["/c", "start", "/wait", title, command] + args
        cmdline = Process.quote_windows(["cmd.exe"] + spawn_args)

        startup_info = LibC::STARTUPINFOW.new
        startup_info.cb = sizeof(LibC::STARTUPINFOW)

        process_info = LibC::PROCESS_INFORMATION.new

        if LibC.CreateProcessW(
             nil,
             Crystal::System.to_wstr(cmdline),
             nil,
             nil,
             0,
             CREATE_NO_WINDOW,
             nil,
             nil,
             pointerof(startup_info),
             pointerof(process_info),
           ) == 0
          return 1
        end

        LibC.WaitForSingleObject(process_info.hProcess, LibC::INFINITE)
        exit_code = 0_u32
        LibC.GetExitCodeProcess(process_info.hProcess, pointerof(exit_code))
        LibC.CloseHandle(process_info.hProcess)
        LibC.CloseHandle(process_info.hThread)
        exit_code.to_i32
      end
    end
  end
{% end %}
