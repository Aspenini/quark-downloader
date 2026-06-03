require "../config"

module QuarkGui
  module GuiLogs
    MAX_LOGS = 10

    def self.logs_dir : Path
      QuarkConfig.ensure_config_dir!
      QuarkConfig.config_dir / "logs"
    end

    def self.open_log : {File, Path}
      Dir.mkdir_p(logs_dir.to_s)
      timestamp = Time.local.to_s("%Y-%m-%d_%H-%M-%S")
      path = logs_dir / "#{timestamp}.log"
      file = File.open(path, "w")
      prune_old_logs!
      {file, path}
    end

    def self.prune_old_logs! : Nil
      files = Dir.glob(logs_dir.to_s + "/*.log")
        .map { |p| {p, File.info(p).modification_time} }
        .sort_by { |_, t| t }

      excess = files.size - MAX_LOGS
      excess.times do
        entry = files.shift
        next unless entry
        File.delete?(entry[0])
      end
    end
  end
end
