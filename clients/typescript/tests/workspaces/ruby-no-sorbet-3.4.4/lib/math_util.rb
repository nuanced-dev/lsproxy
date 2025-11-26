# frozen_string_literal: true

module Utils
  module MathUtil
    extend T::Sig

    sig { params(xs: T::Array[Integer]).returns(Integer) }
    def self.sum(xs)
      xs.reduce(0, :+)
    end

    sig { params(n: Integer).returns(Integer) }
    def self.factorial(n)
      raise ArgumentError, "n must be >= 0" if n.negative?
      (1..n).reduce(1, :*)
    end
  end
end
