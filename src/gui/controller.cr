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
        action = PlatformUi.collect_main_action(QuarkGui.default_output_dir(cli))

        case action
        when MainAction::Download
          QuarkGui.run_download(cli, action.params)
          return
        when MainAction::Settings
          edit_settings
        when MainAction::Cancel
          return
        end
      end
    end

    def self.edit_settings : Nil
      QuarkConfig.load!(quiet: true)
      action = PlatformUi.collect_settings_action(QuarkConfig.settings)

      case action
      when SettingsAction::Save
        save_settings(action.form)
      when SettingsAction::Cancel
      end
    rescue ex
      PlatformUi.show_error("Could not save settings:\n#{ex.message}")
    end

    def self.save_settings(form : SettingsForm) : Nil
      if form.download_dir.strip.empty?
        PlatformUi.show_error("Please choose a default download folder.")
        return
      end

      QuarkConfig.save!(form.to_settings)
    end
  end
end
