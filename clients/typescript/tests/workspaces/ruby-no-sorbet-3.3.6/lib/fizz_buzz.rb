# frozen_string_literal: true

module FizzBuzz
  extend T::Sig

  sig { params(n: Integer).returns(String) }
  def self.value(n)
    if (n % 15).zero?
      "FizzBuzz"
    elsif (n % 3).zero?
      "Fizz"
    elsif (n % 5).zero?
      "Buzz"
    else
      n.to_s
    end
  end

  sig { params(limit: Integer).returns(T::Array[String]) }
  def self.series(limit)
    (1..limit).map { |i| value(i) }
  end
end
