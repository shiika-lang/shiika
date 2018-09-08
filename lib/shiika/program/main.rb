module Shiika
  class Program
    class Main < Element
      props stmts: [Element]

      def calc_type!(env)
        stmts.each{|x| env = x.add_type!(env)}
        return env, (stmts.last ? stmts.last.type : TyRaw["Void"])
      end
    end
  end
end
