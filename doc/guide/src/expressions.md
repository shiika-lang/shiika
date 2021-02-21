# Expressions

## Literals

- `1` evaluates to an instance of `Int`
- `1.0` evaluates to an instance of `Float`
- `"foo"` evaluates to an instance of `String`
- `true` and `false` evaluates to an instance of `Bool`

### Array literal

- `[1, 2]` evaluates to an instance of `Array<Int>`
- `[1, "foo"]` evaluates to an instance of `Array<Object>`

## Self expression

Example

```sk
class A
  def foo
    self  #=> The type of `self` is `A` here
    self.bar
  end

  def bar
    puts "bar"
  end
end
```

In the toplevel, `self` evaluates to the toplevel self. The type of toplevel self is `Object`.

## Variable declaration/assignment

There are two ways to declare a local variable.

- `x = 1`
- `var x = 1`

Reassigning to `x` only allowed for the latter form.

- `@a = 1`
- `var @a = 1`

## Lambda expression

An instance of the classes `Fn0`, `Fn1`, ..., `Fn9` is called a _lambda_. Lambdas can be created by _lambda expression_.

- `fn{ p 1 }` evaluates to an instance of `Fn0<Void>`
- `fn(x: Int){ p x }` evaluates to an instance of `Fn1<Int, Void>`

### Invoking a function

```sk
f = fn{ p 1 }
f.call
```

TODO: `f()` should also be ok https://github.com/yhara/shiika/issues/264

## Method call

- `1.abs`
- `foo`
- `foo()`
- `foo(1, 2, 3)`

### Blocks

- `foo(1, 2, 3){|x: Int| p x}`
- `foo(1, 2, 3) do |x: Int| p x end`

These are mostly equal to

- `foo(1, 2, 3, fn(x: Int){ p x })`

but some behaviors, `break` and `return` for example, are different between _fn_ (a lambda made by lambda expression) and _block_ (a lambda made by `{}` or `do...end` on a method call).

Conventionally, `do...end` is used when the value of the block is not used.

Type annotations are mandatory now but it should be inferred in the future version of Shiika.

## Logical operators

The type of these expressions are `Bool`.

- `!x`
- `x && y`
- `x || y`

Both `x` and `y` must be `Bool`.

## Conditional expression

### If

Example

```sk
if foo
  puts "foo"
elsif bar
  puts "bar
else
  puts "otherwise"
end

x = if a then b else c end
```

The type of an if condition (`foo`, `bar` and `a` avobe) must be `Bool`.

The types of if branches must be the same, except `Void` (such as `puts`) and `Never` (`break` or `return`).

### If modifier

`x if y` is equivalent to

```sk
if y
  x
end
```

### Unless

```sk
unless foo
  puts "otherwise"
end
```

is equivalent to

```sk
if !foo
  puts "otherwise"
end
```

Note: `unless` cannot take `elsif` or `else` clause.

### Unless modifier

`x unless y` is equivalent to

```sk
if !y
  x
end
```

### Conditional operator

`a ? b : c` is equivalent to

```sk
if a
  b
else
  c
end
```

## Loop and jump expressions

### While

```sk
var a = 1
while a < 10
  p a
  a += 1
end
```

Type of a while expressions is `Void`.

### Break

`break` escapes from the current loop, i.e. a `while` or a block.

```sk
var a = 1
while a < 10
  p a
  break if a == 5
  a += 1
end

[1, 2, 3].each do |i: Int|
  p i
  break if i == 2
end
```

You cannot use `break` inside a fn. Use `return` to escape from a fn.

```sk
f = fn(){ break }  #=> error
g = fn(){
  if foo
    return   #=> ok
  end
  bar
}
```

### Return

`return` escapes from the current method or fn.
