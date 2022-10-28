use crate::resource::VarMap;
use eyre::Result;
use fasteval::{Compiler, Evaler};

pub struct Evaluator {
    _parser: Box<fasteval::Parser>,
    slab: Box<fasteval::Slab>,
    instr: Box<fasteval::Instruction>,
}

impl Evaluator {
    pub fn new(expr: &str) -> Result<Self> {
        let parser = fasteval::Parser::new();
        let mut slab = fasteval::Slab::new();
        let instr = parser
            .parse(expr, &mut slab.ps)?
            .from(&slab.ps)
            .compile(&slab.ps, &mut slab.cs);

        Ok(Self {
            _parser: Box::new(parser),
            slab: Box::new(slab),
            instr: Box::new(instr),
        })
    }

    pub fn eval(&self, vars: &mut VarMap) -> Result<f64> {
        use fasteval::eval_compiled_ref;

        Ok(eval_compiled_ref!(
            self.instr.as_ref(),
            self.slab.as_ref(),
            vars
        ))
    }
}
