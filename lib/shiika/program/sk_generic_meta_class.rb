module Shiika
  class Program
    class SkGenericMetaClass < SkGenericClass
      more_props typarams: [TypeParameter], sk_generic_class: SkGenericClass

      def init
        @specialized_classes = {}
      end

      def specialized_class(type_arguments, env)
        super(type_arguments, env, SkSpecializedMetaClass)
      end

      def superclass_name
        raise "SkGenericMetaClass does not have a `superclass'"
      end

      def to_type
        TyGenMeta[name, typarams.map(&:name)]
      end
    end
  end
end
