require "../config"
require "./cli_args"
require "./external_cli_runner"
require "./progress_runner"
require "./types"

module QuarkGui
  def self.run_download(cli : String, params : DownloadParams) : Int32
    QuarkConfig.load!(quiet: true)

    args = build_cli_args(cli, params)
    command = args.first
    cmd_args = args[1..]? || [] of String

    case QuarkConfig.gui_download_mode
    when .external_cli?
      run_download_external_cli(command, cmd_args)
    else
      run_download_with_progress(command, cmd_args)
    end
  end
end
