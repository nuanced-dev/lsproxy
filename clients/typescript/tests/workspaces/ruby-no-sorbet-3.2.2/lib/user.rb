# frozen_string_literal: true

class User
  extend T::Sig

  sig { returns(String) }
  attr_reader :name

  sig { params(name: String, favorite_numbers: T::Array[Integer]).void }
  def initialize(name, favorite_numbers: [])
    @name = name
    @favorite_numbers = favorite_numbers
  end

  sig { returns(T::Array[Integer]) }
  def favorite_numbers
    @favorite_numbers.dup
  end

  sig { params(n: Integer).returns(T::Boolean) }
  def likes?(n)
    @favorite_numbers.include?(n)
  end
end
