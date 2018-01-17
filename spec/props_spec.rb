require 'spec_helper'

describe Shiika::Props do
  class Example1
    extend Shiika::Props
    props :a, :b
  end

  class Example2 < Example1
    more_props :c
  end

  # TODO:
  #   .prop_names
  #   #initialize
  #   attr_accessor
  #   init
  #   to_json
  #   serialize

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
