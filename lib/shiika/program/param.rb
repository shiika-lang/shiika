module Shiika
  class Program
    class Param < Element
      props name: String, type_spec: Type::Base, is_vararg: :boolean

      def calc_type!(env)
        return env, env.find_type(type_spec)
      end
    end
  end
end
