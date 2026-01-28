pub const PREFIX_LET_BIND_ASYNC: &str = "$a";

pub struct Gensym {
    counter: usize,
    prefix: String,
}

impl Gensym {
    pub fn new(prefix: impl Into<String>) -> Self {
        Gensym {
            counter: 0,
            prefix: prefix.into(),
        }
    }

    pub fn new_name(&mut self) -> String {
        let name = format!("{}{}", self.prefix, self.counter);
        self.counter += 1;
        name
    }
}
