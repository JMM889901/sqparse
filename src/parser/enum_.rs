use crate::ast::EnumEntry;
use crate::parser::identifier::identifier;
use crate::parser::parse_result_ext::ParseResultExt;
use crate::parser::token_list::{TokenIter};
use crate::parser::token_list_ext::TokenListExt;
use crate::parser::variable::var_initializer;
use crate::parser::ParseResult;
use crate::token::TerminalToken;

pub fn enum_entry<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, EnumEntry<'a>, Tokens> {
    identifier(tokens).determines(|tokens, name| {
        let (tokens, initializer) = var_initializer(tokens).maybe(tokens)?;
        let (tokens, comma) = tokens.terminal(TerminalToken::Comma).maybe(tokens)?;
        Ok((
            tokens,
            EnumEntry {
                name,
                initializer,
                comma,
            },
        ))
    })
}
