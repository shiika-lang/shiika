require 'shiika/props'
require 'shiika/type'

module Shiika
  class Program
    class ProgramError < StandardError; end

    # Convert Ast into Program
    def initialize(ast)
      raise TypeError unless ast.is_a?(Shiika::Ast::Program)

      # Initial environment
      obj_init = SkInitializer.new("Object", [], [])
      obj_class = SkClass.new("Object", :noparent, obj_init, {}, {})
      @sk_classes = {
        "Object" => obj_class
      }

      ast.defs.grep(Ast::DefClass).each do |x|
        @sk_classes[x.name] = x.to_program
      end
      # TODO: Ast::Defun
      @main = ast.main.to_program
    end
    attr_reader :sk_classes, :main

    class Element
      extend Props
    end

    class SkClass < Element
      props :name, # String
            :parent_name, # String or :noparent
            :sk_initializer, # SkInitializer
            :sk_ivars,   # {String => SkIvar},
            :sk_methods  # {String => SkMethod}
    end

    class SkIvar < Element
      props :name, # String
            :type # Shiika::Type
    end

    class SkInitializer < Element
      props :name, # String (class name it belongs to)
            :iparams, # [Param or IParam]
            :body_stmts

      def arity
        @params.length
      end

      def ivars
        iparams.grep(IParam).map{|x|
          SkIvar.new(x.name, x.type)
        }
      end
    end

    class SkMethod < Element
      props :name,
            :ret_type, # Shiika::Type
            :params,
            :body_stmts

      def arity
        @params.length
      end
    end

    class Main < Element
      props :stmts
    end

    class Param < Element
      props :name, :type_name
    end

    class IParam < Element
      props :name, :type_name
    end

    class Extern < Element
      props :ret_type_name, :name, :params
    end

    class For < Element
      props :varname, :var_type_name,
        :begin_expr, :end_expr, :step_expr, :body_stmts
    end

    class Return < Element
      props :expr
    end

    class If < Element
      props :cond_expr, :then_stmts, :else_stmts
    end

    class BinExpr < Element
      props :op, :left_expr, :right_expr
    end

    class UnaryExpr < Element
      props :op, :expr
    end

    class FunCall < Element
      props :name, :args
    end

    class MethodCall < FunCall
      props :receiver_expr, :method_name, :args
    end

    class AssignLvar < Element
      props :varname, :expr, :isvar
    end

    class AssignIvar < Element
      props :varname, :expr
    end

    class AssignConst < Element
      props :varname, :expr
    end

    class VarRef < Element
      props :name
    end

    class Literal < Element
      props :value
    end
  end
end
