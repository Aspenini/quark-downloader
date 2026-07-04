require "http/client"

module ToolHttp
  USER_AGENT = "quark-downloader"

  class FetchError < Exception; end

  def self.fetch_body(url : String) : String
    headers = HTTP::Headers{"User-Agent" => USER_AGENT}
    current = url

    5.times do
      response = HTTP::Client.get(URI.parse(current), headers: headers)

      case response.status_code
      when 200
        body = response.body
        raise FetchError.new("Empty response from #{current}") if body.empty?
        return body
      when 301, 302, 303, 307, 308
        location = response.headers["Location"]?
        raise FetchError.new("Redirect without Location header") unless location

        current = if location.starts_with?("http")
                    location
                  else
                    URI.parse(current).resolve(location).to_s
                  end
      else
        raise FetchError.new("HTTP request failed: #{response.status_code}")
      end
    end

    raise FetchError.new("Too many redirects")
  end

  def self.download_file(url : String, dest : Path)
    File.delete?(dest.to_s) if File.exists?(dest.to_s)
    File.write(dest, fetch_body(url))
  end
end
