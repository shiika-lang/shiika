module Shiika
  class Type
    class TyRaw < Type
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

    class TyMethod < Type
      def initialize(name, param_tys, ret_ty)
        @name, @param_tys, @ret_ty = name, param_tys, ret_ty
      end
      attr_reader :name, :param_tys, :ret_ty
    end

    # Indicates this node has no type (eg. return statement)
    class NoType < Type
    end
  end
end
