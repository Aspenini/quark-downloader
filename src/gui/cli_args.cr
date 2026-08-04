require "../config"
require "./types"

require "../download"

module QuarkGui
  def self.build_cli_args(cli : String, params : DownloadParams) : Array(String)
    args = [cli]
    params.urls.each { |url| args.concat(["--url", url]) }
    args.concat([
      "--type", params.media_type,
      "--format", params.format,
      "--output-dir", params.output_dir,
      "--no-pause",
      "--emit-result-json",
    ])
  end

  # Single source of truth for the default folder (same as CLI).
  def self.default_output_dir(_cli : String = "") : String
    QuarkDownload.default_output_dir
  end
end
