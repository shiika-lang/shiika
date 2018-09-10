module Shiika
  class Program
    class SkSpecializedClass < Element
      props generic_class: SkGenericClass,
            type_arguments: [Type::ConcreteType]
      alias sk_generic_class generic_class

      def init
        n_typarams, n_tyargs = generic_class.typarams.length, type_arguments.length
        if n_typarams != n_tyargs
          raise SkTypeError, "#{generic_class} takes #{n_typarams} type parameters "+
            "but got #{n_tyargs}"
        end
        @name = "#{sk_generic_class.name}<" + type_arguments.map(&:name).join(', ') + ">"
        @methods = {}  # String => SkMethod
      end
      attr_reader :name
      
      def calc_type!(env)
        return env, TySpe[sk_generic_class.name, type_arguments]
      end

      # Return true if this class is a (maybe indirect) subclass of `other`
      def subclass_of?(other, env)
        if self == other
          false
        else
          parent = env.find_class(self.superclass_name)
          if parent == other
            true
          else
            parent.subclass_of?(other, env)
          end
        end
      end

      # Lazy method creation (create when first called)
      # Return SkMethod if this class have a method named `name`.
      # Return nil otherwise
      def find_method(name)
        if @methods.key?(name)
          @methods[name]
        elsif (gen_method = sk_generic_class.sk_methods[name])
          ret = gen_method.inject_type_arguments(type_mapping)
          @methods[name] = ret
          ret
        else
          nil
        end
      end

      # eg. `"A<Int>"` for `B<Int>`, where `class B<T> extends A<T>`
      def superclass_name
        generic_class.superclass_template.substitute(type_mapping).name
      end

      private

      def type_mapping
        generic_class.typarams.zip(type_arguments).map{|typaram, tyarg|
          [typaram.name, tyarg]
        }.to_h
      end
    end
  end
end
