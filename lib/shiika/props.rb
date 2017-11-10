module Shiika
  # Helps you to create value object class.
  #
  # Example:
  #
  #   class User
  #     extend Props
  #     props :name, :age
  #   end
  #   u = User.new("taro", 13)
  #   u.name  #=> "taro"
  #   u.age   #=> 13
  #
  # Note: define `init` method instead of `initialize` when you want
  # to do something on initializaiton.
  module Props
    def props(*names)
      define_method "initialize" do |*args|
        if names.length != args.length
          raise ArgumentError,
            "wrong number of arguments (given #{args.length}, expected #{names.length})"
        end
        names.zip(args).each do |name, arg|
          instance_variable_set("@#{name}", arg)
        end
        init
      end
      attr_reader *names

      define_method "init", proc{}
      private "init"
    end
  end
end
