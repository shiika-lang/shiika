require 'spec_helper'

describe "Evaluator" do
  def run(src)
    ast = Shiika::Parser.new.parse(src)
    program = Shiika::Program.new(ast)
    return Shiika::Evaluator.new.run(program)
  end

  it 'should evaluate a program' do
    expect(run("123")).to eq(123)
  end
end
