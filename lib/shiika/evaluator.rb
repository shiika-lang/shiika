require 'shiika/program'
require 'shiika/type'
require 'shiika/evaluator/env'

module Shiika
  # Evaluates Shiika::Program.
  # Note: this is just a prototype and will be discarded once compilation 
  # into LLVM IR is implemented.
  class Evaluator
    include Shiika::Type

    def initialize
    end

    # program: Shiika::Program
    def run(program)
      program.add_type!
      env = initial_env(program)
      env, last_value = eval_stmts(env, program.sk_main.stmts)
      return last_value
    end

    private

    # Create Program::Env which includes initial sk_classes and constants
    def initial_env(program)
      constants = program.sk_classes.keys.reject{|x| x =~ /\AMeta:[^:]/}
        .map{|name|
          cls_obj = SkObj.new(TyMeta[name], {})
          [name, cls_obj]
        }.to_h
      return Shiika::Evaluator::Env.new({
        sk_classes: program.sk_classes,
        constants: constants
      })
    end

    def eval_stmts(env, stmts)
      last_value = nil
      stmts.each{|x| env, last_value = eval_stmt(env, x)}
      return env, last_value
    end

    def eval_stmt(env, x)
      return eval_expr(env, x)
    end

    def eval_expr(env, x)
      case x
      when Program::AssignLvar
        env, value = eval_expr(env, x.expr)
        lvar = Lvar.new(x.varname, x.expr.type, (x.isvar ? :var : :let), value)
        newenv = env.merge(:local_vars, {x.varname => lvar})
        return newenv, value
      when Program::LvarRef
        lvar = env.find_lvar(x.name)
        return env, lvar.value
      when Program::If
        env, cond = eval_expr(env, x.cond_expr)
        if cond.type != TyRaw['Bool']
          raise "if condition did not evaluated to bool: #{cond.inspect}"
        end
        cond_value = cond.ivar_values['@rb_val']
        return eval_stmts(env, cond_value ? x.then_stmts : x.else_stmts)
      when Program::MethodCall
        arg_values = x.args.map do |arg_expr|
          env, value = eval_expr(env, arg_expr)
          value
        end
        env, receiver = eval_expr(env, x.receiver_expr)
        sk_method = receiver.find_method(env, x.method_name)
        return env, invoke_sk_method(env, receiver, sk_method, arg_values)
      when Program::LvarRef
        TODO
      when Program::IvarRef
        value = env.find_ivar_value(x.name)
        raise TypeError unless value.is_a?(SkObj)
        return env, value
      when Program::ConstRef
        value = env.find_const(x.name)
        raise TypeError unless value.is_a?(SkObj)
        return env, value
      when Program::ClassSpecialization
        sp_cls = env.find_class_from_type(x.type)
        return env, SkObj.new(x.type, {}) # TODO: cache?
      when Program::Literal
        v = case x.value
            when Float then SkObj.new(TyRaw['Float'], {'@rb_val' => x.value})
            when Integer then SkObj.new(TyRaw['Int'], {'@rb_val' => x.value})
            when true, false then SkObj.new(TyRaw['Bool'], {'@rb_val' => x.value})
            else raise
            end
        return env, v
      when SkObj
        return env, x
      else
        raise "TODO: #{x.class}"
      end
    end

    def invoke_sk_method(env, receiver, sk_method, raw_arg_values)
      arg_values = process_varargs(env, sk_method, raw_arg_values)
      if sk_method.body_stmts.is_a?(Proc)  # stdlib
        if sk_method.body_stmts.parameters.include?([:keyreq, :raw_arg_values])
          # Some stdlib methods (e.g. Object.new) needs raw_arg_values
          value = sk_method.body_stmts.call(env, receiver, *arg_values,
                                            raw_arg_values: raw_arg_values)
        else
          value = sk_method.body_stmts.call(env, receiver, *arg_values)
        end
        if value.is_a?(Evaluator::Call)
          invocation = Program::MethodCall.new(receiver_expr: value.receiver_obj,
                                               method_name: value.method_name,
                                               args: value.arg_objs)
          _, result = eval_stmt(env, invocation)
          return value.after.call(result)
        else
          return value
        end
      else
        lvars = sk_method.params.zip(arg_values).map{|x, val|
          [x.name, Lvar.new(x.name, x.type, :let, val)]
        }.to_h
        bodyenv = env.merge(:local_vars, lvars).merge(:sk_self, receiver)
        _, value = eval_stmts(bodyenv, sk_method.body_stmts)
        return value 
      end
    end

    def process_varargs(env, sk_method, raw_arg_values)
      if sk_method.has_varparam?
        varargs = raw_arg_values[sk_method.vararg_range]
        vararg_ary = SkObj.new(sk_method.varparam.type, {'@items' => varargs})
        return raw_arg_values.first(sk_method.n_head_params) +
               [vararg_ary] +
               raw_arg_values.last(sk_method.n_tail_params)
      else
        return raw_arg_values
      end
    end

    # Runtime representation of Shiika local variables
    class Lvar
      # kind : :let, :var, :param, :special
      def initialize(name, type, kind, value)
        @name, @type, @kind, @value = name, type, kind, value
      end
      attr_reader :name, :type, :kind, :value

      def inspect
        "#<E::Lvar #{kind} #{name.inspect} #{type} #{value.inspect}>"
      end
    end

    # Runtime representation of Shiika objects
    class SkObj
      def initialize(type, ivar_values)
        raise type.inspect unless type.is_a?(Type::ConcreteType)
        raise unless ivar_values.is_a?(Hash)
        @type, @ivar_values = type, ivar_values
      end
      attr_reader :type, :ivar_values

      def ==(other)
        other.is_a?(SkObj) &&
        other.type == @type and
          other.ivar_values == @ivar_values
      end

      def find_method(env, method_name)
        sk_class = env.find_class_from_type(@type)
        return sk_class.find_method(method_name)
      end
    end

    # A special value returned by "native" methods (i.e. Shiika methods
    # that are implemented in Ruby)
    #
    # If Call is returned by Shiika method, Evaluator will invoke
    # the specified method on `receiver_obj` with `arg_objs` and
    # call `after` with the resulting Shiika object.
    class Call
      def initialize(receiver_obj, method_name, arg_objs, &after)
        @receiver_obj, @method_name, @arg_objs, @after = receiver_obj, method_name, arg_objs, after
      end
      attr_reader :receiver_obj, :method_name, :arg_objs, :after
    end
  end
end
