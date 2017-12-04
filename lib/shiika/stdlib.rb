require 'shiika/program'
require 'shiika/evaluator'
require 'shiika/type'

module Shiika
  module Stdlib
    include Shiika::Type

    CLASSES = [
      {
        name: "Object",
        parent: :noparent,
        initializer: {
          params: [],
          body: ->(){}
        },
        ivars: [],
        methods: []
      },
      {
        name: "Int",
        parent: "Object",
        initializer: {
          params: [],
          body: ->(){}
        },
        ivars: {},
        methods: [
          {
            name: "+",
            ret_type: TyRaw["Int"],
            param_type_names: ["Int"],
            body: ->(this, other){
              n = this.ivar_values[0] + other.ivar_values[0]
              SkObj.new('Int', [n])
            }
          }
        ]
      }
    ].map{|spec|
      init = Program::SkInitializer.new(
        spec[:name], spec[:initializer][:params], spec[:initializer][:body]
      )
      sk_methods = spec[:methods].map{|x|
        params = x[:param_type_names].map{|ty_name|
          Program::Param.new("(no name)", ty_name)
        }
        sk_method = Program::SkMethod.new(
          x[:name], x[:ret_type], params, x[:body]
        )
        [x[:name], sk_method]
      }.to_h
      sk_class = Program::SkClass.new(spec[:name], spec[:parent], init,
                                      spec[:ivars], sk_methods)
      [spec[:name], sk_class]
    }.to_h
  end
end
