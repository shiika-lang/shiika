require 'shiika/program'

module Shiika
  class Evaluator
    def initialize
    end

    # program: Shiika::Program
    def run(program)
      program.add_type!
      @sk_classes = program.sk_classes
      return eval_stmts(program.sk_main.stmts)
    end

    private

    def eval_stmts(stmts)
      last_value = nil
      stmts.each{|x| last_value = eval_stmt(x)}
      return last_value
    end

    def eval_stmt(x)
      eval_expr(x)
    end

    def eval_expr(x)
      case x
      when Program::If
        cond = eval_expr(x.cond_expr)
        if cond != true && cond != false
          raise "if condition did not evaluated to bool: #{cond.inspect}"
        end
        return eval_stmts(cond ? x.then_stmts : x.else_stmts)
      when Program::Literal
        return x.value
      else
        TODO
      end
    end
  end
end
