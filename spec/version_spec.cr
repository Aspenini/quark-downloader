require "spec"
require "../src/version"

describe QuarkVersion do
  it "exposes a non-empty version from shard.yml" do
    QuarkVersion::VERSION.should_not be_empty
    QuarkVersion.window_title.should contain(QuarkVersion::APP_NAME)
    QuarkVersion.window_title.should contain(QuarkVersion::VERSION)
  end
end
