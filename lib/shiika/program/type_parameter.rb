module Shiika
  class Program
    class TypeParameter < Element
      props :name

      def type
        @type ||= Type::TyParam.new(name)
      end
    end
  end
end
