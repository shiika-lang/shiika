module Shiika
  # Represents types in Shiika
  module Type
    class Base; end

    # Type for classes which can have an instance
    # eg. Array<Array<Int>> is OK but Array<Array> is NG
    class ConcreteType < Base
      # Return true if this type conforms to `other` type
      def conforms?(other)
        # TODO: subtypes
        if other.is_a?(TyParam)
          self == TyRaw["Object"]
        else
          self == other
        end
      end
    end

    # Type for normal (i.e. non-generic, non-meta) class
    class TyRaw < ConcreteType
      @@types = {}
      def self.[](name)
        @@types[name] ||= new(name)
      end

      def initialize(name)
        raise "Use TyMeta for #{name}" if name =~ /Meta:/
        @name = name
        @@types[name] = self
      end
      attr_reader :name

      alias to_key name

      # Apply mapping (String => ConcreteType) to this type
      def substitute(mapping)
        # If name is included in the mapping, this TyRaw refers to a type parameter
        # and needs to be substituted with type argument
        mapping[name] || self
      end

      def inspect
        "#<TyRaw #{name}>"
      end
      alias to_s inspect
    end

    # Type for (non-generic) metaclass
    class TyMeta < ConcreteType
      @@types = {}
      def self.[](*args)
        @@types[args] ||= new(*args)
      end

      # eg. TyMeta.new('Array') represents Meta:Array
      def initialize(base_name)
        @base_name = base_name
      end
      attr_reader :base_name

      # Type for the non-meta class (i.e. the instance of this metaclass)
      def instance_type
        TyRaw[@base_name]
      end

      def substitute(mapping)
        self
      end

      def to_key
        "Meta:#{@base_name}"
      end
      alias name to_key
    end

    # Type for generic metaclass
    class TyGenMeta < ConcreteType
      @@types = {}
      def self.[](*args)
        @@types[args] ||= new(*args)
      end

      # eg. TyGenMeta.new('Pair', ['S','T']) represents Meta:Pair<S,T>
      def initialize(base_name, typaram_names)
        @base_name, @typaram_names = base_name, typaram_names
      end
      attr_reader :base_name

      def substitute(mapping)
        self
      end
    end

    # Type for specialized generic class (eg. Pair[Int, Bool])
    class TySpe < ConcreteType
      @@types = {}
      def self.[](*args)
        @@types[args] ||= new(*args)
      end

      # type_args: [TyRaw or TySpe]
      def initialize(base_name, type_args)
        raise unless String === base_name
        @base_name, @type_args = base_name, type_args
      end
      attr_reader :base_name, :type_args

      def base_type
        TyRaw[base_name]
      end

      def substitute(mapping)
        # Find type parameters recursively
        TySpe[base_name, type_args.map{|x| x.substitute(mapping)}]
      end

      def to_key
        @base_name + "<" + @type_args.map(&:to_key).join(', ') + ">"
      end
      alias name to_key
    end

    # Type for metaclass of specialized class
    class TySpeMeta < ConcreteType
      @@types = {}
      def self.[](*args)
        @@types[args] ||= new(*args)
      end

      # type_args: [TyRaw or TySpe]
      def initialize(base_class_name, type_args)
        @base_class_name, @type_args = base_class_name, type_args
      end
      attr_reader :base_class_name, :type_args

      def base_name
        "Meta:#{base_class_name}"
      end

      def name
        "Meta:#{base_class_name}<#{type_args.map(&:name).join(', ')}>"
      end

      # For SkMethod#full_name
      def spclass_name
        "#{base_class_name}<#{type_args.map(&:name).join(', ')}>"
      end

      def instance_type
        TySpe[base_class_name, type_args]
      end

      def substitute(mapping)
        # Find type parameters recursively
        TySpeMeta[base_class_name, type_args.map{|x| x.substitute(mapping)}]
      end
    end

    class TyParam < Base
      def initialize(name)
        @name = name
      end
      attr_reader :name

      def upper_bound
        TyRaw["Object"]
      end

      def conforms?(other)
        # TODO: subtypes
        TyRaw["Object"].conforms?(other)
      end

      def inspect
        "#<TyParam #{name}>"
      end
      alias to_s inspect
    end

    class TyMethod < Base
      def initialize(name, param_types, ret_type)
        @name, @param_types, @ret_type = name, param_types, ret_type
      end
      attr_reader :name, :param_types, :ret_type

      def inspect
        param_types = @param_types.map(&:inspect).join(', ')
        "#<TyMethod (#{param_types})->#{@ret_type.inspect}>"
      end
    end
  end
end
