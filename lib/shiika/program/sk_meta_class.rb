module Shiika
  class Program
    # Holds class methods of a class
    class SkMetaClass < SkClass
      def to_type
        TyMeta[name.sub(/^Meta:/, '')]
      end
    end
  end
end
