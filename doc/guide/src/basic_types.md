# Basic classes

## Bool

```
true
false
```

## Int

```
12 + 34
```

## Float

```
1.2 + 3.4
```

## String

```
puts "Hello, world!"
```

## Array

```
let a = [1, 2, 3]

let b = Array<Int>.new
b.push(0)
```

## Dict

```
let d = Dict<String, Int>.new
d["a"] = 1
d["b"] = 2

p d["a"]  #=> 1
```

(There is no literals for dictionaries. Do you want?)

## Maybe

```
let a = Some.new(1)
let b = None

match a
when Some(n)
  p n
when None
  p "none."
end
```

## Result

`Result` is defined as follows and used by classes like `File`.

```
enum Result<V>
  case Ok(value: V)
  case Fail(err: Error)
  ...
```

## Error

`Error` is defined as follows.

```
class Error
  def initialize(@msg: String); end
  ...

  # TODO: def backtrace -> Array<String>
```

## And more

Please refer to `./builtin/*.sk` for other built-in types.
