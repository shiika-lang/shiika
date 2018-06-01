require 'spec_helper'

describe "Type check" do
  ProgramError = Shiika::Program::ProgramError
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

  context 'variable assignment' do
    it 'reassign to read-only local variable' do
      src = <<~EOD
         a = 1
         a = 2
      EOD
      expect{ type!(src) }.to raise_error(ProgramError)
    end

    it 'reassign to writable local variable (ok)' do
      src = <<~EOD
         var a = 1
         a = 2
      EOD
      expect{ type!(src) }.not_to raise_error
    end

    it 'reassign to writable local variable (ng)' do
      src = <<~EOD
         var a = 1
         a = true
      EOD
      expect{ type!(src) }.to raise_error(SkTypeError)
    end
  end

  context 'generics' do
    it 'number of type arguments' do
      src = <<~EOD
         class A<S, T>
         end
         A<Int>
      EOD
      expect{ type!(src) }.to raise_error(SkTypeError)
    end

    it 'type of initializer' do
      src = <<~EOD
         class A<T>
           def initialize(x: T); end
         end
         A<Int>.new(true)
      EOD
      expect{ type!(src) }.to raise_error(SkTypeError)
    end

    it 'type of instance method' do
      src = <<~EOD
         class A<T>
           def foo(x: T) -> Void; end
         end
         A<Int>.new.foo(true)
      EOD
      expect{ type!(src) }.to raise_error(SkTypeError)
    end

    it 'calling method of instance method parameter' do
      src = <<~EOD
         class A<T>
           def foo(x: T) -> Int
             x.abs
           end
         end
         A<Int>.new.foo(1)
      EOD
      expect{ type!(src) }.to raise_error(SkTypeError)
    end

    it 'type of instance variable (ok)' do
      src = <<~EOD
         class A<T>
           def initialize(@a: T)
             var a = Object.new
             a = @a
           end
         end
         A<Int>.new(1)
      EOD
      expect{ type!(src) }.not_to raise_error
    end

    it 'type of instance variable (ng)' do
      src = <<~EOD
         class A<T>
           def initialize(@a: T)
             @a = 2
           end
         end
         A<Int>.new(1)
      EOD
      expect{ type!(src) }.to raise_error(SkTypeError)
    end
  end
end
