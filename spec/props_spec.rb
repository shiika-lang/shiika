require 'spec_helper'

describe Shiika::Props do
  class Example1
    extend Shiika::Props
    props a: Integer, b: Integer
  end

  class Example2 < Example1
    more_props c: Integer
  end

  class Example3
    extend Shiika::Props
    props a: Integer
    def init
      @init_called = true
    end
  end

  describe ".props_spec" do
    it "should return a hash of prop names and types" do
      expect(Example1.props_spec).to eq({a: Integer, b: Integer})
    end
  end

  describe "#initialize" do
    it "should raise arity error" do
      expect{ Example1.new }.to raise_error(ArgumentError)
      expect{ Example1.new(a: 1, b: 2, x:3) }.to raise_error(ArgumentError)
    end

    describe 'type check' 
  end

  describe "#init" do
    it "should be called on initialization" do
      obj = Example3.new(a: 1)
      expect(obj.instance_variable_get(:@init_called)).to be(true)
    end
  end

  describe "#to_json" do
    it "should return a JSON str" do
      obj = Example1.new(a: 1, b: 2)
      expect(obj.to_json).to eq(
        {"class" => 'Example1', "a" => 1, "b" => 2}.to_json
      )
    end
  end

  describe "#serialize" do
    it "should return a PORO" do
      obj = Example1.new(a: 1, b: 2)
      expect(obj.serialize).to eq({class: 'Example1', a: 1, b: 2})
    end
  end

  describe "readers" do
    it 'should de defined' do
      obj = Example1.new(a: 1, b: 2)
      expect(obj.a).to be(1)
      expect(obj.b).to be(2)
    end
  end

  describe "more_props" do
    it 'should add readers' do
      obj = Example2.new(a: 1, b: 2, c: 3)
      expect(obj.a).to be(1)
      expect(obj.b).to be(2)
      expect(obj.c).to be(3)
    end
  end
end
