module Shiika
  class Program
    class Env
      # data: {
      #     sk_classes: {String => Program::SkClass},
      #     local_vars: {String => Program::Lvar},
      #     sk_self: Program::SkClass,
      #   }
      def initialize(data)
        @data = {
          sk_classes: {},
          local_vars: {},
          sk_self: :notset,
        }.merge(data)
      end

      def inspect
        "#<Env:#{@data.inspect}>"
      end

      def merge(key, hash)
        newdata = @data.merge({key => @data[key].merge(hash)})
        Env.new(newdata)
      end

      def find_type(name)
        raise ProgramError, "unknown type: #{name}" unless @data[:sk_classes].key?(name)
        return TyRaw[name]
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
    end
  end
end
