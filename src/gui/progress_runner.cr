require "../process_status"
require "./progress_parse"

{% if flag?(:windows) %}
  require "./win32_progress"
{% else %}
  require "./platform_ui"
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
    def self.spawn_progress_ui : Process
      theme = QuarkConfig.gui_theme.to_config

      {% if flag?(:darwin) %}
        if helper = MacUi.helper_path
          return Process.new(
            helper,
            args: ["--progress", "", theme],
            input: Process::Redirect::Pipe,
            output: Process::Redirect::Close,
            error: Process::Redirect::Close,
          )
        end
      {% end %}

      wish = TkUi.ensure_wish!
      script = TkUi.ensure_script!
      Process.new(
        wish,
        args: [script, "--progress", "", theme],
        input: Process::Redirect::Pipe,
        output: Process::Redirect::Close,
        error: Process::Redirect::Close,
      )
    end

    def self.run_download_with_progress_unix(command : String, cmd_args : Array(String)) : Int32
      progress = spawn_progress_ui
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

      # Closing the progress window means cancel: stop the download instead
      # of letting it run on invisibly.
      download_finished = false
      user_closed = false
      progress_exited = Channel(Nil).new(1)
      spawn do
        progress.wait
        unless download_finished
          user_closed = true
          begin
            cli_process.terminate
          rescue
          end
        end
        progress_exited.send(nil)
      end

      ui_alive = true
      done = Channel(Nil).new(2)
      forward = ->(io : IO) do
        # Keep draining CLI output even after the UI is gone, so the CLI
        # never blocks on a full pipe while shutting down.
        begin
          io.each_line do |line|
            next unless ui_alive
            begin
              relay.relay(line, prog_in)
            rescue IO::Error
              ui_alive = false
            end
          end
        rescue IO::Error
        end
      end

      if stdout = cli_process.output
        spawn do
          begin
            forward.call(stdout)
          ensure
            done.send(nil)
          end
        end
      else
        done.send(nil)
      end

      if stderr = cli_process.error
        spawn do
          begin
            forward.call(stderr)
          ensure
            done.send(nil)
          end
        end
      else
        done.send(nil)
      end

      2.times { done.receive }
      status = cli_process.wait
      download_finished = true
      exit_code = QuarkProcess.exit_code(status)

      if user_closed
        # Cancelled by the user; no completion popup.
        return 1
      end

      begin
        prog_in.puts("DONE\t#{exit_code}")
        prog_in.flush
        prog_in.close
      rescue IO::Error
      end

      progress_exited.receive

      PlatformUi.show_completion(exit_code == 0, exit_code)
      exit_code
    end
  {% end %}
end
