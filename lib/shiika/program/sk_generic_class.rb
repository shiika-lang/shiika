module Shiika
  class Program
    class SkGenericClass < SkClass
      more_props typarams: [TypeParameter]

      def init
        @specialized_classes = {}
      end
      attr_reader :specialized_classes

      # type_arguments: [Type]
      def specialized_class(type_arguments, env, cls=SkSpecializedClass)
        key = type_arguments.map(&:to_key).join(', ')
        @specialized_classes[key] ||= begin
          sp_cls = cls.new(generic_class: self, type_arguments: type_arguments)
          sp_cls.add_type!(env)
          sp_cls
        end
      end

      def meta_type
        TyGenMeta[name, typarams.map(&:name)]
      end

      def superclass_name
        raise "SkGenericClass does not have a `superclass'"
      end

      private

      def methods_env(env)
        env.merge(:sk_self, self)
           .merge(:typarams, typarams.map{|x| [x.name, x.type]}.to_h)
      end
    end
  end
end
