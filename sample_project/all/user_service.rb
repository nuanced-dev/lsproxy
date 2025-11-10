# typed: strict
# Sorbet-typed user service

class UserService
  extend T::Sig

  sig { params(user_id: Integer).returns(T.nilable(User)) }
  def find_user(user_id)
    # Simulated database lookup
    return nil if user_id < 1

    User.new(user_id, "User #{user_id}")
  end

  sig { params(name: String, email: String).returns(User) }
  def create_user(name, email)
    # Simulated user creation
    User.new(generate_id, name, email)
  end

  sig { returns(Integer) }
  def generate_id
    Time.now.to_i
  end
end

class User
  extend T::Sig

  sig { returns(Integer) }
  attr_reader :id

  sig { returns(String) }
  attr_reader :name

  sig { returns(T.nilable(String)) }
  attr_reader :email

  sig { params(id: Integer, name: String, email: T.nilable(String)).void }
  def initialize(id, name, email = nil)
    @id = id
    @name = name
    @email = email
  end

  sig { returns(String) }
  def to_s
    "User(#{id}, #{name}, #{email})"
  end
end
