module Shiika
  class Program
    class ArrayExpr < Expression
      props exprs: [Expression]

      def calc_type!(env)
        exprs.each{|x| env = x.add_type!(env)}
        elem_type = exprs.first.type
        exprs.each do |x|
          if x.type != elem_type
            raise SkTypeError, 'Currently all elements of an array must have'+
              ' the same type'
          end
        end
        # Make sure Meta:Array<T> is created (to call .new on it)
        sp_cls = env.find_class('Meta:Array').specialized_class([elem_type], env)
        sp_cls.find_method('new')
        return env, TySpe['Array', [elem_type]]
      end
    end
  end
end
