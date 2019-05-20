# Shiika

A statically-typed programming language.

## Example

        class Pair<S, T>
          def initialize(@a: S, @b: T)
          end

          def fst -> S; @a; end
          def snd -> T; @b; end
        end
        Pair<Int, Bool>.new(1, true).fst

## Status

### WIP

- [ ] Inheritance

### Done

- [x] Class method, instance method
- [x] Basic generics
- [x] Variable-length arguments
- [x] Array literal

### Todo (short-term)

- Subtyping
- String class
- Hash class

### Todo (middle term)

- lambda/function(block)
- Module (like Ruby's `Module`)
- Enum
- Optional
- Constants
- ...

### Todo (long term)

- Generate LLVM IR

### Todo (postponed error checks)

- [ ] Check all ivars will be initialized (like Swift)

## License

MIT

## Contact

https://github.com/yhara/shiika/issues
