module Shiika
  class Program
    class Return < Element
      props expr: Expression
      attr_reader :expr_type

      def calc_type!(env)
        expr.add_type!(env)
        @expr_type = expr.type
        return env, TyRaw["Void"]
      end
    end
  end
end
