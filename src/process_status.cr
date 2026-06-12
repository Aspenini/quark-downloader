# Process::Status#exit_code raises for signal-killed processes; every place
# that inspects a child's status should go through this instead.
module QuarkProcess
  def self.exit_code(status : Process::Status?, fallback : Int32 = 1) : Int32
    return fallback unless status
    return 0 if status.success?
    return status.exit_code if status.normal_exit?

    fallback
  end
end
