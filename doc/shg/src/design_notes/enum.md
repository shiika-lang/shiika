# Enum (Tentative)

## Basic example

```
  def foo -> Option<Int>
    if bar
      None
    else
      Some.new(123)
    end
  end
```

```
class MyLib
  # This exports four constants Expr, Nil, Value and Cons.
  enum Expr<T>
    case Nil
    case Value(v: T)
    case Cons(car: Expr<T>, cdr: Expr<T>)
  end

  def foo -> Expr<Int>
    if bar
      Nil
    elsif baz
      Value.new(99)
    else
      Cons.new(Nil, Nil)
    end
  end
end
```

## Enum cases

```
enum E
  # Defines a class E::A and its instance ::E::A
  case A
  # Defines a class E::B
  case B(x: Int)
end

A  #=> ::E::A
B  #=> ::E::B
```

## Why .new is needed

If this is allowed, there should be a method `MyLib#Left`, that is very wierd.

```
class MyLib
  enum Either<V, E>
    case Left(E)
    case Right(V)
  end

  def foo -> Either<Int, String>
    Left(1)   # ???
  end
end
```
