require 'shiika/program'

module Shiika
  class Evaluator
    class KNormalization
      # program: Shiika::Program
      def convert(program)
        KNormalization.reset_last_id
        return Shiika::Program.new(
          convert_sk_classes(program.sk_classes),
          convert_main(program.sk_main)
        )
      end

      private

      def convert_sk_classes(sk_classes)
        sk_classes.transform_values{|x|
          ret = x.dup
          ret.sk_initializer = convert_initializer(x.sk_initializer),
          ret.class_methods = convert_class_methods(x.class_methods),
          ret.sk_methods = convert_sk_methods(x.sk_methods)
          if x.respond_to?(:sk_class) # x is SkMetaClass
            ret.sk_class = x.sk_class
          end
          ret
        }
      end

      def convert_initializer(x)
        Program::SkInitializer.new(x.name,
                                   x.iparams,
                                   convert_stmts(x.body_stmts))
      end

      def convert_class_methods(class_methods)
        class_methods.transform_values{|x|
          Program::SkClassMethod.new(
            x.name,
            x.params,
            x.ret_type_name,
            convert_stmts(x.body_stmts)
          )
        }
      end

      def convert_sk_methods(sk_methods)
        sk_methods.transform_values{|x|
          Program::SkMethod.new(
            x.name,
            x.params,
            x.ret_type_name,
            convert_stmts(x.body_stmts)
          )
        }
      end

      def convert_main(sk_main)
        Program::Main.new(convert_stmts(sk_main.stmts))
      end

      def convert_stmts(stmts)
        return stmts if stmts.is_a?(Proc)
        stmts.flat_map{|x|
          new_stmts, expr = expand_stmt(x)
          new_stmts + [expr]
        }
      end

      # Convert stmt into multiple stmts which does not include nested method call.
      # Return [stmts, name] where name is the name of the local variable
      # which contians the value of the last statement of stmts.
      #
      # eg.
      #   foo(x, bar(y))
      #   =>
      #   tmp1 = bar(y)
      #   foo(x, tmp1)
      def expand_stmt(x)
        case x
        when Program::If
          new_stmts, new_cond_expr = expand_stmt(x.cond_expr)
          x.cond_expr = new_cond_expr
          x.then_stmts = convert_stmts(x.then_stmts)
          x.else_stmts = convert_stmts(x.else_stmts)
          return new_stmts, x
        when Program::MethodCall
          new_stmts, new_receiver_expr = expand_stmt(x.receiver_expr)
          new_arg_exprs = []
          x.args.each do |arg_expr|
            arg_stmts, arg_value_expr = expand_stmt(arg_expr)
            new_stmts.concat(arg_stmts)
            lvar_name = generate_name()
            new_stmts.push(Program::AssignLvar.new(lvar_name, arg_value_expr, :let))
            new_arg_exprs.push(Program::LvarRef.new(lvar_name))
          end
          x.receiver_expr = new_receiver_expr
          x.args = new_arg_exprs
          return new_stmts, x
        when Program::AssignmentExpr
          new_stmts, last_expr = expand_stmt(x.expr)
          x.expr = last_expr
          return new_stmts, x
        else
          return [], x
        end
      end

      # Return true when stmt never contains a method call
      def simple_stmt?(stmt)
        case stmt
        when Program::If,
             Program::MethodCall,
             Program::AssignmentExpr
          false
        else
          true
        end
      end

      @@last_id = 0
      def generate_name
        # Note: this may conflict with user-defined lvar name.
        # I don't care about it because this evaluator is a prototype
        "tmp#{@@last_id+=1}"
      end

      # For unit test
      def self.reset_last_id
        @@last_id = 0
      end
    end
  end
end
