module Shiika
  class Program
    class SkIvar < Element
      props name: String, type_spec: Type::Base

      def calc_type!(env)
        return env, env.find_type(type_spec)
      end
    end
  end
end
