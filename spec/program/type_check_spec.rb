require 'spec_helper'

describe "Type check" do
  SkTypeError = Shiika::Program::SkTypeError

  def type!(src)
    ast = Shiika::Parser.new.parse(src)
    prog = ast.to_program
    prog.add_type!
  end

  context 'method call' do
    it 'arity' do
      src = <<~EOD
         class A
           def self.foo(x: Int, y: Int) -> Void
           end
         end
         A.foo(1)
      EOD
      expect{ type!(src) }.to raise_error(SkTypeError)
    end

    it 'argument type' do
      src = <<~EOD
         class A
           def self.foo(x: Int) -> Void
           end
         end
         A.foo(true)
      EOD
      expect{ type!(src) }.to raise_error(SkTypeError)
    end
  end

  context 'variable assignment'
end
