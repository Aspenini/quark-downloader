module QuarkGui
  CLI_NAME = {% if flag?(:windows) %} "quark-downloader.exe" {% else %} "quark-downloader" {% end %}

  def self.resolve_cli : String?
    if override = ENV["QUARK_DOWNLOADER_CLI"]?
      return override if File.exists?(override)
    end

    if gui_exe = Process.executable_path
      parent = Path[gui_exe].parent
      unless parent.to_s.gsub('\\', '/').includes?("/crystal/cache")
        sibling = parent / CLI_NAME
        return sibling.to_s if File.exists?(sibling.to_s)
      end
    end

    dev = Path["build"] / CLI_NAME
    return dev.to_s if File.exists?(dev.to_s)

    Process.find_executable("quark-downloader")
  end
end
