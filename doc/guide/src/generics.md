# Generics

## Generic classes

A class can take _type parameters_.

```sk
class Stack<T>
  def initialize
    let @items = Array<T>.new
  end

  def push(item: T)
    @items.push(item)
  end

  def pop -> T
    @items.pop
  end
end

let s = Stack<Int>.new
s.push(1)
p s.pop  #=> 1
```

Built-in classes like `Array<T>`, `Dict<K, V>`, `Maybe<V>` and
`Fn1<A, R>` are generic classes.

To create an instance of a generic class, specify the type arguments
with `<>`.

```sk
let a = Array<Int>.new
let d = Dict<String, Int>.new
```

For array literals, the type argument is inferred from the elements.

```sk
let a = [1, 2, 3]       # Array<Int>
let b = [1, "foo"]      # Array<Object>
```

## Generic methods

A method can also take type parameters.

```sk
class A
  def self.first<T>(a: Array<T>) -> T
    a[0]
  end
end
```

Type arguments of a method call are specified with `<>` after the
method name.

```sk
[1, 2, 3].map<String>{|i: Int| i.to_s}
```

Currently the type arguments are mandatory in most cases, but they
should be inferred in future versions of Shiika.

## Generic modules

Modules can be generic too. See [Modules](./modules.md) for the
`Enumerable<E>` example.
