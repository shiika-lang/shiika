module Shiika
  class Program
    class ClassSpecialization < Expression
      props class_expr: ConstRef, type_arg_exprs: [ConstRef]

      def calc_type!(env)
        class_expr.add_type!(env)
        type_arg_exprs.each{|x| x.add_type!(env)}

        unless TyGenMeta === class_expr.type
          raise SkTypeError, "not a generic class: #{class_expr.type}"
        end
        base_class_name = class_expr.type.base_name
        type_args = type_arg_exprs.map{|expr|
          raise SkTypeError, "not a class: #{expr.inspect}" unless expr.type.is_a?(TyMeta)
          expr.type.instance_type
        }
        sp_cls, sp_meta = create_specialized_class(env, base_class_name, type_args)
        newenv = env.merge(:sk_classes, {
          sp_cls.name => sp_cls,
          sp_meta.name => sp_meta,
        })
        return newenv, TySpeMeta[base_class_name, type_args]
      end

      private

      # Create specialized class and its metaclass (if they have not been created yet)
      def create_specialized_class(env, base_class_name, type_args)
        gen_cls = env.find_class(base_class_name)
        raise if !(SkGenericClass === gen_cls) &&
                 !(SkGenericMetaClass === gen_cls)
        sp_cls = gen_cls.specialized_class(type_args, env)
        gen_meta = env.find_meta_class(base_class_name)
        sp_meta = gen_meta.specialized_class(type_args, env)
        return sp_cls, sp_meta
      end
    end
  end
end
