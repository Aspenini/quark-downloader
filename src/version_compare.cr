module VersionCompare
  def self.parse(v : String) : Tuple(Int32, Int32, Int32)
    parts = v.lstrip("v").split('.').map(&.to_i)
    {
      parts[0]? || 0,
      parts[1]? || 0,
      parts[2]? || 0,
    }
  end

  # Returns -1 if a < b, 0 if equal, 1 if a > b.
  def self.compare(a : String, b : String) : Int32
    pa = parse(a)
    pb = parse(b)
    return -1 if pa < pb
    return 1 if pa > pb
    0
  end

  def self.at_least?(installed : String, minimum : String) : Bool
    compare(installed, minimum) >= 0
  end

  def self.newer?(latest : String, installed : String) : Bool
    compare(latest, installed) > 0
  end
end
