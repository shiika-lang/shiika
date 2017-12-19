require 'shiika/program'
require 'shiika/evaluator'
require 'shiika/type'

module Shiika
  # Built-in library
  # Used by Shiika::Evaluator
  # Also used by Shiika::Program (because shiika programs implicitly
  # rely on Object class, etc.)
  module Stdlib
    include Shiika::Type
    SkObj = Shiika::Evaluator::SkObj

    CLASSES = [
      {
        name: "Object",
        parent: :noparent,
        ivars: {},
        class_methods: [
          {
            name: "new",
            ret_type_name: "Object",
            param_type_names: [],
            body: ->(class_obj, *args){
              sk_class_name = class_obj.sk_class_name[/Meta:(.*)/, 1] or
                raise class_obj.inspect
              obj = SkObj.new(sk_class_name, {})
              Evaluator::Call.new(obj, "initialize", args) do |result|
                obj
              end
            }
          }
        ],
        methods: [
          {
            name: "initialize",
            param_type_names: [],
            body: ->(){}
          }
        ]
      },
      {
        name: "Int",
        parent: "Object",
        ivars: {
          '@rb_val' => 'Int'
        },
        class_methods: [],
        methods: [
          {
            name: "initialize",
            param_type_names: [],
            body: ->(){}
          },
          {
            name: "+",
            ret_type_name: "Int",
            param_type_names: ["Int"],
            body: ->(this, other){
              n = this.ivar_values['@rb_val'] + other.ivar_values['@rb_val']
              SkObj.new('Int', {'@rb_val' => n})
            }
          },
          {
            name: "abs",
            ret_type_name: "Int",
            param_type_names: [],
            body: ->(this){
              n = this.ivar_values['@rb_val'].abs
              SkObj.new('Int', {'@rb_val' => n})
            }
          },
          {
            name: "tmp",
            ret_type_name: "Int",
            param_type_names: [],
            body: ->(this){
              Evaluator::Call.new(this, "abs", []) do |result|
                n = result.ivar_values['@rb_val']
                SkObj.new('Int', {'@rb_val' => n})
              end
            }
          }
        ]
      }
    ]

    # Build Program::XX from CLASSES
    def self.sk_classes
      CLASSES.flat_map{|spec|
        sk_methods = spec[:methods].map{|x|
          params = x[:param_type_names].map{|ty_name|
            Program::Param.new("(no name)", ty_name)
          }
          if x[:name] == "initialize"
            sk_method = Program::SkInitializer.new(
              params, x[:body]
            )
          else
            sk_method = Program::SkMethod.new(
              x[:name], params, x[:ret_type_name], x[:body]
            )
          end
          [x[:name], sk_method]
        }.to_h
        sk_class_methods = spec[:class_methods].map{|x|
          params = x[:param_type_names].map{|ty_name|
            Program::Param.new("(no name)", ty_name)
          }
          sk_method = Program::SkMethod.new(
            x[:name], params, x[:ret_type_name], x[:body]
          )
          [x[:name], sk_method]
        }.to_h
        sk_class, meta_class = Program::SkClass.build(
          spec[:name], spec[:parent],
          spec[:ivars], sk_class_methods, sk_methods
        )
        [[sk_class.name, sk_class],
         [meta_class.name, meta_class]]
      }.to_h
    end
  end
end
