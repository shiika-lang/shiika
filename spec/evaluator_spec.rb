require 'spec_helper'

describe "Evaluator" do
  SkObj = Shiika::Evaluator::SkObj

  def run(src)
    ast = Shiika::Parser.new.parse(src)
    program = Shiika::Program.new(ast)
    return Shiika::Evaluator.new.run(program)
  end

  def sk_int(n)
    SkObj.new('Int', [n])
  end

#  it 'class'
#
#  it 'constant'
#
#  it 'instance variable'
#
#  it 'instance generation' do
#    src = ~EOD
#      class A
#        def foo
#          2
#        end
#      end
#      A.new.foo
#    EOD
#    expect(run(src)).to eq(2)
#  end
#
  it 'class method invocation' do
    src = <<~EOD
      class A
        def self.foo -> Int
          1
        end
      end
      A.foo
    EOD
    expect(run(src)).to eq(sk_int(1))
  end

  it 'method invocation' do
    expect(run("1 + 1")).to eq(sk_int(2))
  end

  it 'local variable' do
    expect(run("a = 1; a")).to eq(sk_int(1))
  end

  it 'if' do
    expect(run("if true; 1; else 2; end")).to eq(sk_int(1))
  end

  it 'literal' do
    expect(run("123")).to eq(sk_int(123))
  end
end
