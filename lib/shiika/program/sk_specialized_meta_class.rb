module Shiika
  class Program
    class SkSpecializedMetaClass < SkSpecializedClass
      alias sk_generic_meta_class generic_class

      def init
        super
        sk_generic_class = sk_generic_meta_class.sk_generic_class
        @name = "Meta:#{sk_generic_class.name}<" + type_arguments.map(&:name).join(', ') + ">"
        @sk_new = Program::SkMethod.new(
          name: "new",
          params: sk_generic_class.sk_methods["initialize"].params.map(&:dup),
          ret_type_spec: TySpe[sk_generic_class.name, type_arguments],
          body_stmts: Stdlib.object_new_body_stmts,
        )
      end

      def calc_type!(env)
        typarams = sk_generic_meta_class.typarams.zip(type_arguments).map{|tparam, targ|
          [tparam.name, targ]
        }.to_h
        menv = env.merge(:sk_self, self)
                  .merge(:typarams, typarams)
        @sk_new.add_type!(menv)
        return env, TySpeMeta[sk_generic_meta_class.sk_generic_class.name, type_arguments]
      end

      def find_method(name)
        if name == "new"
          # Special treatment for `new` becuase SkGenericMetaClass does not have `new`
          return @sk_new.inject_type_arguments(type_mapping)
        else
          super
        end
      end
    end
  end
end
