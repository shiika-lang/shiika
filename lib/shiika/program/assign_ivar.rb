module Shiika
  class Program
    class AssignIvar < AssignmentExpr
      props varname: String, expr: Expression

      def calc_type!(env)
        newenv = super
        ivar = env.find_ivar(varname)
        if ivar.type != expr.type  # TODO: subtypes
          raise SkTypeError, "ivar #{varname} of class #{env.sk_self} is #{ivar.type} but expr is #{expr.type}"
        end
        return newenv, expr.type
      end
    end
  end
end
