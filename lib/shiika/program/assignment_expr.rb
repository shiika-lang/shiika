module Shiika
  class Program
    class AssignmentExpr < Expression
      def calc_type!(env)
        newenv = expr.add_type!(env)
        raise SkProgramError, "cannot assign Void value" if expr.type == TyRaw["Void"]
        return newenv
      end
    end
  end
end
