use crate::mv::disassembler::code::translator::Context;
use crate::mv::disassembler::code::exp::ExpLoc;
use crate::embedded::Bytecode;

#[allow(dead_code)]
#[derive(Debug)]
pub enum Algorithm {
    EmptyIf,
    If {
        true_branch: Branch,
    },
    IfElse {
        true_branch: Branch,
        false_branch: Branch,
    },
    Loop {
        body: Branch,
    },
    While {
        body: Branch,
    },
    None,
}

impl Algorithm {
    pub fn br_true<'a>(true_offset: usize, ctx: &mut impl Context<'a>) -> Algorithm {
        let condition = ExpLoc::default();
        let condition_index = ctx.last_exp().unwrap_or_else(|| &condition).range().0;
        let remaining_code = ctx.remaining_code();
        let next = remaining_code[0].offset();

        if let Some(false_offset) = next {
            Self::with_unconditional(condition_index, true_offset, *false_offset as usize, ctx)
        } else {
            Self::only_conditional(condition_index, true_offset, ctx)
        }
    }

    ///BrTrue(`true_offset`)
    /// .....
    fn only_conditional<'a>(condition_index: usize, true_offset: usize, ctx: &mut impl Context<'a>) -> Algorithm {
        let opcode_offset = ctx.opcode_offset();
        if true_offset == opcode_offset + 1 {
            //if (...) {}
            Algorithm::EmptyIf
        } else {
            //BrTrue(true_offset)
            //  {:current_offset  + 1
            //        false branch.
            //        br(true_end_offset)
            //  }:true_offset - 1
            //  {:true_offset
            //      true branch.
            //  }:true_end_offset

            let last_opcode_in_false_branch = ctx.opcode_by_absolute_offset(true_offset - 1);

            if last_opcode_in_false_branch.is_branch() {
                if let Some(end_of_true_branch) = last_opcode_in_false_branch.offset() {
                    let end_of_true_branch = *end_of_true_branch as usize;
                    let false_branch = Branch::new(opcode_offset + 1, true_offset - 2, 0, BranchEnd::Continue);
                    if end_of_true_branch <= condition_index {
                        Algorithm::IfElse { true_branch: Branch::new(true_offset, ctx.end_offset(), 1, BranchEnd::Nop), false_branch }
                    } else {
                        Algorithm::IfElse { true_branch: Branch::new(true_offset, end_of_true_branch, 1, BranchEnd::Nop), false_branch }
                    }
                } else {
                    let false_branch = Branch::new(opcode_offset + 1, true_offset - 1, 0, BranchEnd::Nop);
                    Algorithm::IfElse { true_branch: Branch::new(true_offset, ctx.end_offset(), 0, BranchEnd::Nop), false_branch }
                }
            } else {
                let false_branch = Branch::new(opcode_offset + 1, true_offset - 1, 0, BranchEnd::Nop);
                Algorithm::IfElse { true_branch: Branch::new(true_offset, ctx.end_offset(), 0, BranchEnd::Nop), false_branch }
            }
        }
    }

    ///BrTrue(`true_offset`)
    ///Branch(`false_offset`)
    ///....
    fn with_unconditional<'a>(condition_index: usize, true_offset: usize, false_offset: usize, ctx: &mut impl Context<'a>) -> Algorithm {
        //BrTrue(true_offset)
        //Branch(false_offset)
        //....
        //true_branch
        if false_offset > true_offset {
            //BrTrue(true_offset)
            //Branch(false_offset)
            //{
            //....
            //Branch(out_off_branch_offset) or Ret
            //}
            let last_opcode_in_true_branch = ctx.opcode_by_absolute_offset(false_offset - 1);
            if last_opcode_in_true_branch.is_branch() {
                if let Some(jmp_from_true_branch) = last_opcode_in_true_branch.offset() {
                    let jmp_from_true_branch = *jmp_from_true_branch as usize;
                    //BrTrue(true_offset)
                    //Branch(false_offset)
                    //{
                    //....
                    //Branch(out_off_branch_offset)
                    //}
                    if jmp_from_true_branch > ctx.opcode_offset() {
                        if last_opcode_in_true_branch.is_conditional_branch() {
                            // if () { if () {}}
                            Algorithm::If { true_branch: Branch::new(true_offset, false_offset - 1, 0, BranchEnd::Nop) }
                        } else {
                            let true_branch = Branch::new(true_offset, false_offset - 1, 0, BranchEnd::Nop);
                            let false_branch = Branch::new(false_offset, jmp_from_true_branch - 1, 0, BranchEnd::Nop);
                            Algorithm::IfElse { true_branch, false_branch }
                        }
                    } else {
                        if jmp_from_true_branch == condition_index {
                            // while : out_off_true_branch_offset
                            //BrTrue(true_offset)
                            //Branch(false_offset)
                            //{:true_offset
                            //  .... loop branch
                            //  Branch(out_off_true_branch_offset)
                            //}: false_offset
                            Algorithm::While { body: Branch::new(true_offset, false_offset - 2/*ignore branch opcode*/, 1, BranchEnd::Nop) }
                        } else {
                            //if
                            //BrTrue(true_offset)
                            //Branch(false_offset)
                            //{:true_offset
                            //  .... true branch
                            //  continue;
                            //} else
                            //{:false_offset
                            //  .... false branch
                            //}:out_off_true_branch_offset
                            // TODO
                            todo!();
                            Algorithm::None
                        }
                    }
                } else {
                    let end = if &Bytecode::Ret == last_opcode_in_true_branch {
                        BranchEnd::Return
                    } else {
                        BranchEnd::Nop
                    };
                    //if
                    //BrTrue(true_offset)
                    //Branch(false_offset)
                    //{
                    //....
                    //ret || abort
                    //}
                    Algorithm::If { true_branch: Branch::new(true_offset, false_offset - 1/*ignore branch opcode*/, 0, end) }
                }
            } else {
                //if
                //BrTrue(true_offset)
                //Branch(false_offset)
                //{:true_offset
                //  .... true branch
                //  continue;
                //}:false_offset

                dbg!(last_opcode_in_true_branch);
                Algorithm::If { true_branch: Branch::new(true_offset, false_offset - 1, 0, BranchEnd::Nop) }
            }
        } else {
            //TODO
            todo!();
            Algorithm::None
        }
    }


    pub fn br<'a>(offset: usize, ctx: &mut impl Context<'a>) -> Algorithm {
        Algorithm::None
    }
}

#[derive(Debug)]
pub struct Branch {
    pub start_offset: usize,
    pub end_offset: usize,
    pub ignored_tail: usize,
    pub special_end: BranchEnd,
}

impl Branch {
    pub fn new(start_offset: usize, end_offset: usize, ignored_tail: usize, special_end: BranchEnd) -> Branch {
        Branch {
            start_offset,
            end_offset,
            ignored_tail,
            special_end,
        }
    }

    pub fn size(&self) -> usize {
        self.end_offset - self.start_offset + 1
    }
}

#[derive(Debug)]
pub enum BranchEnd {
    Nop,
    Continue,
    Break,
    Return,
}
