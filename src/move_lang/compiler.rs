use move_lang::{
    shared::Address, strip_comments_and_verify, parser, compile_program,
    parser::syntax::parse_file_string, stdlib,
};
use anyhow::{Result, Error};
use move_lang::errors::Errors;
use move_lang::to_bytecode::translate::CompiledUnit;

pub fn build(source_code: &str, deps: Vec<&str>, address: Address) -> Result<CompiledUnit> {
    let pprog_res = parse_program(source_code, deps)?;
    let mut prog = compile_program(pprog_res, Some(address)).map_err(|_err| {
        // TODO: render errors.
        Error::msg("Compile error.")
    })?;
    Ok(prog.remove(0))
}

fn parse_module(
    src: &str,
    name: &'static str,
) -> Result<(Option<parser::ast::FileDefinition>, Errors)> {
    let mut errors: Errors = Vec::new();

    let no_comments_buffer = match strip_comments_and_verify(name, src) {
        Err(err) => {
            errors.push(err);
            return Ok((None, errors));
        }
        Ok(no_comments_buffer) => no_comments_buffer,
    };
    let def_opt = match parse_file_string(name, &no_comments_buffer) {
        Ok(def) => Some(def),
        Err(err) => {
            errors.push(err);
            None
        }
    };
    Ok((def_opt, errors))
}

fn parse_program(
    source_code: &str,
    mut deps: Vec<&str>,
) -> Result<Result<parser::ast::Program, Errors>> {
    let mut source_definitions = Vec::new();
    let mut lib_definitions = Vec::new();
    let mut errors: Errors = Vec::new();

    let (def_opt, mut es) = parse_module(source_code, "src")?;
    if let Some(def) = def_opt {
        source_definitions.push(def);
    }
    errors.append(&mut es);

    deps.extend(stdlib());
    for module in deps {
        let (def_opt, mut es) = parse_module(&module, "dep")?;
        if let Some(def) = def_opt {
            lib_definitions.push(def);
        }
        errors.append(&mut es);
    }

    let res = if errors.is_empty() {
        Ok(parser::ast::Program {
            source_definitions,
            lib_definitions,
        })
    } else {
        Err(errors)
    };
    Ok(res)
}
