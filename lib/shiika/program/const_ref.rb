module Shiika
  class Program
    class ConstRef < Expression
      props name: String

      def calc_type!(env)
        const = env.find_const(name)
        return env, const.type
      end
    end
  end
end
