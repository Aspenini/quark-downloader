require "http/client"
require "./version"

module ToolHttp
  USER_AGENT = "quark-downloader/#{QuarkVersion::VERSION}"

  class FetchError < Exception; end

  def self.default_headers : HTTP::Headers
    HTTP::Headers{"User-Agent" => USER_AGENT}
  end

  def self.fetch_body(url : String) : String
    headers = default_headers
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

  # Streams the response to a temp file, then renames into place so large
  # tool downloads never need to sit fully in RAM.
  def self.download_file(url : String, dest : Path)
    headers = default_headers
    current = url
    part_path = "#{dest}.part"
    written = false

    File.delete?(part_path) if File.exists?(part_path)
    File.delete?(dest.to_s) if File.exists?(dest.to_s)

    begin
      5.times do
        HTTP::Client.get(URI.parse(current), headers: headers) do |response|
          case response.status_code
          when 200
            File.open(part_path, "w") do |file|
              IO.copy(response.body_io, file)
            end
            begin
              File.rename(part_path, dest.to_s)
            rescue
              File.copy(part_path, dest.to_s)
              File.delete?(part_path)
            end
            written = true
            return
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
      end

      raise FetchError.new("Too many redirects")
    ensure
      File.delete?(part_path) if !written && File.exists?(part_path)
    end
  end
end
