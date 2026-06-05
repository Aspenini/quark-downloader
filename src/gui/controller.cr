require "../config"
require "./cli_path"
require "./platform_ui"
require "./run_cli"
require "./types"

module QuarkGui
  module Controller
    def self.run : Nil
      cli = QuarkGui.resolve_cli
      unless cli
        PlatformUi.show_missing_cli
        return
      end

      loop do
        QuarkConfig.load!(quiet: true)
        session = PlatformUi.collect_main_session(
          QuarkGui.default_output_dir(cli),
          QuarkConfig.settings,
        )

        if form = session.settings_form
          next unless save_settings(form)
        end

        action = session.action
        case action
        when MainAction::Download
          QuarkGui.run_download(cli, action.params)
          return
        when MainAction::Cancel
          return
        end
      end
    end

    def self.save_settings(form : SettingsForm) : Bool
      if form.download_dir.strip.empty?
        PlatformUi.show_error("Please choose a default download folder.")
        return false
      end

      QuarkConfig.save!(form.to_settings)
      true
    end
  end
end
