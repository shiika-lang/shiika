require 'set'
require 'shiika/props'
require 'shiika/program'

module Shiika
  # Ast (Abstract Syntax Tree)
  # - Shiika::Parser generates Ast
  # - Ast can be converted into Shiika::Program with #to_program
  module Ast
    ProgramError = Program::ProgramError

    class Node
      extend Props

      def self.short_name
        self.name.split(/::/).last
      end

      # Return corresponding instance of Shiika::Program::*
      def to_program
        # Get Program::Literal, etc.
        pclass = Program.const_get(self.class.short_name)
        values = self.class.prop_names.map{|x| __send__(x)}
        return pclass.new(*values)
      end

      private

      # Convert [Ast::Node] into [Program::XX]
      def ary_to_program(ary)
        ary.map(&:to_program)
      end
    end

    # The whole program
    # Consists of definitions(defs) and the rest(main)
    class Source < Node
      props :defs, :main

      def to_program
        sk_classes = Shiika::Stdlib.sk_classes
        self.defs.grep(Ast::DefClass).each do |x|
          sk_class, meta_class = x.to_program
          sk_classes[sk_class.name] = sk_class
          sk_classes[meta_class.name] = meta_class
        end
        sk_main = self.main.to_program
        return Program.new(sk_classes, sk_main)
      end
    end

    # Statements written in the toplevel
    class Main < Node
      props :stmts

      def to_program
        Program::Main.new(stmts.map(&:to_program))
      end
    end

    class DefClass < Node
      props :name, :defmethods

      # return [sk_class, meta_class]
      def to_program
        defclassmethods = defmethods.grep(Ast::DefClassMethod)
        sk_class_methods = defclassmethods.map{|x|
          [x.name, x.to_program]
        }.to_h
        sk_methods = (defmethods - defclassmethods).map{|x|
          [x.name, x.to_program]
        }.to_h
        sk_methods["initialize"] ||= Program::SkInitializer.new([], [])
        return Program::SkClass.build(
          name, "Object", sk_methods["initialize"].ivars,
          sk_class_methods,
          sk_methods
        )
      end
    end

    class Defun < Node
      props :name, :params, :ret_type_name, :body_stmts
    end

    class DefClassMethod < Defun
      def to_program
        Program::SkClassMethod.new(name, params.map(&:to_program),
                                   ret_type_name,
                                   ary_to_program(body_stmts))
      end
    end

    class DefMethod < Defun
      def to_program
        Program::SkMethod.new(name, params.map(&:to_program),
                              ret_type_name,
                              ary_to_program(body_stmts))
      end
    end

    class DefInitialize < DefMethod
      props :params, :body_stmts

      def name
        "initialize"
      end

      def to_program
        Program::SkInitializer.new(params.map(&:to_program),
                                   ary_to_program(body_stmts))
      end
    end

    class Param < Node
      props :name, :type_name
    end

    class IParam < Node
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

      def to_program
        Program::If.new(cond_expr.to_program,
                        ary_to_program(then_stmts),
                        ary_to_program(else_stmts))
      end
    end

    class BinExpr < Node
      props :op, :left_expr, :right_expr

      def to_program
        Program::MethodCall.new(left_expr.to_program, op,
                                [right_expr.to_program])
      end
    end

    class UnaryExpr < Node
      props :op, :expr

      def to_program
        Program::MethodCall.new(expr.to_program, op, [])
      end
    end

    class FunCall < Node
      props :name, :args
    end

    class MethodCall < FunCall
      props :receiver_expr, :method_name, :args

      def to_program
        Program::MethodCall.new(receiver_expr.to_program,
                                method_name,
                                ary_to_program(args))
      end
    end

    class AssignLvar < Node
      props :varname, :expr, :isvar

      def to_program
        Program::AssignLvar.new(varname, expr.to_program, isvar)
      end
    end

    class AssignIvar < Node
      props :varname, :expr

      def to_program
        Program::AssignIvar.new(varname, expr.to_program)
      end
    end

    class AssignConst < Node
      props :varname, :expr

      def to_program
        Program::AssignConst.new(varname, expr.to_program)
      end
    end

    class LvarRef < Node
      props :name
    end

    class IvarRef < Node
      props :name
    end

    class ConstRef < Node
      props :name
    end

    class Literal < Node
      props :value
    end
  end
end
