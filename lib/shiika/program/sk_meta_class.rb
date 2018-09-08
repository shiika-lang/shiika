module Shiika
  class Program
    # Holds class methods of a class
    class SkMetaClass < SkClass
      def to_type
        TyMeta[name]
      end
    end
  end
end
