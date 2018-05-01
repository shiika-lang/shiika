module Shiika
  # Represents types in Shiika
  module Type
    class Base; end

    # Type for normal (i.e. non-generic, non-meta) class
    class TyRaw < Base
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

      def inspect
        "#<TyRaw #{name}>"
      end
      alias to_s inspect
    end

    # Type for (non-generic) metaclass
    class TyMeta < Base
      @@types = {}
      def self.[](*args)
        @@types[args] ||= new(*args)
      end

      # eg. TyMeta.new('Array') represents Meta:Array
      def initialize(base_name)
        @base_name = base_name
      end
      attr_reader :base_name

      def base_type
        TyRaw[@base_name]
      end

      def to_key
        "Meta:#{@base_name}"
      end
      alias name to_key
    end

    # Type for generic metaclass
    class TyGenMeta < Base
      @@types = {}
      def self.[](*args)
        @@types[args] ||= new(*args)
      end

      # eg. TyGenMeta.new('Pair', ['S','T']) represents Meta:Pair<S,T>
      def initialize(base_name, typaram_names)
        @base_name, @typaram_names = base_name, typaram_names
      end
      attr_reader :base_name

      def base_type
        TyRaw[@base_name]
      end
    end

    # Type for specialized generic class (eg. Pair[Int, Bool])
    class TySpe < Base
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

      def to_key
        @base_name + "[" + @type_args.map(&:to_key).join(', ') + "]"
      end
      alias name to_key
    end

    # Type for metaclass of specialized class
    class TySpeMeta < Base
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
    end

    class TyParam < Base
      def initialize(name)
        @name = name
      end
      attr_reader :name

      def inspect
        "#<TyParam #{name}>"
      end
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
