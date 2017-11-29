require 'shiika/program'

module Shiika
  class Evaluator
    def initialize
    end

    # program: Shiika::Program
    def run(program)
      program.add_type!
      @sk_classes = program.sk_classes
      env = Shiika::Program::Env.new({sk_classes: @sk_classes})
      env, last_value = eval_stmts(env, program.sk_main.stmts)
      return last_value
    end

    private

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
        if cond != true && cond != false
          raise "if condition did not evaluated to bool: #{cond.inspect}"
        end
        return eval_stmts(env, cond ? x.then_stmts : x.else_stmts)
      when Program::Literal
        return env, x.value
      else
        raise "TODO: #{x.inspect}"
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
  end
end
