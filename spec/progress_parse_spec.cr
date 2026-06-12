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

  it "reports prolonged periods without downloader output" do
    QuarkGui.inactivity_status(14_999_u64).should be_nil
    QuarkGui.inactivity_status(15_000_u64).should eq(
      "Waiting for network/server response (15s without output)..."
    )
  end

  it "formats durations as M:SS and H:MM:SS" do
    QuarkGui.format_duration(0_i64).should eq("0:00")
    QuarkGui.format_duration(75_i64).should eq("1:15")
    QuarkGui.format_duration(3_725_i64).should eq("1:02:05")
    QuarkGui.format_duration(-5_i64).should eq("0:00")
  end

  it "estimates remaining playlist time from completed items" do
    # 3 items done (on item 4) in 90s => 30s/item, 97 items left => ~48:30.
    QuarkGui.playlist_eta_text(4, 100, 90_000_u64).should eq("Playlist: ~48:30 left")
  end

  it "withholds a playlist estimate until it has a sample" do
    QuarkGui.playlist_eta_text(1, 100, 5_000_u64).should be_nil   # no item finished yet
    QuarkGui.playlist_eta_text(nil, 100, 5_000_u64).should be_nil # not a playlist
    QuarkGui.playlist_eta_text(6, 5, 40_000_u64).should be_nil    # nothing remaining
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

  it "emits queue context for URL markers and playlist items" do
    output = IO::Memory.new
    relay = QuarkGui::ProgressRelay.new

    relay.relay("==> URL 2 of 5: https://example.com/a", output)
    relay.relay("[download] Downloading item 3 of 12", output)

    lines = output.to_s.lines.map(&.strip)
    lines.should contain("QUEUE\tURL 2 of 5")
    lines.should contain("QUEUE\tURL 2 of 5 - item 3 of 12")
    lines.count("ETA").should eq(2)
  end

  it "resets the bar when a new URL starts" do
    output = IO::Memory.new
    relay = QuarkGui::ProgressRelay.new

    relay.relay("[download]  90.0% of 1.00MiB", output)
    relay.relay("==> URL 2 of 2: https://example.com/b", output)

    lines = output.to_s.lines.map(&.strip)
    lines[-2].should eq("PROGRESS\t0.0")
    lines.last.should eq("ETA")
  end

  it "emits no queue context for plain single downloads" do
    output = IO::Memory.new
    relay = QuarkGui::ProgressRelay.new

    relay.relay("[youtube] abc: Downloading webpage", output)
    relay.relay("[download]  25.0% of 1.00MiB", output)

    output.to_s.lines.any?(&.starts_with?("QUEUE")).should be_false
  end

  it "lets playlist items restart setup progress" do
    output = IO::Memory.new
    relay = QuarkGui::ProgressRelay.new

    relay.relay("[download]  100.0% of 1.00MiB", output)
    relay.relay("[download] Downloading item 2 of 3", output)
    relay.relay("[youtube] abc: Downloading webpage", output)

    lines = output.to_s.lines.map(&.strip)
    lines.should contain("QUEUE\titem 2 of 3")
    # After a new item begins, setup status lines nudge the bar again.
    lines.last.starts_with?("STATUS\t").should be_true
  end
end
