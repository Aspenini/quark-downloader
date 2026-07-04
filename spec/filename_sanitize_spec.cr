require "spec"
require "file_utils"
require "../src/filename_sanitize"

describe FilenameSanitize do
  describe ".sanitize_component" do
    it "maps yt-dlp fullwidth substitutes to ASCII" do
      cases = {
        "A｜B"  => "A-B",
        "A：B"  => "A-B",
        "A＼B"  => "A-B",
        "A／B"  => "A-B",
        "A？B"  => "AB",
        "A＊B"  => "AB",
        "A＂B＂" => "A'B'",
        "A＜B＞" => "A(B)",
      }
      cases.each do |input, expected|
        FilenameSanitize.sanitize_component(input).should eq(expected)
      end
    end

    it "maps typographic punctuation" do
      FilenameSanitize.sanitize_component("a – b — c − d").should eq("a - b - c - d")
      FilenameSanitize.sanitize_component("“quote” and ‘this’").should eq("'quote' and 'this'")
      FilenameSanitize.sanitize_component("wait… now").should eq("wait... now")
      # Trailing dots are trimmed (invalid on Windows), so a trailing … vanishes.
      FilenameSanitize.sanitize_component("wait…").should eq("wait")
      FilenameSanitize.sanitize_component("1080×720").should eq("1080x720")
    end

    it "handles the example title" do
      title = "The Big Bang Theory Season 6 ｜ Bloopers vs Actual Scene"
      FilenameSanitize.sanitize_component(title).should eq(
        "The Big Bang Theory Season 6 - Bloopers vs Actual Scene"
      )
    end

    it "transliterates accents and drops other non-ASCII" do
      FilenameSanitize.sanitize_component("Café Crème").should eq("Cafe Creme")
      FilenameSanitize.sanitize_component("abc 日本語 def").should eq("abc def")
    end

    it "keeps non-ASCII when ascii_only is false" do
      FilenameSanitize.sanitize_component("Café 日本語", ascii_only: false).should eq("Café 日本語")
    end

    it "always removes path separators and control chars" do
      FilenameSanitize.sanitize_component("a/b\\c", ascii_only: false).should eq("a-b-c")
      FilenameSanitize.sanitize_component("a \u0001bcd", ascii_only: false).should eq("a bcd")
    end

    it "replaces Windows-invalid ASCII characters" do
      FilenameSanitize.sanitize_component(%(a:b|c<d>e"f?g*h)).should eq("a-b-cdefgh")
    end

    it "collapses whitespace" do
      FilenameSanitize.sanitize_component("a \t  b\n c").should eq("a b c")
    end

    it "applies each spaces policy" do
      FilenameSanitize.sanitize_component("a b c", spaces: FilenameSanitize::SpacesPolicy::Keep).should eq("a b c")
      FilenameSanitize.sanitize_component("a b c", spaces: FilenameSanitize::SpacesPolicy::Underscore).should eq("a_b_c")
      FilenameSanitize.sanitize_component("a b c", spaces: FilenameSanitize::SpacesPolicy::Dash).should eq("a-b-c")
      FilenameSanitize.sanitize_component("a b c", spaces: FilenameSanitize::SpacesPolicy::Remove).should eq("abc")
    end

    it "trims leading/trailing spaces and dots" do
      FilenameSanitize.sanitize_component("  name.. ").should eq("name")
      FilenameSanitize.sanitize_component(". hidden .").should eq("hidden")
    end

    it "suffixes Windows reserved names" do
      FilenameSanitize.sanitize_component("CON").should eq("CON_")
      FilenameSanitize.sanitize_component("com1").should eq("com1_")
      FilenameSanitize.sanitize_component("CONCERT").should eq("CONCERT")
    end

    it "falls back for empty results" do
      FilenameSanitize.sanitize_component("").should eq("download")
      FilenameSanitize.sanitize_component("？＊").should eq("download")
    end

    it "truncates long components" do
      long = "a" * 400
      FilenameSanitize.sanitize_component(long).size.should eq(FilenameSanitize::MAX_COMPONENT_LENGTH)
    end
  end

  describe ".sanitize_filename" do
    it "sanitizes the stem and keeps the extension" do
      FilenameSanitize.sanitize_filename("Video ｜ Clip [x].mp4").should eq("Video - Clip [x].mp4")
      FilenameSanitize.sanitize_filename(
        "A B.webm",
        spaces: FilenameSanitize::SpacesPolicy::Underscore,
      ).should eq("A_B.webm")
    end

    it "handles names without extension" do
      FilenameSanitize.sanitize_filename("plain ｜ name").should eq("plain - name")
    end
  end

  describe ".collision_free" do
    it "numbers colliding names and returns free ones unchanged" do
      dir = File.join(Dir.tempdir, "quark-sanitize-#{Time.utc.to_unix_ms}")
      Dir.mkdir_p(dir)
      begin
        FilenameSanitize.collision_free(dir, "a.mp4").should eq("a.mp4")

        File.touch(File.join(dir, "a.mp4"))
        FilenameSanitize.collision_free(dir, "a.mp4").should eq("a (2).mp4")

        File.touch(File.join(dir, "a (2).mp4"))
        FilenameSanitize.collision_free(dir, "a.mp4").should eq("a (3).mp4")
      ensure
        FileUtils.rm_rf(dir)
      end
    end
  end
end
