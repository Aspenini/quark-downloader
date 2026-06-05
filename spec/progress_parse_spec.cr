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

  it "parses yt-dlp ETA values from progress lines" do
    line = "[download]  40.5% of 1.80MiB at 256.29KiB/s ETA 00:04"
    QuarkGui.parse_eta(line).should eq("00:04")
    QuarkGui.eta_status_text("00:04").should eq("Time left: 00:04 left")
    QuarkGui.eta_status_text(nil).should eq("Time left: estimating...")
  end

  it "parses the newest ETA from carriage-return fragments" do
    line = "[download]   1.0% of 1.00MiB ETA 01:00\r[download]   2.0% of 1.00MiB ETA 00:30"
    QuarkGui.parse_progress_percent(line).should eq(2.0)
    QuarkGui.parse_eta(line).should eq("00:30")
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

  it "reserves a setup sliver before real download progress" do
    QuarkGui.display_download_percent(-1.0).should eq(QuarkGui::SETUP_PROGRESS_MAX)
    QuarkGui.display_download_percent(0.0).should eq(QuarkGui::SETUP_PROGRESS_MAX)
    QuarkGui.display_download_percent(50.0).should eq(54.0)
    QuarkGui.display_download_percent(100.0).should eq(100.0)
    QuarkGui.display_download_percent(120.0).should eq(100.0)
  end

  it "nudges setup progress for metadata status lines" do
    setup = 0.0

    10.times do
      if next_percent = QuarkGui.next_setup_progress(setup, "[youtube] abc: Downloading webpage")
        setup = next_percent
      end
    end

    setup.should eq(QuarkGui::SETUP_PROGRESS_MAX)
    QuarkGui.next_setup_progress(setup, "[youtube] abc: Downloading webpage").should be_nil
    QuarkGui.next_setup_progress(2.0, "[download]  5.0% of 1.00MiB").should be_nil
    QuarkGui.next_setup_progress(2.0, "Done.").should be_nil
  end

  it "relays ETA events with download progress" do
    output = IO::Memory.new
    relay = QuarkGui::ProgressRelay.new

    relay.relay("[download]  25.0% of 1.00MiB at 100KiB/s ETA 00:12", output)

    lines = output.to_s.lines.map(&.strip)
    lines.first.starts_with?("PROGRESS\t").should be_true
    lines[1].should eq("ETA\t00:12")
  end
end
