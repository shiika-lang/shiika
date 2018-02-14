require 'spec_helper'

describe Shiika::Props do
  class Example1
    extend Shiika::Props
    props :a, :b
  end

  class Example2 < Example1
    more_props :c
  end

  class Example3
    extend Shiika::Props
    props :a
    def init
      @init_called = true
    end
  end

  describe ".prop_names" do
    it "should return a list of prop names" do
      expect(Example1.prop_names).to eq([:a, :b])
    end
  end

  describe "#initialize" do
    it "should raise arity error" do
      expect{ Example1.new }.to raise_error(ArgumentError)
      expect{ Example1.new(1,2,3) }.to raise_error(ArgumentError)
    end
  end

  describe "#init" do
    it "should be called on initialization" do
      obj = Example3.new(1)
      expect(obj.instance_variable_get(:@init_called)).to be(true)
    end
  end

  describe ".new_from_hash" do
    it "should create an instance" do
      obj = Example1.new_from_hash(a: 1, b: 2)
      expect(obj.a).to be(1)
      expect(obj.b).to be(2)
    end
  end

  describe "#to_json" do
    it "should return a JSON str" do
      obj = Example1.new(1, 2)
      expect(obj.to_json).to eq(
        {"class" => 'Example1', "a" => 1, "b" => 2}.to_json
      )
    end
  end

  describe "#serialize" do
    it "should return a PORO" do
      obj = Example1.new(1, 2)
      expect(obj.serialize).to eq({class: 'Example1', a: 1, b: 2})
    end
  end

  describe "readers" do
    it 'should de defined' do
      obj = Example1.new(1, 2)
      expect(obj.a).to be(1)
      expect(obj.b).to be(2)
    end
  end

  describe "more_props" do
    it 'should add readers' do
      obj = Example2.new(1, 2, 3)
      expect(obj.a).to be(1)
      expect(obj.b).to be(2)
      expect(obj.c).to be(3)
    end
  end
end
