require 'shiika/program/env'

module Shiika
  class Evaluator
    # Environment
    # Mostly the same as Program::Env but has some additional methods
    class Env < Program::Env
      def find_ivar_value(name)
        unless (sk_self = @data[:sk_self])
          raise ProgramError, "ivar reference out of a class: #{name}" 
        end
        return sk_self.ivar_values.fetch(name)
      end
    end
  end
end
