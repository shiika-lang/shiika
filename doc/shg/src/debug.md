# Debugging

Some hints to debug the Shiika compiler.

## Debug parser

Uncomment this in src/parser/base.rs

```rust
    /// Print parser debug log (uncomment to enable)
    pub(super) fn debug_log(&self, _msg: &str) {
        //println!("{}{} {}", self.lv_space(), _msg, self.lexer.debug_info());
    }
```
