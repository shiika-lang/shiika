module Shiika
  module Type
    class Base; end

    class TyRaw < Base
      @@types = {}
      def self.[](name)
        @@types[name] ||= new(name)
      end

      def initialize(name)
        @name = name
        @@types[name] = self
      end
      attr_reader :name

      def inspect
        "#<TyRaw #{name}>"
      end
      alias to_s inspect
    end

    class TyMethod < Base
      def initialize(name, param_types, ret_type)
        @name, @param_types, @ret_type = name, param_types, ret_type
      end
      attr_reader :name, :param_types, :ret_type
    end

    # Indicates this node has no type (eg. return statement)
    class NoType < Base
    end
  end
end
