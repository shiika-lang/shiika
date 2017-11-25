require 'set'
require 'shiika/props'
require 'shiika/program'

module Shiika
  module Ast
    ProgramError = Shiika::Program::ProgramError

    class Node
      extend Props

      def self.short_name
        self.name.split(/::/).last
      end

      # Return corresponding instance of Shiika::Program::*
      def to_program
        # Get Program::Literal, etc.
        pclass = Shiika::Program.const_get(self.class.short_name)
        values = self.class.prop_names.map{|x| __send__(x)}
        return pclass.new(*values)
      end
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

      def to_program
        def_inits = defmethods.grep(DefInitialize)
        raise ProgramError, "duplicated `initialize`" if def_inits.size > 1
        if def_inits.empty?
          sk_initializer = Shiika::Program::SkInitializer.new(name, [], [])
        else
          sk_initializer = def_inits.first.to_program
        end
        sk_methods = (defmethods - def_inits).map(&:to_program)

        return Shiika::Program::SkClass.new(name, "Object", sk_initializer,
                                            sk_initializer.ivars, sk_methods)
      end

      def sk_initializer
        @sk_initializer ||= begin
          def_initialize = defmethods
        end
      end
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
