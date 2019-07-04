mod converter;

pub struct Mir {
    pub funcs: Vec<MirFunc>,
}

pub struct MirFunc {
    pub name: string,
    pub body_stmts: MirStatement
}
