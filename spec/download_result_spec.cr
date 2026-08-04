require "spec"
require "../src/download_result"

describe DownloadResult do
  it "serializes and parses emit lines" do
    result = DownloadResult.new(
      exit_code: 0,
      output_dir: "/tmp/out",
      files: ["/tmp/out/a.mp4"],
      errors: [] of String,
      failed_urls: [] of String,
      log_path: "/tmp/log.txt",
      playlist_error_count: 0,
    )

    line = result.to_emit_line
    line.starts_with?(DownloadResult::RESULT_PREFIX).should be_true

    parsed = DownloadResult.parse_emit_line(line).should_not be_nil
    parsed.exit_code.should eq(0)
    parsed.output_dir.should eq("/tmp/out")
    parsed.files.should eq(["/tmp/out/a.mp4"])
    parsed.log_path.should eq("/tmp/log.txt")
    parsed.success?.should be_true
  end

  it "builds a failure dialog body with errors and log path" do
    result = DownloadResult.new(
      exit_code: 1,
      output_dir: "/tmp/out",
      errors: ["ERROR: boom"],
      failed_urls: ["https://example.com"],
      log_path: "/tmp/x.log",
    )

    body = result.dialog_body
    body.includes?("ERROR: boom").should be_true
    body.includes?("/tmp/x.log").should be_true
    result.success?.should be_false
  end

  it "ignores non-result lines" do
    DownloadResult.parse_emit_line("PROGRESS\t50").should be_nil
  end
end
