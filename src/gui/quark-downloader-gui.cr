require "../version"
require "./controller"

ENV["QUARK_VERSION"] = QuarkVersion::VERSION

QuarkGui::Controller.run
