require 'shiika/props'
require 'shiika/type'
require 'shiika/program/env'

module Shiika
  class Program
    class ProgramError < StandardError; end
    class SkTypeError < StandardError; end
    class SkNameError < StandardError; end

    # Convert Ast into Program
    def initialize(ast)
      raise TypeError unless ast.is_a?(Ast::Source)

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
      @sk_main = ast.main.to_program
    end
    attr_reader :sk_classes, :sk_main

    def add_type!
      # Do nothing if already typed
      return if @sk_main.type

      env = Shiika::Program::Env.new({
        sk_classes: @sk_classes
      })
      @sk_classes.each_value{|x| x.add_type!(env)}
      @sk_main.add_type!(env)
    end

    class Element
      include Type
      extend Props

      def add_type!(env)
        newenv, @type = calc_type!(env)
        return newenv
      end
      attr_reader :type

      def calc_type!(env)
        raise "override me"
      end
    end

    class SkClass < Element
      props :name, # String
            :parent_name, # String or :noparent
            :sk_initializer, # SkInitializer
            :sk_ivars,   # {String => SkIvar},
            :sk_methods  # {String => SkMethod}

      def calc_type!(env)
        @sk_initializer.add_type!(env)
        @sk_methods.each{|x| x.add_type!(env)}
        return env, TyRaw[name]
      end
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

      def calc_type!(env)
        body_stmts.each{|x| env = x.add_type!(env)}
        param_tys = iparams.map(&:type)
        return env, TyMethod.new("initialize", param_tys, TyRaw["Void"])
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

      def calc_type!(env)
        TODO
      end
    end

    class Main < Element
      props :stmts

      def calc_type!(env)
        stmts.each{|x| env = x.add_type!(env)}
        return env, (stmts.last ? stmts.last.type : TyRaw["Void"])
      end
    end

    class Param < Element
      props :name, :type_name
    end

    class IParam < Element
      props :name, :type_name

      def calc_type!(env)
        return env, env.find_type(type_name)
      end
    end

    class Return < Element
      props :expr

      def calc_type!(env)
        return env, TyRaw["Void"]
      end
    end

    class If < Element
      props :cond_expr, :then_stmts, :else_stmts

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

    class MethodCall < Element
      props :receiver_expr, :method_name, :args

      def calc_type!(env)
        TODO
        return ty, env
      end
    end

    class AssignmentExpr < Element
      def calc_type!(env)
        expr.add_type!(env)
        raise ProgramError, "cannot assign Void value" if expr.type == TyRaw["Void"]
      end
    end

    class AssignLvar < AssignmentExpr
      props :varname, :expr, :isvar

      def calc_type!(env)
        super
        lvar = Lvar.new(varname, expr.type, (isvar ? :var : :let))
        newenv = env.merge(:local_vars, {varname => lvar})
        return newenv, expr.type
      end
    end

    class AssignIvar < AssignmentExpr
      props :varname, :expr

      def calc_type!(env)
        super
        ivar = env.find_ivar(name)
        if ivar.type == expr.type
          raise SkTypeError, "ivar #{name} is #{ivar.type} but expr is #{expr.type}"
        end
        # TODO: raise error for assignment to let
        return env, expr.type
      end
    end

    class AssignConst < AssignmentExpr
      props :varname, :expr
      
      def calc_type!(env)
        TODO
      end
    end

    class LvarRef < Element
      props :name

      def calc_type!(env)
        lvar = env.find_lvar(name)
        return env, lvar.type
      end
    end

    class IvarRef < Element
      props :name

      def calc_type!(env)
        ivar = env.find_ivar(name)
        return env, ivar.type
      end
    end

    class ConstRef < Element
      props :name

      def calc_type!(env)
        const = env.find_const(name)
        return env, const.type
      end
    end

    class Literal < Element
      props :value

      def calc_type!(env)
        type = case value
               when true, false
                 TyRaw["Bool"]
               when Integer
                 TyRaw["Int"]
               when Integer
                 TyRaw["Float"]
               else
                 raise "unknown value: #{value.inspect}"
               end
        return env, type
      end
    end

    class Lvar
      # kind : :let, :var, :param, :special
      def initialize(name, type, kind)
        @name, @type, @kind = name, type, kind
      end
      attr_reader :name, :type, :kind

      def inspect
        "#<P::Lvar #{kind} #{name.inspect} #{type}>"
      end
    end
  end
end
