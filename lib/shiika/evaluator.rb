require 'shiika/program'

module Shiika
  class Evaluator
    def initialize
    end

    # program: Shiika::Program
    def run(program)
      program.add_type!
      @sk_classes = program.sk_classes

      last_value = nil
      program.sk_main.stmts.each{|x| last_value = eval_stmt(x)}
      return last_value
    end

    private

    def eval_stmt(stmt)
      eval_expr(stmt)
    end

    def eval_expr(expr)
      case expr
      when Program::Literal
        expr.value
      else
        TODO
      end
    end
  end
end
