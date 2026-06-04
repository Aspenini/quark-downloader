require "./config"

module QuarkLogs
  MAX_LOGS = 10

  @@file : File? = nil
  @@path : Path? = nil
  @@lock = Mutex.new

  def self.logs_dir : Path
    QuarkConfig.ensure_config_dir!
    QuarkConfig.config_dir / "logs"
  end

  def self.active_path : Path?
    @@path
  end

  def self.open_download_log : Path?
    return nil unless QuarkConfig.download_logs?
    return @@path if @@file

    file, path = open_log
    @@file = file
    @@path = path
    path
  end

  def self.open_log(dir : Path = logs_dir) : {File, Path}
    Dir.mkdir_p(dir.to_s)
    timestamp = Time.local.to_s("%Y-%m-%d_%H-%M-%S")
    path = dir / "#{timestamp}.log"
    file = File.open(path, "w")
    prune_old_logs!(dir)
    {file, path}
  end

  def self.prune_old_logs!(dir : Path = logs_dir) : Nil
    pattern = dir.to_s.gsub('\\', '/') + "/*.log"
    files = Dir.glob(pattern)
      .map { |p| {p, File.info(p).modification_time} }
      .sort_by { |_, t| t }

    excess = files.size - MAX_LOGS
    excess.times do
      entry = files.shift
      next unless entry
      File.delete?(entry[0])
    end
  end

  def self.puts(message = "", io : IO = STDOUT) : Nil
    @@lock.synchronize do
      io.puts(message)
      if file = @@file
        file.puts(message)
        file.flush
      end
    end
  rescue IO::Error
  end

  def self.close : Nil
    @@lock.synchronize do
      @@file.try(&.close)
      @@file = nil
      @@path = nil
    end
  end
end
