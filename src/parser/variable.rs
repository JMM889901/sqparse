use crate::ast::{Precedence, VarDefinition, VarInitializer};
use crate::parser::expression::expression;
use crate::parser::identifier::identifier;
use crate::parser::parse_result_ext::ParseResultExt;
use crate::parser::token_list::TokenIter;
use crate::parser::token_list_ext::TokenListExt;
use crate::parser::ParseResult;
use crate::token::TerminalToken;

pub fn var_definition<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, VarDefinition<'a>, Tokens> {
    let (tokens, name) = identifier(tokens)?;
    let (tokens, initializer) = var_initializer(tokens).maybe(tokens)?;
    Ok((tokens, VarDefinition { name, initializer }))
}

pub fn var_initializer<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, VarInitializer<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Assign)
        .determines(|tokens, assign| {
            expression(tokens, Precedence::Comma).map_val(|value| VarInitializer { assign, value })
        })
}
