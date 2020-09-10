use crate::mv::disassembler::code::translator::Context;
use crate::mv::disassembler::code::exp::ExpLoc;
use core::cmp;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum Algorithm {
    EmptyIf {
        condition_index: usize,
    },
    If {
        condition_index: usize,
        true_branch: Branch,
    },
    IfElse {
        condition_index: usize,
        true_branch: Branch,
        false_branch: Branch,
    },
    Loop {
        start_index: usize,
        body: Branch,
    },
    While {
        condition_index: usize,
        body: Branch,
    },
    Continue,
    Break,
    None,
}

impl Algorithm {
    pub fn is_if(&self) -> bool {
        match self {
            Algorithm::If {
                condition_index: _,
                true_branch: _,
            } => true,
            Algorithm::EmptyIf { condition_index: _ } => true,
            Algorithm::IfElse {
                condition_index: _,
                true_branch: _,
                false_branch: _,
            } => true,
            Algorithm::Loop {
                body: _,
                start_index: _,
            } => false,
            Algorithm::While {
                condition_index: _,
                body: _,
            } => false,
            Algorithm::Continue => false,
            Algorithm::Break => false,
            Algorithm::None => false,
        }
    }

    pub fn full_range(&self) -> (usize, usize) {
        match self {
            Algorithm::None => (0, 0),
            Algorithm::EmptyIf { condition_index } => (*condition_index, *condition_index),
            Algorithm::If { condition_index, true_branch } => (*condition_index, true_branch.end_offset),
            Algorithm::IfElse { condition_index, true_branch, false_branch } => (*condition_index, cmp::max(true_branch.end_offset, false_branch.end_offset)),
            Algorithm::Loop { start_index, body } => (*start_index, body.end_offset),
            Algorithm::While { condition_index, body } => (*condition_index, body.end_offset),
            Algorithm::Continue => (0, 0),
            Algorithm::Break => (0, 0),
        }
    }

    pub fn is_loop(&self) -> bool {
        match self {
            Algorithm::If {
                condition_index: _,
                true_branch: _,
            } => false,
            Algorithm::EmptyIf { condition_index: _ } => false,
            Algorithm::IfElse {
                condition_index: _,
                true_branch: _,
                false_branch: _,
            } => false,
            Algorithm::Loop {
                body: _,
                start_index: _,
            } => true,
            Algorithm::While {
                condition_index: _,
                body: _,
            } => true,
            Algorithm::Continue => false,
            Algorithm::Break => false,
            Algorithm::None => false,
        }
    }

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
    fn only_conditional<'a>(
        condition_index: usize,
        true_offset: usize,
        ctx: &mut impl Context<'a>,
    ) -> Algorithm {
        let opcode_offset = ctx.opcode_offset();
        if true_offset == opcode_offset + 1 {
            //if (...) {}
            Algorithm::EmptyIf { condition_index }
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
                    let false_branch = Branch::new(opcode_offset + 1, true_offset - 2, 0);
                    if end_of_true_branch <= condition_index {
                        Algorithm::IfElse {
                            condition_index,
                            true_branch: Branch::new(true_offset, ctx.end_offset(), 1),
                            false_branch,
                        }
                    } else {
                        Algorithm::IfElse {
                            condition_index,
                            true_branch: Branch::new(true_offset, end_of_true_branch, 1),
                            false_branch,
                        }
                    }
                } else {
                    let false_branch = Branch::new(opcode_offset + 1, true_offset - 1, 0);
                    Algorithm::IfElse {
                        condition_index,
                        true_branch: Branch::new(true_offset, ctx.end_offset(), 0),
                        false_branch,
                    }
                }
            } else {
                let false_branch = Branch::new(opcode_offset + 1, true_offset - 1, 0);
                Algorithm::IfElse {
                    condition_index,
                    true_branch: Branch::new(true_offset, ctx.end_offset(), 0),
                    false_branch,
                }
            }
        }
    }

    ///BrTrue(`true_offset`)
    ///Branch(`false_offset`)
    ///....
    fn with_unconditional<'a>(
        condition_index: usize,
        true_offset: usize,
        false_offset: usize,
        ctx: &mut impl Context<'a>,
    ) -> Algorithm {
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
                            Algorithm::If {
                                condition_index,
                                true_branch: Branch::new(true_offset, false_offset - 1, 0),
                            }
                        } else {
                            let true_branch = Branch::new(true_offset, false_offset - 1, 0);
                            let false_branch =
                                Branch::new(false_offset, jmp_from_true_branch - 1, 0);
                            Algorithm::IfElse {
                                condition_index,
                                true_branch,
                                false_branch,
                            }
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
                            Algorithm::While {
                                condition_index,
                                body: Branch::new(
                                    true_offset,
                                    false_offset - 2, /*ignore branch opcode*/
                                    1,
                                ),
                            }
                        } else if jmp_from_true_branch < condition_index {
                            //if() {...} else {break}

                            if ctx.opcode_by_absolute_offset(false_offset).is_branch() {
                                Algorithm::IfElse {
                                    condition_index,
                                    true_branch: Branch::new(true_offset, false_offset - 1, 0),
                                    false_branch: Branch::new(false_offset, false_offset, 0),
                                }
                            } else {
                                Algorithm::None
                            }
                        } else {
                            Algorithm::None
                        }
                    }
                } else {
                    //if
                    //BrTrue(true_offset)
                    //Branch(false_offset)
                    //{
                    //....
                    //ret || abort
                    //}
                    Algorithm::If {
                        condition_index,
                        true_branch: Branch::new(
                            true_offset,
                            false_offset - 1, /*ignore branch opcode*/
                            0,
                        ),
                    }
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
                Algorithm::If {
                    condition_index,
                    true_branch: Branch::new(true_offset, false_offset - 1, 0),
                }
            }
        } else {
            //TODO
            todo!();
            Algorithm::None
        }
    }

    pub fn br<'a>(offset: usize, ctx: &mut impl Context<'a>) -> Algorithm {
        if let Some(algo) = Self::check_loop(offset, ctx) {
            return algo;
        }

        let parent_algorithms = ctx.parent_algorithms();
        if let Some(current) = parent_algorithms.last() {
            if current.is_if() && offset > ctx.opcode_offset() {
                return if current.full_range().1 + 1 == offset {
                    Algorithm::None
                } else {
                    Algorithm::Break
                }
            }
            dbg!(current);
            dbg!(ctx.opcode_offset());
            Algorithm::None
            // if current.is_if() {
            //     dbg!(current);
            //     //loop {}
            //
            //     Algorithm::Break
            // } else {
            //     //error
            //     Algorithm::None
            // }
        } else {
            if offset < ctx.opcode_offset() {
                //loop
                Algorithm::Loop {
                    start_index: 0,
                    body: Branch::new(offset, ctx.opcode_offset() - 1, 0),
                }
            } else {
                //error
                Algorithm::None
            }
        }
    }

    pub fn check_loop<'a>(offset: usize, ctx: &mut impl Context<'a>) -> Option<Algorithm> {
        fn lp<'a>(offset: usize, ctx: &mut impl Context<'a>) -> Option<Algorithm> {
            if offset == ctx.opcode_offset() {
                Some(Algorithm::Loop {
                    start_index: 0,
                    body: Branch::new(offset, offset, 0),
                })
            } else {
                Some(Algorithm::Loop {
                    start_index: 0,
                    body: Branch::new(offset, ctx.opcode_offset() - 1, 0),
                })
            }
        }

        let parent_algorithms = ctx.parent_algorithms();
        if let Some(last) = parent_algorithms.last() {
            match last {
                Algorithm::If {
                    condition_index: _,
                    true_branch,
                } => {
                    if true_branch.check_range(offset) {
                        lp(offset, ctx)
                    } else {
                        None
                    }
                }
                Algorithm::IfElse {
                    condition_index: _,
                    true_branch,
                    false_branch,
                } => {
                    if true_branch.check_range(offset) || false_branch.check_range(offset) {
                        lp(offset, ctx)
                    } else {
                        None
                    }
                }
                Algorithm::Loop {
                    start_index: _,
                    body,
                }
                | Algorithm::While {
                    condition_index: _,
                    body,
                } => {
                    if body.check_range(offset) {
                        lp(offset, ctx)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else {
            lp(offset, ctx)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Branch {
    pub start_offset: usize,
    pub end_offset: usize,
    pub ignored_tail: usize,
}

impl Branch {
    pub fn new(start_offset: usize, end_offset: usize, ignored_tail: usize) -> Branch {
        Branch {
            start_offset,
            end_offset,
            ignored_tail,
        }
    }

    pub fn check_range(&self, offset: usize) -> bool {
        self.start_offset <= offset && self.end_offset >= offset
    }

    pub fn size(&self) -> usize {
        self.end_offset - self.start_offset + 1
    }
}
