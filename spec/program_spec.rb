require 'spec_helper'

describe "Program" do
  def parse(src)
    ast = Shiika::Parser.new.parse(src)
    return Shiika::Program.new(ast)
  end

  it 'can be created' do
    prog = parse("class A; end; 1+1")
    expect(prog.sk_classes.keys.sort).to eq(["A", "Object"])

    sk_a = prog.sk_classes["A"]
    expect(sk_a.serialize).to eq({
      class: "SkClass",
      name: "A",
      parent_name: "Object",
      sk_initializer: {
        class: "SkInitializer",
        name: "A",
        iparams: [],
        body_stmts: [],
      },
      sk_ivars: [],
      sk_methods: [],
    })

    expect(prog.sk_main.serialize).to eq({
      class: "Main",
      stmts: [{
        class: "BinExpr",
        op: "+",
        left_expr: {
          class: "Literal",
          value: 1,
        },
        right_expr: {
          class: "Literal",
          value: 1,
        },
      }]
    })
  end
end
