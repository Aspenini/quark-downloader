require "json"
require "uri"

module QuarkPlaylist
  record ProbeResult, title : String, count : Int32?

  # Heuristic: true for URLs that point at a whole playlist. Watch URLs that
  # merely carry a list= parameter (watch?v=..&list=..) stay single-video.
  def self.playlist_url?(url : String) : Bool
    uri = begin
      URI.parse(url)
    rescue URI::Error
      return false
    end

    host = (uri.host || "").downcase
    path = uri.path.downcase

    return false if host == "youtu.be" || host.ends_with?(".youtu.be")

    return true if path.includes?("/playlist")
    return true if path.includes?("/playlists/")
    return true if path.includes?("/sets/") # SoundCloud

    params = URI::Params.parse(uri.query || "")
    (params.has_key?("list") || params.has_key?("p")) && !params.has_key?("v")
  end

  # Fetches the playlist title (and item count when reported) with a cheap
  # flat extraction. Returns nil on any failure; callers degrade gracefully.
  def self.probe(ytdlp : String, url : String, extra_args : Array(String) = [] of String) : ProbeResult?
    output = IO::Memory.new
    args = ["--flat-playlist", "-I", "1:1", "-J", "--no-warnings"] + extra_args + [url]
    status = Process.run(ytdlp, args: args, output: output, error: Process::Redirect::Close)
    return nil unless status.try(&.success?)

    json = JSON.parse(output.to_s)
    return nil unless json["_type"]?.try(&.as_s?) == "playlist"

    title = json["title"]?.try(&.as_s?)
    return nil unless title && !title.strip.empty?

    ProbeResult.new(title: title.strip, count: json["playlist_count"]?.try(&.as_i?))
  rescue JSON::ParseException
    nil
  end
end
