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
          if !@data[:sk_classes].key?(type.name) && type.name != "Void" &&
             !@data[:typarams].key?(type.name)
            raise ProgramError, "unknown type: #{type.inspect}"
          end
        when TySpe
          TODO
        else
          raise "bug: #{type.inspect}"
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

      def find_lvar(name)
        unless (lvar = @data[:local_vars][name])
          raise SkNameError, "undefined local variable: #{name}"
        end
        return lvar
      end

      # Find Program::SkIvar
      def find_ivar(name)
        unless (sk_self = @data[:sk_self])
          raise ProgramError, "ivar reference out of a class: #{name}" 
        end
        unless (ivar = sk_self.sk_ivars[name])
          raise SkNameError, "class #{sk_self.name} does not have "+
            "an instance variable #{name}"
        end
        return ivar
      end

      def find_method(receiver_type, name)
        case receiver_type
        when TyRaw, TyMeta
          sk_class = @data[:sk_classes].fetch(receiver_type.name)
          return sk_class.find_method(name)
        when TySpe, TySpeMeta
          sk_class = @data[:sk_classes].fetch(receiver_type.base_name)
          sp_class = sk_class.specialized_class(receiver_type.type_args)
          return sp_class.find_method(name)
        else
          raise
        end
      end
    end
  end
end
