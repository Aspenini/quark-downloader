{% unless flag?(:windows) %}
  module QuarkGui
    TERMINAL_CANDIDATES = [
      {"x-terminal-emulator", ["-e"]},
      {"gnome-terminal", ["--wait", "--"]},
      {"konsole", ["-e"]},
      {"xfce4-terminal", ["-e"]},
      {"alacritty", ["-e"]},
      {"foot", ["-e"]},
    ] of {String, Array(String)}

    def self.find_terminal : Tuple(String, Array(String))?
      TERMINAL_CANDIDATES.each do |name, prefix|
        if path = Process.find_executable(name)
          return {path, prefix}
        end
      end
      nil
    end

    def self.shell_escape(s : String) : String
      "'" + s.gsub("'", "'\\''") + "'"
    end

    def self.run_in_terminal(terminal : Tuple(String, Array(String)), command : String, args : Array(String)) : Int32
      path, prefix = terminal
      inner = ([command] + args).map { |a| shell_escape(a) }.join(" ")
      inner += "; echo; read -r -p 'Press Enter to close...' _"

      status = Process.run(path, args: prefix + ["sh", "-c", inner])
      QuarkProcess.exit_code(status)
    end
  end
{% end %}
