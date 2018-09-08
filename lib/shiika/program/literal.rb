module Shiika
  class Program
    class Literal < Expression
      props value: Object  # A Ruby object that describes the value

      def calc_type!(env)
        type = case value
               when true, false
                 TyRaw["Bool"]
               when Integer
                 TyRaw["Int"]
               when Integer
                 TyRaw["Float"]
               else
                 raise "unknown value: #{value.inspect}"
               end
        return env, type
      end
    end
  end
end
