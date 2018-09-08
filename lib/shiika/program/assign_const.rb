module Shiika
  class Program
    class AssignConst < AssignmentExpr
      props varname: String, expr: Expression
      
      def calc_type!(env)
        TODO
      end
    end
  end
end
