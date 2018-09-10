module Shiika
  class Program
    class SkClass < Element
      props name: String,
            superclass_template: Type::ConcreteType, # or TyRaw['__noparent__']
            sk_ivars: {String => SkIvar},
            class_methods: {String => SkMethod},
            sk_methods: {String => SkMethod}

      def self.build(hash)
        typarams = hash[:typarams]
        if typarams.any?
          sk_class = SkGenericClass.new(hash)
        else
          sk_class = SkClass.new(hash.except(:typarams))
        end

        meta_name = "Meta:#{sk_class.name}"
        meta_super = if sk_class.name == 'Object'
                       TyRaw['__noparent__']
                     else
                       sk_class.superclass_template.meta_type
                     end
        sk_new = typarams.empty? && make_sk_new(sk_class)

        meta_attrs = {
          name: meta_name,
          superclass_template: meta_super,
          sk_ivars: {},
          class_methods: {},
          sk_methods: (typarams.empty? ? {"new" => sk_new} : {}).merge(sk_class.class_methods)
        }
        if typarams.any?
          meta_class = SkGenericMetaClass.new(meta_attrs.merge(
            typarams: typarams,
            sk_generic_class: sk_class
          ))
        else
          meta_class = SkMetaClass.new(meta_attrs)
        end
        return sk_class, meta_class
      end

      def self.make_sk_new(sk_class)
        sk_new = Program::SkMethod.new(
          name: "new",
          params: sk_class.sk_methods["initialize"].params.map(&:dup),
          ret_type_spec: sk_class.to_type,
          body_stmts: Stdlib.object_new_body_stmts
        )
        return sk_new
      end

      def calc_type!(env)
        menv = methods_env(env)
        @sk_ivars.each_value{|x| x.add_type!(menv)}
        @sk_methods.each_value{|x| x.add_type!(menv)}
        return env, to_type
      end

      def to_type
        TyRaw[name]
      end

      def meta_type
        TyMeta[name]
      end

      def superclass_name
        superclass_template.name
      end

      # Return true if this class is a (maybe indirect) subclass of `other`
      def subclass_of?(other, env)
        if self == other
          false
        elsif self.superclass_template == TyRaw['__noparent__']
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

      # Return SkMethod if this class have a method named `name`.
      # Return nil otherwise
      def find_method(name)
        @sk_methods[name]
      end

      def inspect
        "#<#{self.class.name.sub('Shiika::Program::', '')}:#{name}>"
      end
      alias to_s inspect

      private

      def methods_env(env)
        env.merge(:sk_self, self)
      end
    end
  end
end
