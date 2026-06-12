require "spec"
require "../src/playlist"

describe QuarkPlaylist do
  describe ".playlist_url?" do
    it "detects playlist URLs" do
      QuarkPlaylist.playlist_url?("https://www.youtube.com/playlist?list=PLx").should be_true
      QuarkPlaylist.playlist_url?("https://youtube.com/playlist?list=PLx&si=abc").should be_true
      QuarkPlaylist.playlist_url?("https://www.youtube.com/watch?list=PLx").should be_true
      QuarkPlaylist.playlist_url?("https://soundcloud.com/artist/sets/my-mix").should be_true
      QuarkPlaylist.playlist_url?("https://example.com/playlists/123").should be_true
    end

    it "treats watch URLs with a list parameter as single videos" do
      QuarkPlaylist.playlist_url?("https://www.youtube.com/watch?v=KF5gdofOO2k&list=PLx").should be_false
      QuarkPlaylist.playlist_url?("https://www.youtube.com/watch?v=KF5gdofOO2k").should be_false
    end

    it "treats short links and plain URLs as single videos" do
      QuarkPlaylist.playlist_url?("https://youtu.be/KF5gdofOO2k").should be_false
      QuarkPlaylist.playlist_url?("https://youtu.be/KF5gdofOO2k?list=PLx").should be_false
      QuarkPlaylist.playlist_url?("https://vimeo.com/12345").should be_false
      QuarkPlaylist.playlist_url?("not a url").should be_false
    end
  end
end
