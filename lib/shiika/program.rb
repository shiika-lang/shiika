require 'active_support/core_ext/hash/except'
require 'shiika/props'
require 'shiika/type'
require 'shiika/program/env'
require 'shiika/program/element'
require 'shiika/program/sk_ivar'
require 'shiika/program/param'
require 'shiika/program/i_param'
require 'shiika/program/type_parameter'
require 'shiika/program/sk_method'
require 'shiika/program/sk_initializer'
require 'shiika/program/sk_class'
require 'shiika/program/sk_generic_class'
require 'shiika/program/sk_specialized_class'
require 'shiika/program/sk_meta_class'
require 'shiika/program/sk_generic_meta_class'
require 'shiika/program/sk_specialized_meta_class'
require 'shiika/program/sk_const'
require 'shiika/program/main'
require 'shiika/program/expression'
require 'shiika/program/return'
require 'shiika/program/if'
require 'shiika/program/method_call'
require 'shiika/program/assignment_expr'
require 'shiika/program/assign_lvar'
require 'shiika/program/assign_ivar'
require 'shiika/program/assign_const'
require 'shiika/program/lvar_ref'
require 'shiika/program/ivar_ref'
require 'shiika/program/const_ref'
require 'shiika/program/class_specialization'
require 'shiika/program/array_expr'
require 'shiika/program/literal'
require 'shiika/program/lvar'

module Shiika
  # Represents a Shiika program
  class Program
    # Shiika-level type error
    class SkTypeError < StandardError; end
    # Shiika-level name error
    class SkNameError < StandardError; end
    # Other Shiika-level errors
    class SkProgramError < StandardError; end

    def initialize(sk_classes, sk_main)
      @sk_classes, @sk_main = sk_classes, sk_main
    end
    attr_reader :sk_classes, :sk_main

    def add_type!
      constants = @sk_classes.reject{|name, sk_class|
        name.start_with?('Meta:')
      }.map{|name, sk_class|
        const = SkConst.new(name: name)
        const.instance_variable_set(:@type, sk_class.meta_type)
        [name, const]
      }.to_h
      env = Shiika::Program::Env.new({
        sk_classes: @sk_classes,
        constants: constants,
      })
      @sk_classes.each_value{|x| x.add_type!(env)}
      @sk_main.add_type!(env)

      # Add specific classes to @sk_classes (for Shiika::Evaluator)
      specific_classes = @sk_classes.values.grep(SkGenericClass).map{|x|
        x.specialized_classes.values.map{|sp_cls|
          [sp_cls.name, sp_cls]
        }.to_h
      }.inject({}, :merge)
      @sk_classes.merge!(specific_classes)
    end

    # Return a PORO that represents this program (for unit tests)
    def serialize
      {
        class: 'Program',
        sk_classes: @sk_classes.transform_values(&:serialize),
        sk_main: @sk_main.serialize,
      }
    end
  end
end
