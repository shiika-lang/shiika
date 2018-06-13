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

    def self.object_new_body_stmts
      CLASSES.first[:class_methods].first[:body]
    end

    CLASSES = [
      {
        name: "Object",
        parent: '__noparent__',
        typarams: [],
        ivars: {},
        class_methods: [
          {
            name: "new",
            ret_type_spec: TyRaw["Object"],
            params: [],
            # args: [SkObj] (varargs are packed with Shiika Array)
            # raw_arg_values: [SkObj] (varargs are not packed)
            body: ->(env, class_obj, *args, raw_arg_values:){
              instance_type = class_obj.type.instance_type
              sk_class = env.find_class_from_type(instance_type)
              sk_initializer = sk_class.find_method("initialize")
              ivar_values = sk_initializer.params.zip(args).select{|param, arg|
                param.is_a?(Program::IParam)
              }.map{|param, arg|
                [param.name, arg]
              }.to_h
              obj = SkObj.new(instance_type, ivar_values)
              Evaluator::Call.new(obj, "initialize", raw_arg_values) do |result|
                obj
              end
            }
          }
        ],
        methods: [
          {
            name: "initialize",
            params: [],
            body: ->(){}
          }
        ]
      },
      {
        name: "Bool",
        parent: "Object",
        typarams: [],
        ivars: {},
        class_methods: [],
        methods: [
          {
            name: "initialize",
            params: [],
            body: ->(){}
          },
        ]
      },
      {
        name: "Int",
        parent: "Object",
        typarams: [],
        ivars: {
          '@rb_val' => TyRaw['Int']
        },
        class_methods: [],
        methods: [
          {
            name: "initialize",
            params: [],
            body: ->(){}
          },
          {
            name: "+",
            ret_type_spec: TyRaw["Int"],
            params: [
              {name: "other", type_spec: TyRaw["Int"], is_vararg: false}
            ],
            body: ->(env, this, other){
              n = this.ivar_values['@rb_val'] + other.ivar_values['@rb_val']
              SkObj.new(TyRaw['Int'], {'@rb_val' => n})
            }
          },
          {
            name: "abs",
            ret_type_spec: TyRaw["Int"],
            params: [],
            body: ->(env, this){
              n = this.ivar_values['@rb_val'].abs
              SkObj.new(TyRaw['Int'], {'@rb_val' => n})
            }
          },
          {
            name: "tmp",
            ret_type_spec: TyRaw["Int"],
            params: [],
            body: ->(env, this){
              Evaluator::Call.new(this, "abs", []) do |result|
                n = result.ivar_values['@rb_val']
                SkObj.new(TyRaw['Int'], {'@rb_val' => n})
              end
            }
          }
        ]
      },
      {
        name: 'Array',
        parent: 'Object',
        typarams: ['ELEM'],
        ivars: {
          '@items' => TyRaw['Void']
        },
        class_methods: [],
        methods: [
          {
            name: "initialize",
            params: [
              {name: "items", type_spec: TySpe['Array', [TyRaw['ELEM']]], is_vararg: true}
            ],
            body: ->(env, this, sk_ary){
              this.ivar_values['@items'] = sk_ary.ivar_values['@items']
            }
          },
          {
            name: 'first',
            ret_type_spec: TyRaw["ELEM"],
            params: [],
            body: ->(env, this){
              this.ivar_values['@items'].first
            }
          }
        ]
      }
    ]

    # Build Program::XX from CLASSES
    def self.sk_classes
      CLASSES.flat_map{|spec|
        sk_methods = spec[:methods].map{|x|
          params = x[:params].map{|param_spec|
            Program::Param.new(param_spec)
          }
          if x[:name] == "initialize"
            sk_method = Program::SkInitializer.new(
              params, x[:body]
            )
          else
            sk_method = Program::SkMethod.new(
              name: x[:name], params: params, ret_type_spec: x[:ret_type_spec], body_stmts: x[:body]
            )
          end
          [x[:name], sk_method]
        }.to_h
        sk_class_methods = spec[:class_methods].map{|x|
          params = x[:params].map{|param_spec|
            Program::Param.new(param_spec)
          }
          sk_method = Program::SkMethod.new(
            name: x[:name], params: params, ret_type_spec: x[:ret_type_spec], body_stmts: x[:body]
          )
          [x[:name], sk_method]
        }.to_h
        sk_ivars = spec[:ivars].map{|name, type|
          [name, Program::SkIvar.new(name: name, type_spec: type)]
        }.to_h
        sk_class, meta_class = Program::SkClass.build(
          name: spec[:name], parent_name: spec[:parent],
          sk_ivars: sk_ivars, class_methods: sk_class_methods, sk_methods: sk_methods,
          typarams: spec[:typarams].map{|x|
            Program::TypeParameter.new(name: x)
          }
        )
        [[sk_class.name, sk_class],
         [meta_class.name, meta_class]]
      }.to_h
    end
  end
end
