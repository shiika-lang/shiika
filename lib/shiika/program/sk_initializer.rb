module Shiika
  class Program
    class SkInitializer < SkMethod
      def initialize(iparams, body_stmts)
        super(name: "initialize", params: iparams, ret_type_spec: TyRaw["Void"], body_stmts: body_stmts)
      end

      def arity
        @params.length
      end

      # Called from Ast::DefClass#to_program
      # (Note: type is not detected at this time)
      def ivars
        params.grep(IParam).map{|x|
          [x.name, SkIvar.new(name: x.name, type_spec: x.type_spec)]
        }.to_h
      end
    end
  end
end
