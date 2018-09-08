module Shiika
  class Program
    class IvarRef < Expression
      props name: String

      def calc_type!(env)
        ivar = env.find_ivar(name)
        return env, ivar.type
      end
    end
  end
end
