module Shiika
  class Program
    # Base class of each program element.
    class Element
      include Type
      extend Props

      def add_type!(env)
        newenv, @type = calc_type!(env)
        raise TypeError unless newenv.is_a?(Shiika::Program::Env)
        return newenv
      end

      def set_type(ty)
        @type = ty
      end

      def type
        @type or raise "type not yet calculated on #{self.inspect}"
      end

      def calc_type!(env)
        raise "override me (#{self.class})"
      end

      def inspect
        cls_name = self.class.name.split('::').last
        ivars = self.instance_variables.map{|name|
          val = self.instance_variable_get(name)
          "#{name}=#{val.inspect}"
        }
        ivars_desc = ivars.join(' ')
        ivars_desc = ivars_desc[0, 90] + "..." if ivars_desc.length > 100
        "#<P::#{cls_name}##{self.object_id} #{ivars_desc}>"
      end

      #
      # Debug print for add_type!
      #
      module DebugAddType
        @@lv = 0
        def add_type!(env, *rest)
          raise "already has type: #{self.inspect}" if @type
          print " "*@@lv; p self
          @@lv += 2
          env = super(env, *rest)
          @@lv -= 2
          print " "*@@lv; puts "=> #{self.type.inspect}"
          env
        end
      end
      def self.inherited(cls)
        cls.prepend DebugAddType if ENV['DEBUG']
      end
    end
  end
end
