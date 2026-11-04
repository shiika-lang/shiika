# Basic Concepts

In Shiika, every value is an _object_ and belongs to a _class_.

## Programs

A Shiika program is a sequence of definitions (classes, modules,
enums, constants) and expressions. The toplevel expressions are
executed in order.

Methods cannot be defined at the toplevel; every method belongs to a
class, a module or an enum.

## Comments

A comment starts with `#` and continues to the end of the line.

```sk
# This is a comment
```

## Identifiers

- Names of local variables and methods start with a lowercase letter
  (`foo`, `foo?`).
- Names of instance variables start with `@` (`@foo`).
- Names of constants, classes, modules and enums start with an
  uppercase letter (`Foo`, `FOO`).
- Names of type parameters start with an uppercase letter (`T`).

## Source files and `require`

A source file can load another file with `require`.

```sk
require "./foo.sk"
```

`require` must appear at the beginning of the file. The path is
relative to the requiring file. A file is loaded only once even if
required multiple times.
