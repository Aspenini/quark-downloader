require "spec"
require "../src/destination_tracker"

describe DestinationTracker do
  it "tracks each destination line form" do
    tracker = DestinationTracker.new
    tracker.observe("[download] Destination: /tmp/Video Title.f137.mp4")
    tracker.observe(%([Merger] Merging formats into "/tmp/Video Title.mp4"))
    tracker.observe("[ExtractAudio] Destination: /tmp/Audio Title.mp3")
    tracker.observe("[VideoConvertor] Destination: /tmp/Clip.webm")
    tracker.observe("[VideoRemuxer] Destination: /tmp/Clip.mp4")
    tracker.observe("[download] /tmp/Old Video.mp4 has already been downloaded")

    tracker.paths.should eq([
      "/tmp/Video Title.f137.mp4",
      "/tmp/Video Title.mp4",
      "/tmp/Audio Title.mp3",
      "/tmp/Clip.webm",
      "/tmp/Clip.mp4",
      "/tmp/Old Video.mp4",
    ])
  end

  it "ignores unrelated lines and deduplicates" do
    tracker = DestinationTracker.new
    tracker.observe("[download]  42.0% of 10.00MiB at 2.00MiB/s ETA 00:05")
    tracker.observe("[youtube] KF5gdofOO2k: Downloading webpage")
    tracker.observe("[download] Destination: /tmp/a.mp4")
    tracker.observe("[download] Destination: /tmp/a.mp4")

    tracker.paths.should eq(["/tmp/a.mp4"])
  end

  it "handles carriage-return packed lines" do
    tracker = DestinationTracker.new
    tracker.observe("[download]  10%\r[download] Destination: /tmp/b.mp4")

    tracker.paths.should eq(["/tmp/b.mp4"])
  end

  it "counts ERROR lines" do
    tracker = DestinationTracker.new
    tracker.observe("ERROR: [youtube] abc: Video unavailable")
    tracker.observe("ERROR: [youtube] def: Private video")
    tracker.observe("[download] Destination: /tmp/c.mp4")

    tracker.error_count.should eq(2)
    tracker.paths.should eq(["/tmp/c.mp4"])
  end
end
