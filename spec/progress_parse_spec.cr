require "spec"
require "../src/gui/progress_parse"

describe QuarkGui do
  it "parses yt-dlp progress percentages" do
    QuarkGui.parse_progress_percent("[download]  42.5% of 10.00MiB").should eq(42.5)
  end

  it "parses carriage-return progress fragments" do
    line = "noise\r[download]  99.9% of 10.00MiB"
    QuarkGui.parse_progress_percent(line).should eq(99.9)
  end

  it "filters and truncates status lines" do
    QuarkGui.parse_status_line("").should be_nil
    QuarkGui.parse_status_line("Done.").should be_nil
    QuarkGui.parse_status_line("Deleting original file x").should be_nil

    long_status = "a" * 100
    status = QuarkGui.parse_status_line(long_status)
    status.should_not be_nil
    status.not_nil!.size.should eq(QuarkGui::STATUS_DISPLAY_MAX)
    status.not_nil!.ends_with?("...").should be_true
  end
end
