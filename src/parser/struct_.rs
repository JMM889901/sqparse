use crate::ast::{Preprocessable, StructDefinition, StructProperty};
use crate::parser::identifier::identifier;
use crate::parser::parse_result_ext::ParseResultExt;
use crate::parser::token_list::TokenIter;
use crate::parser::token_list_ext::TokenListExt;
use crate::parser::type_::type_;
use crate::parser::variable::var_initializer;
use crate::parser::ParseResult;
use crate::token::TerminalToken;
use crate::ContextType;

use super::preprocessed::{preprocessed_if, preprocessed_if_contents_terminal};

pub fn struct_definition<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, StructDefinition<'a>, Tokens> {
    tokens.terminal(TerminalToken::OpenBrace).opens(
        ContextType::Span,
        |tokens| tokens.terminal(TerminalToken::CloseBrace),
        |tokens| {
            tokens
                .many(possibly_preprocessed_struct_property)
        },
        |tokens, open, properties, close| {
            Ok((tokens, StructDefinition {
                open,
                properties,
                close,
            }))
        },
    )
}

pub fn possibly_preprocessed_struct_property<'a, Tokens: TokenIter<'a>>(
    tokens: Tokens,
) -> ParseResult<'a, Preprocessable<'a, StructProperty<'a>>, Tokens> {
    preprocessed_struct_properties(tokens).or_try(|| struct_property(tokens))
}

pub fn struct_property<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, Preprocessable<'a, StructProperty<'a>>, Tokens> {
    type_(tokens).determines(|tokens, type_| {
        let (tokens, name) = identifier(tokens)?;
        let (tokens, initializer) = var_initializer(tokens).maybe(tokens)?;
        let (tokens, comma) = tokens.terminal(TerminalToken::Comma).maybe(tokens)?;
        Ok((
            tokens,
            Preprocessable::UNCONDITIONAL(StructProperty {
                type_,
                name,
                initializer,
                comma,
            }),
        ))
    })
}

pub fn preprocessed_struct_properties<'a, Tokens: TokenIter<'a>>(
    tokens: Tokens,
) -> ParseResult<'a, Preprocessable<'a, StructProperty<'a>>, Tokens> {
    let (tokens, preprocessed) = preprocessed_if(tokens, |tokens| {
        tokens.many_until(
            preprocessed_if_contents_terminal,
            possibly_preprocessed_struct_property,
        )
    })?;
    Ok((tokens, Preprocessable::PREPROCESSED(preprocessed)))
}
