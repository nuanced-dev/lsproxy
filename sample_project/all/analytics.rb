# typed: true
# Analytics module with Sorbet type checking

module Analytics
  extend T::Sig

  sig { params(event_name: String, properties: T::Hash[String, T.untyped]).void }
  def self.track(event_name, properties = {})
    puts "Tracking event: #{event_name}"
    properties.each do |key, value|
      puts "  #{key}: #{value}"
    end
  end

  sig { params(user_id: Integer, action: String).void }
  def self.log_user_action(user_id, action)
    track("user_action", {
      "user_id" => user_id,
      "action" => action,
      "timestamp" => Time.now.iso8601
    })
  end

  sig { returns(T::Hash[String, Integer]) }
  def self.get_stats
    {
      "total_events" => 1000,
      "active_users" => 250,
      "sessions" => 500
    }
  end
end
