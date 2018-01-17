require 'json'
require 'hashie/mash'

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
      define_singleton_method "prop_names" do names end

      define_method "initialize" do |*args|
        if names.length != args.length
          raise ArgumentError,
            "wrong number of arguments for #{self.class}.new (given #{args.length}, expected #{names.length})"
        end
        names.zip(args).each do |name, arg|
          instance_variable_set("@#{name}", arg)
        end
        init
      end
      attr_accessor *names

      define_method "init" do end
      private "init"

      define_method "to_json" do |*args|
        elems = [["class", self.class.name.split(/::/).last]]
        elems.concat(names.map{|x| [x, instance_variable_get("@#{x}")]})
        return elems.to_h.to_json(*args)
      end

      define_method "serialize" do
        JSON.parse(self.to_json, symbolize_names: true)
      end
    end

    # Add more props (eg. adding some props in a child class)
    def more_props(*names)
      props(*prop_names, *names)
    end
  end
end
