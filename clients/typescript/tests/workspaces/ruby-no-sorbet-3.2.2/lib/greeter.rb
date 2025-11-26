# frozen_string_literal: true

require_relative "fizz_buzz"

class Greeter
  extend T::Sig

  sig { params(name: String).void }
  def initialize(name)
    @name = name
  end

  sig { returns(String) }
  def hello
    series = FizzBuzz.series(5).join(", ")
    "Hello, #{@name}! Here's some FizzBuzz: #{series}"
  end
end
