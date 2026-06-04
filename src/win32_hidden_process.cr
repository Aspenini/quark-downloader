{% if flag?(:windows) %}
require "c/processthreadsapi"
require "c/handleapi"
require "c/fileapi"

# Spawn console tools without allocating a visible console window.
# Crystal's Process.new always creates a console for CONSOLE-subsystem children.
module Win32HiddenProcess
  CREATE_NO_WINDOW = 0x08000000_u32
  HANDLE_FLAG_INHERIT = 0x00000001_u32

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
  end

  class Runner
    getter stdout : IO
    getter stderr : IO

    @process_handle : LibC::HANDLE
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
      env_block = if env
                    Crystal::System::Env.make_env_block(env, false).as(Void*)
                  else
                    Pointer(Void).null
                  end

      flags = CREATE_NO_WINDOW | LibC::CREATE_UNICODE_ENVIRONMENT
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
        raise IO::Error.from_winerror("CreateProcessW failed")
      end

      close_handle(stdout_write)
      close_handle(stderr_write)

      @process_handle = process_info.hProcess
      @pid = process_info.dwProcessId
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

    def exists? : Bool
      Crystal::System::Process.exists?(@pid)
    end

    def terminate : Nil
      LibC.TerminateProcess(@process_handle, 1)
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
      return if handle == LibC::INVALID_HANDLE_VALUE
      LibC.CloseHandle(handle)
    end
  end
end
{% end %}
