require 'spec_helper'

module Shiika::Type
  describe Shiika::Type do
    context '#conforms?' do
      before do
        @env = Shiika::Program::Env.new({})
      end

      it 'TyRaw vs TyRaw' do
        expect(TyRaw["Int"].conforms?(TyRaw["Int"], @env)).to be_truthy
        expect(TyRaw["Int"].conforms?(TyRaw["String"], @env)).to be_falsy
        #expect(TyRaw["Int"].conforms?(TyRaw["Object"], @env)).to be_truthy
      end

      it 'TyRaw vs TyMeta' do
        expect(TyRaw["Int"].conforms?(TyMeta["Int"], @env)).to be_falsy
        expect(TyMeta["Int"].conforms?(TyRaw["Int"], @env)).to be_falsy
      end

      it 'TyRaw vs TySpe' do
        ty_int_array = TySpe["Array", [TyRaw["Int"]]]
        #expect(ty_int_array.conforms?(TyRaw["Object"], @env)).to be_truthy
        expect(ty_int_array.conforms?(TyRaw["String"], @env)).to be_falsy
      end

      it 'TySpe vs TySpe' do
        ty_int_array = TySpe["Array", [TyRaw["Int"]]]
        expect(ty_int_array.conforms?(ty_int_array, @env)).to be_truthy
        ty_str_array = TySpe["Array", [TyRaw["String"]]]
        expect(ty_int_array.conforms?(ty_str_array, @env)).to be_falsy
      end
    end
  end
end
