require "../config"
require "./types"

module QuarkGui
  def self.build_cli_args(cli : String, params : DownloadParams) : Array(String)
    args = [cli]
    params.urls.each { |url| args.concat(["--url", url]) }
    args.concat([
      "--type", params.media_type,
      "--format", params.format,
      "--output-dir", params.output_dir,
      "--no-pause",
    ])
  end

  def self.default_output_dir(cli : String) : String
    {% if flag?(:windows) %}
      # Avoid spawning the console CLI during GUI startup (visible flash).
      QuarkConfig.load!(quiet: true)
      QuarkConfig.download_dir(File.join(ENV["USERPROFILE"]? || ".", "Downloads"))
    {% else %}
      output = IO::Memory.new
      status = Process.run(
        cli,
        args: ["--print-default-output-dir"],
        output: output,
        error: Process::Redirect::Close,
      )
      if status.try(&.success?) && !output.to_s.strip.empty?
        return output.to_s.strip
      end

      home = ENV["HOME"]? || "."
      File.join(home, "Downloads")
    {% end %}
  end
end
