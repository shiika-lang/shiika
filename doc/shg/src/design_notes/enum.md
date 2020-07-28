# Enum (Tentative)

## Basic example

```
  def foo -> Option<Int>
    if bar
      None.instance
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
      Nil.instance   # or Expr::Nil<Int>.instance, an instance of Expr::Nil<Int>
    elsif baz
      Value.new(99)
    else
      Cons.new(Nil, Nil)
    end
  end
end
```

## Why .instance is needed

If this is allowed, `Nil` is a short notaiton of `Nil<Int>`, but `Nil` is not a class, so we need to introduce something like "type-parameterized constant".

```
class MyLib
  enum Expr<T>
    case Nil
    case Value(v: T)
    case Cons(car: Expr<T>, cdr: Expr<T>)
  end

  def foo -> Expr<Int>
    Nil            
  end
end
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
