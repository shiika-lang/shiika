# Enums

## Enum definition

```
enum Enum1<A, B>
  case Case1(a: A)
  case Case2(b: B)
  case Case3
end
```

This defines these classes,

- `class Enum1<A, B> : Object`
- `class Enum1::Case1<A> : Enum1<A, Never>`
- `class Enum1::Case2<B> : Enum1<Never, B>`
- `class Enum1::Case3 : Enum1<Never, Never>`

these methods

- `Enum1::Case1<A>.new(a: A) -> Enum1::Case1<A>`
- `Enum1::Case1<A>#a -> A`
- `Enum1::Case2<B>.new(b: B) -> Enum1::Case2<B>`
- `Enum1::Case2<B>#b -> B`

and these constants.

- `::Enum1 : Meta:Enum1`
- `::Enum1::Case1 : Meta:Enum1::Case1`
- `::Enum1::Case2 : Meta:Enum1::Case2`
- `::Enum1::Case3 : Enum1::Case3`

Note that enum case with no parameters (`Case3` here) is special.

- They have no type parameters.
- `Never` is used for superclass type arguments.
- The constant `::Enum1::Case3` holds the (only) instance, not the class.

## Enum classes

The class `Enum1` is called an **Enum class**. The classes `Enum1::Case1`, `Enum1::Case2`, `Enum1::Case3`, are called **Enum case classses**.

Enum classes and enum case classes cannot be an explicit superclass.

```
# error
class A<T> : Enum1<T, T>
end
```
