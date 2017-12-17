require 'shiika/props'
require 'shiika/type'
require 'shiika/program/env'

module Shiika
  # Represents a Shiika program
  class Program
    # Shiika-level type error
    class SkTypeError < StandardError; end
    # Shiika-level name error
    class SkNameError < StandardError; end
    # Other Shiika-level errors
    class ProgramError < StandardError; end

    def initialize(sk_classes, sk_main)
      @sk_classes, @sk_main = sk_classes, sk_main
    end
    attr_reader :sk_classes, :sk_main

    def add_type!
      # Do nothing if already typed
      return if @sk_main.type

      constants = @sk_classes.keys.map{|name|
        const = SkConst.new(name)
        const.instance_variable_set(:@type, Type::TyRaw["Meta:#{name}"])
        [name, const]
      }.to_h
      env = Shiika::Program::Env.new({
        sk_classes: @sk_classes,
        constants: constants,
      })
      @sk_classes.each_value{|x| x.add_type!(env)}
      @sk_main.add_type!(env)
    end

    def serialize
      {
        class: 'Program',
        sk_classes: @sk_classes.transform_values(&:serialize),
        sk_main: @sk_main.serialize,
      }
    end

    class Element
      include Type
      extend Props

      def add_type!(env)
        newenv, @type = calc_type!(env)
        raise TypeError unless newenv.is_a?(Shiika::Program::Env)
        return newenv
      end
      attr_reader :type

      def calc_type!(env)
        raise "override me"
      end

      def inspect
        cls_name = self.class.name.split('::').last
        ivars = self.instance_variables.map{|name|
          val = self.instance_variable_get(name)
          "#{name}=#{val.inspect}"
        }
        "#<P::#{cls_name}##{self.object_id} #{ivars.join ' '}>"
      end

      #
      # Debug print for add_type!
      #
      module DebugAddType
        @@lv = 0
        def add_type!(env, *rest)
          raise self.inspect if @type
          print " "*@@lv; p self
          @@lv += 2
          env = super(env, *rest)
          @@lv -= 2
          #print " "*@@lv; p self
          env
        end
      end
      def self.inherited(cls)
        #cls.prepend DebugAddType
      end
    end

    class SkClass < Element
      props :name, # String
            :parent_name, # String or :noparent
            :sk_initializer, # SkInitializer
            :sk_ivars,   # {String => SkIvar},
            :class_methods,  # {String => SkClassMethod}
            :sk_methods  # {String => SkMethod}

      def self.build(*args)
        sk_class = SkClass.new(*args)
        meta_name = "Meta:#{sk_class.name}"
        meta_parent = if sk_class.parent_name == :noparent 
                        :noparent 
                      else
                        "Meta:#{sk_class.parent_name}"
                      end
        sk_new = make_sk_new(sk_class)
        meta_class = SkMetaClass.new(
          meta_name,
          meta_parent,
          SkInitializer.new(meta_name, [], ->(){}),
          {},
          {},
          {"new" => sk_new}.merge(sk_class.class_methods),
        )
        meta_class.sk_class = sk_class
        return sk_class, meta_class
      end

      @@object_new = nil
      def self.make_sk_new(sk_class)
        if sk_class.name == "Object"
          @@object_new = sk_class.class_methods["new"]
          return @@object_new
        end
        raise "class Object must be built first" unless @@object_new
        
        sk_new = Program::SkMethod.new(
          "new",
          sk_class.sk_initializer.params,
          sk_class.name,
          @@object_new.body_stmts
        )
        return sk_new
      end

      def calc_type!(env)
        @sk_initializer.add_type!(env)
        @sk_methods.each_value{|x| x.add_type!(env)}
        return env, TyRaw[name]
      end

      def find_method(name)
        return @sk_initializer if name == "initialize"
        return @sk_methods.fetch(name)
      end
    end

    # Holds class methods of a class
    class SkMetaClass < SkClass
      attr_accessor :sk_class
    end

    class SkIvar < Element
      props :name, # String
            :type # Shiika::Type
    end

    class SkMethod < Element
      props :name,
            :params,
            :ret_type_name,
            :body_stmts

      def arity
        @params.length
      end

      def calc_type!(env)
        param_tys = params.map{|param|
          env.find_type(param.type_name)
        }

        if !body_stmts.is_a?(Proc) && body_stmts[0] != :runtime_create_object
          lvars = params.map{|x|
            [x.name, Lvar.new(x.name, env.find_type(x.type_name), :let)]
          }.to_h
          bodyenv = env.merge(:local_vars, lvars)
          body_stmts.each{|x| bodyenv = x.add_type!(bodyenv)}
        end

        return env, TyMethod.new(name, param_tys, env.find_type(ret_type_name))
      end
    end

    class SkInitializer < SkMethod
      def initialize(name, iparams, body_stmts)
        super(name, iparams, "Void", body_stmts)
      end

      def arity
        @params.length
      end

      def ivars
        params.grep(IParam).map{|x|
          SkIvar.new(x.name, x.type)
        }
      end

      def calc_type!(env)
        unless body_stmts.is_a?(Proc)
          body_stmts.each{|x| env = x.add_type!(env)}
        end
        param_tys = params.map(&:type)
        return env, TyMethod.new("initialize", param_tys, TyRaw["Void"])
      end
    end

    # TODO: Should we remove this? (This might be confusing because a class
    # method is a method of the meta-class)
    class SkClassMethod < SkMethod; end

    class SkConst < Element
      props :name
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
        args.each{|x| env = x.add_type!(env)}
        env = receiver_expr.add_type!(env)
        sk_method = env.find_method(receiver_expr.type, method_name)
        return env, sk_method.type.ret_type
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
