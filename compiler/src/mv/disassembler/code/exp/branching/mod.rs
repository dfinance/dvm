#![warn(missing_docs)]
pub mod algorithms;

use crate::mv::disassembler::code::exp::Exp;
use crate::mv::disassembler::code::translator::Context;
use crate::embedded::Bytecode;
use crate::mv::disassembler::code::exp::block::Block;
use crate::mv::disassembler::code::exp::branching::algorithms::{Algorithm, Branch};

/// Handles `BrTrue` instruction.
pub fn br_true<'a>(true_offset: usize, ctx: &mut impl Context<'a>) -> Exp<'a> {
    let algo = Algorithm::br_true(true_offset, ctx);
    match &algo {
        Algorithm::While {
            condition_index: _,
            body: branch,
        } => Exp::While(ctx.pop_exp(), translate_branch(branch, ctx, algo)),
        Algorithm::EmptyIf { condition_index: _ } => {
            Exp::If(ctx.pop_exp(), Block::new(vec![], true), None)
        }
        Algorithm::If {
            condition_index: _,
            true_branch,
        } => {
            let true_branch = translate_branch(true_branch, ctx, algo);
            Exp::If(ctx.pop_exp(), true_branch, None)
        }
        Algorithm::IfElse {
            condition_index: _,
            true_branch,
            false_branch,
        } => {
            if true_branch.start_offset > false_branch.start_offset {
                let false_branch = translate_branch(false_branch, ctx, algo);
                let true_branch = translate_branch(true_branch, ctx, algo);
                Exp::If(ctx.pop_exp(), true_branch, Some(false_branch))
            } else {
                let true_branch = translate_branch(true_branch, ctx, algo);
                let false_branch = translate_branch(false_branch, ctx, algo);
                Exp::If(ctx.pop_exp(), true_branch, Some(false_branch))
            }
        }
        _ => Exp::Error(Bytecode::BrFalse(true_offset as u16)),
    }
}

fn translate_branch<'a>(
    branch: &Branch,
    ctx: &mut impl Context<'a>,
    algorithm: Algorithm,
) -> Block<'a> {
    ctx.skip_opcodes(branch.start_offset - ctx.opcode_offset() - 1);
    let mut body = ctx.translate_block(branch.size(), algorithm);
    ctx.skip_opcodes(branch.ignored_tail);

    if let Some(last) = body.last_mut() {
        let last = last.as_mut();
        if let Some(ret) = last.ret() {
            ret.explicit_keyword = true;
        }
    }

    Block::new(body, true)
}

/// Handles `BrFalse` instruction.
pub fn br_false<'a>(index: usize, ctx: &mut impl Context<'a>) -> Exp<'a> {
    // Used only with mvir.
    Exp::Error(Bytecode::BrFalse(index as u16))
}

/// Handles `Branch` instruction.
pub fn br<'a>(offset: usize, ctx: &mut impl Context<'a>) -> Exp<'a> {
    match Algorithm::br(offset, ctx) {
        Algorithm::Break => Exp::Break,
        Algorithm::Continue => Exp::Continue,
        Algorithm::Loop {
            start_index: _,
            body,
        } => Exp::Loop(Block::new(ctx.pop_exp_by_offset(body.start_offset), true)),
        Algorithm::None => Exp::Nop,
        _ => Exp::Error(ctx.opcode_by_relative_offset(0).clone()),
    }
}
