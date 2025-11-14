require 'time'

module LogExecutionTime
  def self.included(base)
    base.extend(ClassMethods)
  end

  module ClassMethods
    def log_time(method_name)
      original_method = instance_method(method_name)
      define_method(method_name) do |*args|
        start_time = Time.now
        result = original_method.bind(self).call(*args)
        end_time = Time.now
        puts "#{method_name} took #{(end_time - start_time).round(2)} seconds to execute"
        result
      end
    end
  end
end
