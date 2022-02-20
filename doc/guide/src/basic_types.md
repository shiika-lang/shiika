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
a = [1, 2, 3]

b = Array<Int>.new
b.push(0)
```

## Maybe

```
a = Some.new(1)
b = None

match a
when Some(n)
  p n
when None
  p "none."
end
```
