module Shiika
  class Program
    class SkMethod < Element
      props name: String,
            params: [Param],
            ret_type_spec: Type::Base,
            body_stmts: nil #TODO: [Element or Proc]
      
      def init
        @class_typarams = []  # [TypeParameter]
      end

      def n_head_params
        has_varparam? ? varparam_idx : params.length
      end

      def n_tail_params
        has_varparam? ? params.length - (varparam_idx + 1) : 0
      end

      def vararg_range
        (n_head_params..-(n_tail_params+1))
      end

      def varparam
        params.find(&:is_vararg)
      end
      alias has_varparam? varparam

      def varparam_idx
        params.index(&:is_vararg)
      end
      private :varparam_idx

      def calc_type!(env)
        # TODO: raise error if there is more than one varargs
        # TODO: raise error if the type of vararg is not Array
        params.each{|x| x.add_type!(env)}
        ret_type = env.find_type(ret_type_spec)

        if !body_stmts.is_a?(Proc) && body_stmts[0] != :runtime_create_object
          lvars = params.map{|x|
            [x.name, Lvar.new(x.name, x.type, :let)]
          }.to_h
          bodyenv = env.merge(:local_vars, lvars)
          body_stmts.each{|x| bodyenv = x.add_type!(bodyenv)}
          check_body_stmts_type(body_stmts, ret_type)
          check_wrong_return_stmt(body_stmts, ret_type)
        end

        return env, TyMethod.new(name, params.map(&:type),
                                 ret_type)
      end

      def full_name(sk_class_or_type)
        case sk_class_or_type
        when SkMetaClass
          "#{sk_class_or_type.name}.#{self.name}"
        when SkClass
          "#{sk_class_or_type.name}##{self.name}"
        when TyRaw
          "#{sk_class_or_type.name}##{self.name}"
        when TyMeta, TyGenMeta
          "#{sk_class_or_type.base_name}.#{self.name}"
        when TySpe
          "#{sk_class_or_type.name}##{self.name}"
        when TySpeMeta
          "#{sk_class_or_type.spclass_name}.#{self.name}"
        else
          raise sk_class_or_type.inspect
        end
      end

      def inject_type_arguments(type_mapping)
        new_params = params.map{|x|
          param_cls = x.class  # Param or IParam
          param_cls.new(name: x.name,
                        type_spec: x.type_spec.substitute(type_mapping),
                        is_vararg: x.is_vararg).tap{|param|
            param.set_type(param.type_spec)
          }
        }
        SkMethod.new(
          name: name,
          params: new_params,
          ret_type_spec: ret_type_spec.substitute(type_mapping),
          body_stmts: body_stmts
        ).tap{|sk_method|
          sk_method.set_type(TyMethod.new(name,
                                          new_params.map(&:type),
                                          sk_method.ret_type_spec))
        }
      end

      private

      def check_body_stmts_type(body_stmts, ret_type)
        return if ret_type == TyRaw['Void']
        body_type = if body_stmts.empty?
                      TyRaw['Void']
                    else
                      last_stmt = body_stmts.last
                      return if last_stmt.is_a?(Program::Return)
                      last_stmt.type
                    end
        if body_type != ret_type
          raise SkTypeError, "method `#{name}' is declared to return #{ret_type}"+
            " but returns #{body_type}"
        end
      end

      def check_wrong_return_stmt(body_stmts, ret_type)
        body_stmts.each do |x|
          case x
          when Program::Return
            if x.expr_type != ret_type
              raise SkTypeError, "method `#{name}' is declared to return #{ret_type}"+
                " but tried to return #{x.expr_type}"
            end
          when Program::If
            check_wrong_return_stmt(ret_type, x.then_stmts)
            check_wrong_return_stmt(ret_type, x.else_stmts)
          end
        end
      end
    end
  end
end
