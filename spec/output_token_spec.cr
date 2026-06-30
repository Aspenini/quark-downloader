require "spec"
require "../src/download"

private def tokens(text : String) : Array(String)
  out = [] of String
  QuarkDownload.each_output_token(IO::Memory.new(text)) { |t| out << t }
  out
end

describe "QuarkDownload.each_output_token" do
  # yt-dlp reports progress with carriage returns and only emits a real newline
  # when an item finishes. The stall watchdog must see each "\r" tick, otherwise
  # it mistakes an active download for a stalled one and kills it.
  it "splits on carriage returns as well as newlines" do
    tokens("a\rb\rc\n").should eq(["a", "b", "c"])
  end

  it "yields a trailing token without a terminator" do
    tokens("[download]  50% of 10MiB").should eq(["[download]  50% of 10MiB"])
  end

  it "treats \\r\\n as a single break, not two empty tokens" do
    tokens("line1\r\nline2\n").should eq(["line1", "", "line2"])
  end

  it "returns nothing for empty input" do
    tokens("").should eq([] of String)
  end
end
