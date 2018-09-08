module Shiika
  class Program
    class Lvar
      # kind : :let, :var, :param, :special
      def initialize(name, type, kind)
        @name, @type, @kind = name, type, kind
      end
      attr_reader :name, :type, :kind

      def inspect
        "#<P::Lvar #{kind} #{name.inspect} #{type}>"
      end
    end
  end
end
