require 'spec_helper'

class Shiika::Program
  describe SkClass do
    before do
      @sk_classes = Shiika::Stdlib.sk_classes
      @env = Shiika::Program::Env.new({
        sk_classes: @sk_classes
      })
    end

    describe "#subclass_of?" do
      it "direct subclass" do
        expect(@sk_classes['Int'].subclass_of?(@sk_classes['Object'], @env)).to be_truthy
      end

      it "indirect subclass" do
        my_int = SkClass.new(
          name: "MyInt",
          superclass_template: Shiika::Type::TyRaw["Int"],
          sk_ivars: {},
          class_methods: {},
          sk_methods: {}
        )
        expect(my_int.subclass_of?(@sk_classes['Int'], @env)).to be_truthy
      end

      it "specialized class" do
        bool_ary = @sk_classes['Array'].specialized_class([Shiika::Type::TyRaw['Bool']], @env)
        expect(bool_ary.subclass_of?(@sk_classes['Object'], @env))
      end

      it "metaclass" do
        expect(@sk_classes['Meta:Int'].subclass_of?(@sk_classes['Meta:Object'], @env)).to be_truthy
      end

      it "specialized metaclass" do
        meta_bool_ary = @sk_classes['Meta:Array'].specialized_class([Shiika::Type::TyRaw['Bool']], @env)
        expect(meta_bool_ary.subclass_of?(@sk_classes['Meta:Object'], @env)).to be_truthy
      end
    end
  end
end
