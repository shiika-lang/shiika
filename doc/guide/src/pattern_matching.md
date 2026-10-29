# Pattern Matching

The `match` expression compares a value against patterns and evaluates
the clause of the first pattern that matches.

```sk
let a = Some.new(1)

match a
when Some(n)
  p n       #=> 1
when None
  p "none."
end
```

A clause starts with `when` followed by a pattern. The body may be
written on the next line, or on the same line after `then`.

```sk
let x = match pick
        when Some(v) then v
        when None then 0
        end
```

`match` is an expression; it evaluates to the value of the executed
clause. Like `if`, all the clauses must have the same type (except
`Void` and `Never`).

## Patterns

### Extractor pattern

Matches an instance of a class (typically an enum case class) and
binds its instance variables to variables.

```sk
match tree
when Tree::Node(left, right)
  ...
when Tree::Leaf(value)
  ...
end
```

The case name is resolved like a constant; inside the methods of the
enum itself you can simply write `Node(l, r)`. Patterns can be nested
like `Some(Pair(a, b))`.

### Literal pattern

Matches when the value equals the literal (`Int`, `Float`, `String`,
`true`/`false`).

```sk
match n
when 1 then "one"
when 2 then "two"
else "many"
end
```

### Variable pattern

A lowercase name matches any value and binds it to that name. It also
serves as a wildcard. Use `_` when you don't need the value.

```sk
match n
when 1 then 100
when x then x + 1  # matches any other Int
end
```

## Else clause

An `else` clause matches when no `when` clause matched.

```sk
match n
when 1 then "one"
else "other"
end
```
