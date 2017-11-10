require 'set'
require 'shiika/props'

module Shiika
  module Ast
    class Node
      extend Props
    end

    # The whole program
    # Consists of definitions(defs) and the rest(main)
    class Program < Node
      props :defs, :main
    end

    # Statements written in the toplevel
    class Main < Node
      props :stmts
    end

    class DefClass < Node
      props :name, :defmethods
    end

    class Defun < Node
      props :name, :params, :ret_type_name, :body_stmts
    end

    class DefMethod < Defun
    end

    class DefInitialize < DefMethod
      props :params, :body_stmts
    end

    class Param < Node
      props :name, :type_name
    end

    class Extern < Node
      props :ret_type_name, :name, :params
    end

    class For < Node
      props :varname, :var_type_name,
        :begin_expr, :end_expr, :step_expr, :body_stmts
    end

    class Return < Node
      props :expr
    end

    class If < Node
      props :cond_expr, :then_stmts, :else_stmts
    end

    class BinExpr < Node
      props :op, :left_expr, :right_expr
    end

    class UnaryExpr < Node
      props :op, :expr
    end

    class FunCall < Node
      props :name, :args
    end

    class MethodCall < FunCall
      props :receiver_expr, :method_name, :args
    end

    class AssignLvar < Node
      props :varname, :expr, :isvar
    end

    class AssignIvar < Node
      props :varname, :expr
    end

    class AssignConst < Node
      props :varname, :expr
    end

    class VarRef < Node
      props :name
    end

    class Literal < Node
      props :value
    end
  end
end
