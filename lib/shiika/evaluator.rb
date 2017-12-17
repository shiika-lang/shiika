require 'shiika/program'
require 'shiika/evaluator/k_normalization'

module Shiika
  # Evaluates Shiika::Program.
  # Note: this is just a prototype and will be discarded once compilation 
  # into LLVM IR is implemented.
  class Evaluator
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
          cls_obj = SkObj.new("Meta:#{name}", [])
          [name, cls_obj]
        }.to_h
      env = Shiika::Program::Env.new({
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
        if cond.sk_class_name != 'Bool'
          raise "if condition did not evaluated to bool: #{cond.inspect}"
        end
        cond_value = cond.ivar_values[0]
        return eval_stmts(env, cond_value ? x.then_stmts : x.else_stmts)
      when Program::MethodCall
        arg_values = x.args.map do |arg_expr|
          env, value = eval_expr(env, arg_expr)
          value
        end
        env, receiver = eval_expr(env, x.receiver_expr)
        sk_method = receiver.find_method(env, x.method_name)
        if sk_method.body_stmts.is_a?(Proc)  # stdlib
          value = sk_method.body_stmts.call(receiver, *arg_values)
          if value.is_a?(Evaluator::Call)
            invocation = Program::MethodCall.new(value.receiver_obj,
                                                 value.method_name,
                                                 value.arg_objs)
            _, result = eval_stmt(env, invocation)
            return env, value.after.call(result)
          else
            return env, value
          end
        else
          lvars = sk_method.params.zip(arg_values).map{|x, val|
            [x.name, Lvar.new(x.name, env.find_type(x.type_name), :let, val)]
          }.to_h
          bodyenv = env.merge(:local_vars, lvars)
          _, value = eval_stmts(bodyenv, sk_method.body_stmts)
          return env, value 
        end
      when Program::ConstRef
        value = env.find_const(x.name)
        raise TypeError unless value.is_a?(SkObj)
        return env, value
      when Program::Literal
        v = case x.value
            when Float then SkObj.new('Float', [x.value])
            when Integer then SkObj.new('Int', [x.value])
            when true, false then SkObj.new('Bool', [x.value])
            else raise
            end
        return env, v
      when SkObj
        return env, x
      else
        raise "TODO: #{x.class}"
      end
    end

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

    class SkObj
      def initialize(sk_class_name, ivar_values)
        raise TypeError, sk_class_name.inspect unless sk_class_name.is_a?(String)
        raise TypeError unless ivar_values.is_a?(Array)
        @sk_class_name, @ivar_values = sk_class_name, ivar_values
      end
      attr_reader :sk_class_name, :ivar_values

      def ==(other)
        other.is_a?(SkObj) &&
        other.sk_class_name == @sk_class_name and
          other.ivar_values == @ivar_values
      end

      def find_method(env, method_name)
        sk_class = env.find_class(@sk_class_name)
        return sk_class.find_method(method_name)
      end
    end

    class Call
      def initialize(receiver_obj, method_name, arg_objs, &after)
        @receiver_obj, @method_name, @arg_objs, @after = receiver_obj, method_name, arg_objs, after
      end
      attr_reader :receiver_obj, :method_name, :arg_objs, :after
    end
  end
end
