use crate::ast::{Precedence, Preprocessable, TableSlot, TableSlotType};
use crate::parser::expression::expression;
use crate::parser::parse_result_ext::ParseResultExt;
use crate::parser::slot::slot;
use crate::parser::token_list::TokenIter;
use crate::parser::token_list_ext::TokenListExt;
use crate::parser::ParseResult;
use crate::token::{LiteralToken, StringToken, TerminalToken, Token, TokenType};
use crate::{ContextType, ParseErrorType};

use super::preprocessed::{preprocessed_if, preprocessed_if_contents_terminal};

pub fn possibly_preprocessed_table_slot<'a, Tokens: TokenIter<'a>>(
    tokens: Tokens,
) -> ParseResult<'a, Preprocessable<'a, TableSlot<'a>>, Tokens> {
    preprocessed_table_slot(tokens).or_try(|| table_slot(tokens))
}

pub fn preprocessed_table_slot<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, Preprocessable<'a, TableSlot<'a>>, Tokens> {
    let (tokens, preprocessed) = preprocessed_if(tokens, |tokens| {
        tokens.many_until(
            preprocessed_if_contents_terminal,
            possibly_preprocessed_table_slot,
        )
    })?;
    Ok((tokens, Preprocessable::PREPROCESSED(preprocessed)))
}

pub fn table_slot<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, Preprocessable<'a, TableSlot<'a>>, Tokens> {
    let (tokens, ty) = table_slot_type(tokens)?;
    let (tokens, comma) = tokens.terminal(TerminalToken::Comma).maybe(tokens)?;
    Ok((
        tokens,
        Preprocessable::UNCONDITIONAL(TableSlot { ty, comma }),
    ))
}

fn table_slot_type<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, TableSlotType<'a>, Tokens> {
    json_property_slot(tokens)
        .or_try(|| slot(tokens).map_val(TableSlotType::Slot))
        .or_error(|| tokens.error(ParseErrorType::ExpectedTableSlot))
}

fn json_property_slot<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, TableSlotType<'a>, Tokens> {
    string_literal(tokens)
        .determines(|tokens, (name, name_token)| {
            let (tokens, colon) = tokens.terminal(TerminalToken::Colon)?;
            let (tokens, value) = expression(tokens, Precedence::Comma)?;
            Ok((
                tokens,
                TableSlotType::JsonProperty {
                    name,
                    name_token,
                    colon,
                    value,
                },
            ))
        })
        .with_context_from(ContextType::Property, tokens)
}

pub fn string_literal<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, (&'a str, &'a Token<'a>), Tokens> {
    if let Some((tokens, item)) = tokens.split_first() {
        if let TokenType::Literal(LiteralToken::String(StringToken::Literal(name))) = item.token.ty
        {
            return Ok((tokens, (name, &item.token)));
        }
    }

    Err(tokens.error(ParseErrorType::ExpectedStringLiteral))
}
