require "../version"
require "./controller"
require "./update_check"

ENV["QUARK_VERSION"] = QuarkVersion::VERSION

if ARGV[0]? == "--check-updates"
  QuarkGui::UpdateCheck.run!
  exit 0
end

QuarkGui::Controller.run
