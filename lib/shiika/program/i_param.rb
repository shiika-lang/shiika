module Shiika
  class Program
    class IParam < Param
      props name: String, type_spec: Type::Base, is_vararg: :boolean
    end
  end
end
