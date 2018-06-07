require 'spec_helper'

describe "Stdlib" do
  def run(src)
    ast = Shiika::Parser.new.parse(src)
    program = ast.to_program
    return Shiika::Evaluator.new.run(program)
  end

  def sk_int(n)
    Shiika::Evaluator::SkObj.new(Shiika::Type::TyRaw['Int'], {'@rb_val' => n})
  end

  describe 'Array' do
    it '.new' do
      src = <<~EOD
        Array<Int>.new(1, 2, 3).first
        # Also ok :-)
        # Array<Bool>.new(true, false, false).first
      EOD
      expect(run(src)).to eq(sk_int(1))
    end
  end
end
