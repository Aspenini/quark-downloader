require "file_utils"
require "spec"
require "../src/logs"

describe QuarkLogs do
  it "keeps only the newest rotated logs" do
    dir = Path[Dir.tempdir] / "quark-logs-#{Time.utc.to_unix_ms}"
    Dir.mkdir_p(dir.to_s)

    12.times do |i|
      path = dir / "download-#{i}.log"
      File.write(path.to_s, i.to_s)
      time = Time.utc - (12 - i).seconds
      File.utime(time, time, path.to_s)
    end

    QuarkLogs.prune_old_logs!(dir)

    files = Dir.glob(dir.to_s.gsub('\\', '/') + "/*.log").map { |path| Path[path].basename.to_s }.sort
    files.size.should eq(QuarkLogs::MAX_LOGS)
    files.includes?("download-0.log").should be_false
    files.includes?("download-1.log").should be_false
  ensure
    FileUtils.rm_rf(dir.to_s) if dir
  end
end
