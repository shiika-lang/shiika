require 'spec_helper'

module Shiika::Type
  describe Shiika::Type do
    context '#conforms_to?' do
      before do
        @env = Shiika::Program::Env.new({})
      end

      it 'TyRaw vs TyRaw' do
        expect(TyRaw["Int"].conforms_to?(TyRaw["Int"], @env)).to be_truthy
        expect(TyRaw["Int"].conforms_to?(TyRaw["String"], @env)).to be_falsy
        expect(TyRaw["Int"].conforms_to?(TyRaw["Object"], @env)).to be_truthy
      end

      it 'TyRaw vs TyMeta' do
        expect(TyRaw["Int"].conforms_to?(TyMeta["Int"], @env)).to be_falsy
        expect(TyMeta["Int"].conforms_to?(TyRaw["Int"], @env)).to be_falsy
      end

      it 'TyRaw vs TySpe' do
        ty_int_array = TySpe["Array", [TyRaw["Int"]]]
        expect(ty_int_array.conforms_to?(TyRaw["Object"], @env)).to be_truthy
        expect(ty_int_array.conforms_to?(TyRaw["String"], @env)).to be_falsy
      end

      it 'TySpe vs TySpe' do
        ty_int_array = TySpe["Array", [TyRaw["Int"]]]
        expect(ty_int_array.conforms_to?(ty_int_array, @env)).to be_truthy
        ty_str_array = TySpe["Array", [TyRaw["String"]]]
        expect(ty_int_array.conforms_to?(ty_str_array, @env)).to be_falsy
      end
    end
  end
end
