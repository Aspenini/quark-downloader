{% if flag?(:windows) %}
  require "./win32_spawn"
{% else %}
  require "./terminal_launcher"
{% end %}

module QuarkGui
  def self.run_download_external_cli(command : String, cmd_args : Array(String)) : Int32
    {% if flag?(:windows) %}
      Win32Spawn.run_cmd_start_wait("Quark Downloader", command, cmd_args)
    {% else %}
      if terminal = find_terminal
        run_in_terminal(terminal, command, cmd_args)
      else
        status = Process.run(
          command: command,
          args: cmd_args,
          output: Process::Redirect::Inherit,
          error: Process::Redirect::Inherit,
        )
        QuarkProcess.exit_code(status)
      end
    {% end %}
  end
end
