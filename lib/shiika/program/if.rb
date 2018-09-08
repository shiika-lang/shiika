module Shiika
  class Program
    class If < Expression
      props cond_expr: Expression, then_stmts: [Element], else_stmts: [Element]

      def calc_type!(env)
        cond_expr.add_type!(env)
        if cond_expr.type != TyRaw["Bool"]
          raise SkTypeError, "`if` condition must be Bool"
        end
        then_stmts.each{|x| env = x.add_type!(env)}
        else_stmts.each{|x| env = x.add_type!(env)}

        then_type = then_stmts.last&.type
        else_type = else_stmts.last&.type
        if_type = case
                  when then_type && else_type
                    if then_type != else_type
                      raise SkTypeError, "`if` type mismatch (then-clause: #{then_type},"
                      " else-clause: #{else_type})"
                    end
                    then_type
                  when then_type
                    then_type
                  when else_type
                    else_type
                  else
                    TyRaw["Void"]
                  end
        return env, if_type
      end
    end
  end
end
