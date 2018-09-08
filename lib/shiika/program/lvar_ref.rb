module Shiika
  class Program
    class LvarRef < Expression
      props name: String

      def calc_type!(env)
        lvar = env.find_lvar(name)
        return env, lvar.type
      end
    end
  end
end
