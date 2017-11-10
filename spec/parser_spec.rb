require 'spec_helper'

describe "Parser" do
  def parse(src)
    Shiika::Parser.new.parse(src)
  end

  context 'program' do
    it 'definitions + top_statements' do
      expect {
        parse("class A; end; 1+1")
      }.not_to raise_error
    end

    it 'definitions only' do
      expect {
        parse("class A; end")
      }.not_to raise_error
    end

    it 'top_statements only' do
      expect {
        parse("1+1")
      }.not_to raise_error
    end

    it 'nothing' do
      expect {
        parse("")
      }.not_to raise_error
    end

    it 'starts with comment line' do
      expect {
        parse("# comment.\n1 + 1")
      }.not_to raise_error
    end
  end

  context 'method definition' do
    it 'allow names with `!`' do
      expect {
        parse("class A; def foo! -> Void; end; end; A.new.foo!")
      }.not_to raise_error
    end

    it 'allow names with `!`' do
      expect {
        parse("class A; def foo? -> Void; end; end; A.new.foo?")
      }.not_to raise_error
    end
  end

  it "should allow trailing space on a line" do
    expect {
      parse("class A \nend\n1+1")
    }.not_to raise_error
  end
end
