require 'shiika/program'

module Shiika
  module Stdlib
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
      }
    ].map{|spec|
      init = Program::SkInitializer.new(
        spec[:name], spec[:initializer][:params], spec[:initializer][:body]
      )
      sk_class = Program::SkClass.new(spec[:name], spec[:parent], init,
                                      spec[:ivars], spec[:methods])
      [spec[:name], sk_class]
    }.to_h
  end
end
