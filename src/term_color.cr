# Minimal ANSI styling for interactive CLI. Disabled when not a TTY, when
# NO_COLOR is set, when TERM=dumb, or when running under the GUI (QUARK_GUI).
module TermColor
  @@enabled : Bool? = nil
  @@windows_vt_tried = false

  def self.enabled? : Bool
    if cached = @@enabled
      return cached
    end

    @@enabled = detect_enabled
  end

  def self.force=(value : Bool) : Nil
    @@enabled = value
  end

  def self.reset! : Nil
    @@enabled = nil
  end

  def self.detect_enabled : Bool
    return true if ENV["FORCE_COLOR"]? == "1"
    return false if ENV["NO_COLOR"]?
    return false if ENV["QUARK_GUI"]? == "1"
    return false if ENV["TERM"]? == "dumb"
    return false unless STDOUT.tty?

    {% if flag?(:windows) %}
      enable_windows_vt!
    {% end %}

    true
  end

  {% if flag?(:windows) %}
    def self.enable_windows_vt! : Nil
      return if @@windows_vt_tried
      @@windows_vt_tried = true

      handle = LibC.GetStdHandle(LibC::STD_OUTPUT_HANDLE)
      return if handle == LibC::INVALID_HANDLE_VALUE || handle.null?

      mode = LibC::DWORD.new(0)
      return if LibC.GetConsoleMode(handle, pointerof(mode)) == 0

      LibC.SetConsoleMode(handle, mode | LibC::ENABLE_VIRTUAL_TERMINAL_PROCESSING)
    rescue
    end
  {% end %}

  def self.wrap(code : String, text : String) : String
    return text unless enabled?

    "\e[#{code}m#{text}\e[0m"
  end

  def self.bold(text : String) : String
    wrap("1", text)
  end

  def self.dim(text : String) : String
    wrap("2", text)
  end

  def self.red(text : String) : String
    wrap("31", text)
  end

  def self.green(text : String) : String
    wrap("32", text)
  end

  def self.yellow(text : String) : String
    wrap("33", text)
  end

  def self.cyan(text : String) : String
    wrap("36", text)
  end
end
