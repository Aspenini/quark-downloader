{% if flag?(:windows) %}
  require "c/processthreadsapi"
  require "c/handleapi"
  require "c/fileapi"
  require "c/jobapi2"

  # Spawn console tools without allocating a visible console window.
  # Crystal's Process.new always creates a console for CONSOLE-subsystem children.
  module Win32HiddenProcess
    CREATE_NO_WINDOW                   = 0x08000000_u32
    CREATE_SUSPENDED                   = 0x00000004_u32
    HANDLE_FLAG_INHERIT                = 0x00000001_u32
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000_u32

    struct PipeAttributes
      @nLength : LibC::DWORD
      @lpSecurityDescriptor : Void*
      @bInheritHandle : LibC::BOOL

      def initialize
        @nLength = sizeof(PipeAttributes).to_u32
        @lpSecurityDescriptor = Pointer(Void).null
        @bInheritHandle = 1
      end
    end

    lib LibKernel32
      fun CreatePipe(
        hReadPipe : LibC::HANDLE*,
        hWritePipe : LibC::HANDLE*,
        lpPipeAttributes : Void*,
        nSize : LibC::DWORD,
      ) : LibC::BOOL

      fun SetHandleInformation(
        hObject : LibC::HANDLE,
        dwMask : LibC::DWORD,
        dwFlags : LibC::DWORD,
      ) : LibC::BOOL

      fun TerminateJobObject(
        hJob : LibC::HANDLE,
        uExitCode : LibC::UInt,
      ) : LibC::BOOL
    end

    class Runner
      getter stdout : IO
      getter stderr : IO

      @process_handle : LibC::HANDLE
      @job_handle : LibC::HANDLE
      @pid : LibC::DWORD

      def initialize(command : String, args : Array(String) = [] of String, env : Process::Env = nil)
        stdout_read, stdout_write = create_pipe_pair
        stderr_read, stderr_write = create_pipe_pair

        @stdout = IO::FileDescriptor.new(handle: stdout_read.address, blocking: true)
        @stderr = IO::FileDescriptor.new(handle: stderr_read.address, blocking: true)

        startup_info = LibC::STARTUPINFOW.new
        startup_info.cb = sizeof(LibC::STARTUPINFOW)
        startup_info.dwFlags = LibC::STARTF_USESTDHANDLES
        startup_info.hStdInput = LibC::INVALID_HANDLE_VALUE
        startup_info.hStdOutput = stdout_write
        startup_info.hStdError = stderr_write

        process_info = LibC::PROCESS_INFORMATION.new
        cmdline = Process.quote_windows([command] + args)
        @job_handle = create_job
        env_block = if env
                      Crystal::System::Env.make_env_block(env, false).as(Void*)
                    else
                      Pointer(Void).null
                    end

        flags = CREATE_NO_WINDOW | CREATE_SUSPENDED | LibC::CREATE_UNICODE_ENVIRONMENT
        if LibC.CreateProcessW(
             nil,
             Crystal::System.to_wstr(cmdline),
             nil,
             nil,
             1,
             flags,
             env_block,
             nil,
             pointerof(startup_info),
             pointerof(process_info),
           ) == 0
          close_handle(stdout_write)
          close_handle(stderr_write)
          close_handle(@job_handle)
          raise IO::Error.from_winerror("CreateProcessW failed")
        end

        close_handle(stdout_write)
        close_handle(stderr_write)

        @process_handle = process_info.hProcess
        @pid = process_info.dwProcessId

        if LibC.AssignProcessToJobObject(@job_handle, @process_handle) == 0
          LibC.TerminateProcess(@process_handle, 1)
          close_handle(process_info.hThread)
          close_handle(@process_handle)
          close_handle(@job_handle)
          raise RuntimeError.from_winerror("AssignProcessToJobObject")
        end

        if LibC.ResumeThread(process_info.hThread) == 0xFFFFFFFF_u32
          LibKernel32.TerminateJobObject(@job_handle, 1)
          close_handle(process_info.hThread)
          close_handle(@process_handle)
          close_handle(@job_handle)
          raise RuntimeError.from_winerror("ResumeThread")
        end

        close_handle(process_info.hThread)
      end

      def wait : Process::Status
        if LibC.WaitForSingleObject(@process_handle, LibC::INFINITE) != LibC::WAIT_OBJECT_0
          raise RuntimeError.from_winerror("WaitForSingleObject")
        end

        exit_code = 0_u32
        if LibC.GetExitCodeProcess(@process_handle, pointerof(exit_code)) == 0
          raise RuntimeError.from_winerror("GetExitCodeProcess")
        end

        Process::Status.new(exit_code)
      end

      # Waits up to timeout_ms for the process to exit. Returns true if it has
      # finished, false if it is still running (timed out). Safe to call from a
      # watchdog thread while another thread blocks on the no-arg wait.
      def wait(timeout_ms : UInt32) : Bool
        LibC.WaitForSingleObject(@process_handle, timeout_ms) == LibC::WAIT_OBJECT_0
      end

      def exists? : Bool
        Crystal::System::Process.exists?(@pid)
      end

      def terminate : Nil
        LibKernel32.TerminateJobObject(@job_handle, 1)
      end

      def finalize
        close_handle(@process_handle)
        close_handle(@job_handle)
      end

      private def create_job : LibC::HANDLE
        job = LibC.CreateJobObjectW(nil, nil)
        if job.null?
          raise RuntimeError.from_winerror("CreateJobObjectW")
        end

        limits = LibC::JOBOBJECT_EXTENDED_LIMIT_INFORMATION.new(
          basicLimitInformation: LibC::JOBOBJECT_BASIC_LIMIT_INFORMATION.new(
            limitFlags: JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
          ),
        )

        if LibC.SetInformationJobObject(
             job,
             LibC::JOBOBJECTINFOCLASS::ExtendedLimitInformation,
             pointerof(limits),
             sizeof(typeof(limits)),
           ) == 0
          close_handle(job)
          raise RuntimeError.from_winerror("SetInformationJobObject")
        end

        job
      end

      private def create_pipe_pair : {LibC::HANDLE, LibC::HANDLE}
        read_pipe = Pointer(Void).null
        write_pipe = Pointer(Void).null
        attrs = PipeAttributes.new

        unless LibKernel32.CreatePipe(pointerof(read_pipe), pointerof(write_pipe), pointerof(attrs), 65536) != 0
          raise IO::Error.from_winerror("CreatePipe failed")
        end

        unless LibKernel32.SetHandleInformation(read_pipe, HANDLE_FLAG_INHERIT, 0) != 0
          close_handle(read_pipe)
          close_handle(write_pipe)
          raise IO::Error.from_winerror("SetHandleInformation failed")
        end

        {read_pipe, write_pipe}
      end

      private def close_handle(handle : LibC::HANDLE) : Nil
        return if handle.null? || handle == LibC::INVALID_HANDLE_VALUE
        LibC.CloseHandle(handle)
      end
    end
  end
{% end %}
