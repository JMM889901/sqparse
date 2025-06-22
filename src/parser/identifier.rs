use crate::ast::{Identifier, MethodIdentifier};
use crate::parser::parse_result_ext::ParseResultExt;
use crate::parser::token_list::TokenIter;
use crate::parser::token_list_ext::TokenListExt;
use crate::parser::ParseResult;
use crate::token::{TerminalToken, TokenType};
use crate::ParseErrorType;

pub fn identifier<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, Identifier<'a>, Tokens> {
    if let Some((tokens, item)) = tokens.split_first() {
        if let TokenType::Identifier(value) = item.token.ty {
            return Ok((
                tokens,
                Identifier {
                    value,
                    token: &item.token,
                },
            ));
        }
    }

    Err(tokens.error(ParseErrorType::ExpectedIdentifier))
}

pub fn method_identifier<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, MethodIdentifier<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Constructor)
        .map_val(MethodIdentifier::Constructor)
        .or_try(|| identifier(tokens).map_val(MethodIdentifier::Identifier))
}
