require 'json'
require 'hashie/mash'

module Shiika
  # Helps you to create value object class.
  #
  # Example:
  #
  #   class User
  #     extend Props
  #     props name: String, age: Integer, active: :boolean
  #   end
  #   u = User.new("taro", 13)
  #   u.name  #=> "taro"
  #   u.age   #=> 13
  #
  # Note: define `init` method instead of `initialize` when you want
  # to do something on initializaiton.
  module Props
    def self.parse_spec(*given_spec)
      if given_spec.length == 1 && given_spec[0].is_a?(Hash)
        given_spec[0]
      else
        given_spec.map{|s| [s, nil]}.to_h
      end
    end

    def self.conforms?(type, arg)
      case type
      when nil
        true
      when :boolean
        arg == true || arg == false
      when Array
        raise "unkown type spec: #{type.inspect}" unless type.length == 1
        arg.is_a?(Array) && arg.all?{|x| conforms?(type[0], x)}
      when Hash
        raise "unkown type spec: #{type.inspect}" unless type.size == 1
        arg.is_a?(Hash) &&
          arg.keys.all?{|x| conforms?(type.keys.first, x)} &&
          arg.values.all?{|x| conforms?(type.values.first, x)}
      when Module
        arg.is_a?(type)
      else
        raise "unkown type spec: #{type.inspect}"
      end
    end

    def props(*given_spec)
      spec = Props.parse_spec(*given_spec)
      define_singleton_method "props_spec" do spec end

      define_method "initialize" do |*args|
        if args.size != 1 || !args[0].is_a?(Hash)
          raise ArgumentError, "#{self.class}.new takes a Hash (got #{args[0].inspect})"
        end
        hash = args[0]
        if (unknown = hash.keys - spec.keys).any?
          raise ArgumentError, "unknown key(s) for #{self.class}.new: #{unknown.inspect}"
        end
        spec.each do |name, type|
          raise "#{name} must be supplied for #{self.class}.new" unless hash.key?(name)
          value = hash[name]
          if Props.conforms?(type, value)
            instance_variable_set("@#{name}", value)
          else
            raise TypeError, "#{self.class}##{name} expects #{type} but given #{arg.inspect}"
          end
        end
        init
      end
      attr_accessor *spec.keys

      define_method "init" do end
      private "init"

      define_method "props_values" do
        spec.keys.map{|k| [k, instance_variable_get("@#{k}")]}.to_h
      end

      define_method "to_json" do |*args|
        elems = [["class", self.class.name.split(/::/).last]]
        elems.concat(spec.keys.map{|x| [x, instance_variable_get("@#{x}")]})
        return elems.to_h.to_json(*args)
      end

      define_method "serialize" do
        JSON.parse(self.to_json, symbolize_names: true)
      end
    end

    # Add more props (eg. adding some props in a child class)
    def more_props(*more_spec)
      spec = Props.parse_spec(*more_spec)
      props(props_spec.merge(spec))
    end
  end
end
