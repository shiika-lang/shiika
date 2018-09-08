module Shiika
  class Program
    class AssignLvar < AssignmentExpr
      props varname: String, expr: Expression, isvar: :boolean

      def calc_type!(env)
        newenv = super
        lvar = env.find_lvar(varname, allow_missing: true)
        if lvar
          if lvar.kind == :let
            raise SkProgramError, "lvar #{varname} is read-only (missing `var`)"
          end
          unless newenv.conforms_to?(expr.type, lvar.type)
            raise SkTypeError, "the type of expr (#{expr.type}) does not conform to the type of lvar #{varname} (#{lvar.type})"
          end
        else
          lvar = Lvar.new(varname, expr.type, (isvar ? :var : :let))
        end
        retenv = newenv.merge(:local_vars, {varname => lvar})
        return retenv, expr.type
      end
    end
  end
end
