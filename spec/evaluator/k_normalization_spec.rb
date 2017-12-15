require 'spec_helper'

describe Shiika::Evaluator::KNormalization do
  def convert(src)
    ast = Shiika::Parser.new.parse(src)
    program = ast.to_program
    result = Shiika::Evaluator::KNormalization.new.convert(program)
    return result.serialize[:sk_main][:stmts]
  end

  it 'no conversion needed' do
    expect(convert("123")).to eq([
      {class: 'Literal', value: 123}
    ])
  end

  EXPANDED_METHOD_CALL = [
    {:class=>"AssignLvar",
     :varname=>"tmp1",
     :expr=>
      {:class=>"MethodCall",
       :receiver_expr=>{:class=>"ConstRef", :name=>"A"},
       :method_name=>"foo",
       :args=>[]},
     :isvar=>"let"},
    {:class=>"MethodCall",
     :receiver_expr=>{:class=>"ConstRef", :name=>"A"},
     :method_name=>"bar",
     :args=>[{:class=>"LvarRef", :name=>"tmp1"}]}
  ]
  it 'nested method invocation' do
    src = <<~EOD
      class A
        def self.foo -> Int; 1; end
        def self.bar(x: Int) -> Int; x; end
      end
      A.bar(A.foo)
    EOD
    expect(convert(src)).to eq(EXPANDED_METHOD_CALL)
  end

  it 'nested method invocation in `if`' do
    src = <<~EOD
      class A
        def self.foo -> Int; 1; end
        def self.bar(x: Int) -> Int; x; end
      end
      if true
        A.bar(A.foo)
      end
    EOD
    expect(convert(src)).to eq([
      {:class=>"If",
       :cond_expr=>{:class=>"Literal", :value=>true},
       :then_stmts=>EXPANDED_METHOD_CALL,
       :else_stmts=>[]}
    ])
  end
end
