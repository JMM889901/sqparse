use crate::ast::GlobalDefinition;
use crate::parser::identifier::identifier;
use crate::parser::parse_result_ext::ParseResultExt;
use crate::parser::statement::{
    class_definition_statement, const_definition_statement, enum_definition_statement,
    struct_definition_statement, type_definition_statement, typed_var_definition_statement,
};
use crate::parser::token_list::TokenIter;
use crate::parser::token_list_ext::TokenListExt;
use crate::parser::type_::type_;
use crate::parser::variable::var_initializer;
use crate::parser::ParseResult;
use crate::token::TerminalToken;
use crate::ParseErrorType;

pub fn global_definition<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, GlobalDefinition<'a>, Tokens> {
    function_global(tokens)
        .or_try(|| const_global(tokens))
        .or_try(|| enum_global(tokens))
        .or_try(|| class_global(tokens))
        .or_try(|| struct_global(tokens))
        .or_try(|| type_global(tokens))
        .or_try(|| untyped_var_global(tokens))
        .or_try(|| typed_var_global(tokens))
        .or_error(|| tokens.error(ParseErrorType::ExpectedGlobalDefinition))
}

fn function_global<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, GlobalDefinition<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Function)
        .determines(|tokens, function| {
            identifier(tokens).map_val(|name| GlobalDefinition::Function { function, name })
        })
}

fn const_global<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, GlobalDefinition<'a>, Tokens> {
    const_definition_statement(tokens).map_val(GlobalDefinition::Const)
}

fn enum_global<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, GlobalDefinition<'a>, Tokens> {
    enum_definition_statement(tokens).map_val(GlobalDefinition::Enum)
}

fn class_global<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, GlobalDefinition<'a>, Tokens> {
    class_definition_statement(tokens).map_val(GlobalDefinition::Class)
}

fn struct_global<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, GlobalDefinition<'a>, Tokens> {
    struct_definition_statement(tokens).map_val(GlobalDefinition::Struct)
}

fn type_global<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, GlobalDefinition<'a>, Tokens> {
    type_definition_statement(tokens).map_val(GlobalDefinition::Type)
}

fn untyped_var_global<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, GlobalDefinition<'a>, Tokens> {
    let (tokens, name) = identifier(tokens)?;
    let (tokens, initializer) = var_initializer(tokens)?;
    Ok((tokens, GlobalDefinition::UntypedVar { name, initializer }))
}

fn typed_var_global<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, GlobalDefinition<'a>, Tokens> {
    let (tokens, type_) = type_(tokens).not_line_ending().not_definite()?;

    typed_var_definition_statement(tokens, type_).map_val(GlobalDefinition::TypedVar)
}
