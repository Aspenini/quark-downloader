require "./progress_parse"

{% if flag?(:windows) %}
  require "./win32_progress"
{% else %}
  require "./tk_ui"
{% end %}

module QuarkGui
  def self.run_download_with_progress(command : String, cmd_args : Array(String)) : Int32
    {% if flag?(:windows) %}
      Win32Progress.run(command, cmd_args)
    {% else %}
      run_download_with_progress_unix(command, cmd_args)
    {% end %}
  end

  {% unless flag?(:windows) %}
    def self.run_download_with_progress_unix(command : String, cmd_args : Array(String)) : Int32
      wish = TkUi.ensure_wish!
      script = TkUi.ensure_script!

      progress = Process.new(
        wish,
        args: [script, "--progress"],
        input: Process::Redirect::Pipe,
        output: Process::Redirect::Close,
        error: Process::Redirect::Close,
      )
      prog_in = progress.input.not_nil!
      relay = ProgressRelay.new

      env = ENV.to_h.merge({"QUARK_GUI" => "1"})
      cli_process = Process.new(
        command: command,
        args: cmd_args,
        env: env,
        output: Process::Redirect::Pipe,
        error: Process::Redirect::Pipe,
      )

      done = Channel(Nil).new(2)

      if stdout = cli_process.output
        spawn do
          stdout.each_line do |line|
            relay.relay(line, prog_in)
          end
          done.send(nil)
        end
      else
        done.send(nil)
      end

      if stderr = cli_process.error
        spawn do
          stderr.each_line do |line|
            relay.relay(line, prog_in)
          end
          done.send(nil)
        end
      else
        done.send(nil)
      end

      2.times { done.receive }
      status = cli_process.wait
      exit_code = if status.success?
                    0
                  else
                    status.exit_code || 1
                  end

      prog_in.puts("DONE\t#{exit_code}")
      prog_in.flush
      prog_in.close

      progress.wait
      if cli_process.exists?
        cli_process.terminate
        cli_process.wait
      end

      TkUi.show_completion(exit_code == 0, exit_code)
      exit_code
    end
  {% end %}
end
