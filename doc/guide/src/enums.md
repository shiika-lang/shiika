# Enums

## Enum definition

Example:

```
enum Tree<V>
  case Node(left: Tree<V>, right: Tree<V>)
  case Leaf(value: V)

  def dump
    match self
    when Node(l, r)
      print "("
      l.dump
      print ", "
      r.dump
      print ")"
    when Leaf(v)
      print v.inspect
    end
  end
end

# Currently you need to write `<Int>` but this can be omitted in the future.
tree = Tree::Node<Int>.new(
  Tree::Node<Int>.new(
    Tree::Leaf<Int>.new(1),
    Tree::Leaf<Int>.new(2)
  ),
  Tree::Leaf<Int>.new(3)
)
tree.dump
puts ""
```
