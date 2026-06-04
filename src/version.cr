module QuarkVersion
  VERSION = {{ `shards version`.strip.stringify }}

  APP_NAME = "Quark Downloader"

  def self.window_title : String
    "#{APP_NAME} #{VERSION}"
  end

  def self.settings_window_title : String
    "#{window_title} Settings"
  end
end
