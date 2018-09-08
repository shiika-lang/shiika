module Shiika
  class Program
    class MethodCall < Expression
      props method_name: String,
            receiver_expr: nil, #TODO Expression or Evaluator::SkObj
            args: nil #TODO [Expression or Evaluator::SkObj]

      def calc_type!(env)
        args.each{|x| env = x.add_type!(env)}
        env = receiver_expr.add_type!(env)
        sk_method = env.find_method(receiver_expr.type, method_name)
        check_arg_types(sk_method, env)
        return env, sk_method.type.ret_type
      end

      private

      def check_arg_types(sk_method, env)
        n_args = args.length
        params = sk_method.params
        varparam = params.find(&:is_vararg)

        # Assert that sufficient number of args are given
        least_arity = varparam ? params.length - 1 : params.length
        if n_args < least_arity
          raise SkTypeError, "method #{sk_method.name} takes " +
            "#{'at least ' if varparam}#{least_arity} parameters but got #{n_args}"
        end

        check_nonvar_arg_types(sk_method, env)

        if varparam
          # Check type of varargs
          elem_type = varparam.type.type_args.first
          varargs = args[sk_method.vararg_range]
          varargs.each do |arg|
            if arg.type != elem_type
              raise SkTypeError, "variable-length parameter #{varparam.name} of `#{sk_method.full_name(receiver_expr.type)}` is #{varparam.type} but got #{arg.type} for its element"
            end
          end
          # Make sure Meta:Array<T> is created (to call .new on it)
          sp_cls = env.find_class('Meta:Array').specialized_class([elem_type], env)
          sp_cls.find_method('new')
        end
      end

      def check_nonvar_arg_types(sk_method, env)
        params = sk_method.params
        n_head = sk_method.n_head_params
        n_tail = sk_method.n_tail_params

        matches = params.first(n_head).zip(args.first(n_head)) +
                  params.last(n_tail).zip(args.last(n_tail))
        matches.each do |param, arg|
          if !env.conforms_to?(arg.type, param.type)
            raise SkTypeError, "parameter `#{param.name}' of `#{sk_method.full_name(receiver_expr.type)}' is #{param.type} but got #{arg.type}"
          end
        end
      end
    end
  end
end
