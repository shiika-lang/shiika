require 'shiika/type'

module Shiika
  class Program
    # Environment
    # Used both by Shiika::Program and Shiika::Evaluator
    class Env
      include Type

      # data: {
      #     sk_classes: {String => Program::SkClass},
      #     typarams: {String => Type::TyParam},
      #     local_vars: {String => Program::Lvar},
      #     sk_self: Program::SkClass,
      #   }
      def initialize(data)
        @data = {
          sk_classes: {},
          typarams: {},
          local_vars: {},
          constants: {},
          sk_self: :notset,
        }.merge(data)
      end

      # Create new instance of `Env` by merging `hash` into the one at the key
      def merge(key, x)
        if x.is_a?(Hash)
          self.class.new(@data.merge({key => @data[key].merge(x)}))
        else
          self.class.new(@data.merge({key => x}))
        end
      end

      def check_type_exists(type)
        case type
        when TyRaw
          if type.name == "Void" ||
             @data[:sk_classes].key?(type.name) ||
             @data[:typarams].key?(type.name)
            # OK
          else
            raise SkProgramError, "unknown type: #{type.inspect}"
          end
        when TySpe
          check_type_exists(type.base_type)
          type.type_args.each{|t| check_type_exists(t)}
        else
          raise "bug: #{type.inspect}"
        end
      end
      private :check_type_exists

      # eg. return TyParam["T"] if T is a typaram
      #     return TyRaw["T"] if there is class T
      #     otherwise, raise error 
      def find_type(type_spec)
        if type_spec.is_a?(TyRaw) && (typaram = @data[:typarams][type_spec.name])
          typaram
        else
          check_type_exists(type_spec)
          type_spec
        end
      end

      def find_class(name)
        return @data[:sk_classes].fetch(name)
      end

      def find_meta_class(base_name)
        return @data[:sk_classes].fetch("Meta:#{base_name}")
      end

      def find_const(name)
        return @data[:constants].fetch(name)
      end

      def find_lvar(name, allow_missing: false)
        lvar = @data[:local_vars][name]
        if !allow_missing && !lvar
          raise SkNameError, "undefined local variable: #{name}"
        end
        return lvar
      end

      # Find Program::SkIvar
      def find_ivar(name)
        unless (sk_self = @data[:sk_self])
          raise SkProgramError, "ivar reference out of a class: #{name}" 
        end
        unless (ivar = sk_self.sk_ivars[name])
          raise SkNameError, "class #{sk_self.name} does not have "+
            "an instance variable #{name}"
        end
        return ivar
      end

      def find_method(receiver_type, name, origin: receiver_type)
        sk_method = 
          case receiver_type
          when TyRaw, TyMeta
            sk_class = @data[:sk_classes].fetch(receiver_type.name)
            sk_class.find_method(name)
          when TySpe, TySpeMeta
            gen_cls = @data[:sk_classes].fetch(receiver_type.base_name)
            sp_cls = gen_cls.specialized_class(receiver_type.type_args, self)
            sp_cls.find_method(name)
          when TyGenMeta  # Class method of a generic class
            gen_meta_cls = @data[:sk_classes].fetch(receiver_type.base_meta_name)
            gen_meta_cls.find_method(name)
          when TyParam
            typaram = receiver_type
            find_method(typaram.upper_bound, name)
          else
            raise "env.find_method(#{receiver_type}) is not implemented yet"
          end
        if sk_method
          return sk_method
        else
          if (super_type = supertype_of(receiver_type))
            return find_method(super_type, name, origin: origin)
          else
            raise SkTypeError, "method `#{name}' not found on #{origin}"
          end
        end
      end

      def supertype_of(type)
        sk_class = find_class(type.name)
        return nil if sk_class.superclass_name == '__noparent__'
        super_class = find_class(sk_class.superclass_name)
        return super_class.type
      end

      def sk_self
        @data[:sk_self]
      end

      # Return true if type1 conforms to type2 (eg. TyRaw['Int'] conforms to TyRaw['Object'])
      def conforms_to?(type1, type2)
        get_cls = ->(type){
          if type.is_a?(TyParam)
            find_class('Object')
          else
            find_class(type.name)
          end
        }
        cls1, cls2 = get_cls[type1], get_cls[type2]
        return cls1 == cls2 || cls1.subclass_of?(cls2, self)
      end
    end
  end
end
