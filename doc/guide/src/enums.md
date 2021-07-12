# Enums

## Enum definition

```
enum Tree<V>
  case Node(left: Tree<V>, right: Tree<V>)
  case Leaf(value: V)
end
```

(Unfortunately there is no syntax to define methods yet. It will come with basic pattern maching. Stay tuned!)
