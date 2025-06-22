use crate::ast::{ArrayValue, Precedence, Preprocessable};
use crate::parser::expression::expression;
use crate::parser::parse_result_ext::ParseResultExt;
use crate::parser::token_list::TokenIter;
use crate::parser::token_list_ext::TokenListExt;
use crate::parser::{ParseResult};
use crate::token::TerminalToken;

use super::preprocessed::{preprocessed_if, preprocessed_if_contents_terminal};

pub fn possibly_preprocessed_array_value<'a, Tokens: TokenIter<'a>>(
    tokens: Tokens
) -> ParseResult<'a, Preprocessable<'a, ArrayValue<'a>>, Tokens> {
    preprocessed_array_value(tokens).or_try(|| array_value(tokens))
}

pub fn array_value<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, Preprocessable<'a, ArrayValue<'a>>, Tokens> {
    let (tokens, value) = expression(tokens, Precedence::Comma)?;
    let (tokens, separator) = tokens.terminal(TerminalToken::Comma).maybe(tokens)?;
    Ok((
        tokens,
        Preprocessable::UNCONDITIONAL(ArrayValue { value, separator }),
    ))
}

pub fn preprocessed_array_value<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, Preprocessable<'a, ArrayValue<'a>>, Tokens> {
    let (tokens, preprocessed) = preprocessed_if(tokens, |tokens| {
        tokens.many_until(
            preprocessed_if_contents_terminal,
            possibly_preprocessed_array_value,
        )
    })?;
    Ok((tokens, Preprocessable::PREPROCESSED(preprocessed)))
}
