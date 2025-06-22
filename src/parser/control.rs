use crate::ast::{
    ForDefinition, ForeachIndex, Identifier, IfStatementType, Precedence, Statement, SwitchCase,
    SwitchCaseCondition, Type,
};
use crate::parser::expression::expression;
use crate::parser::identifier::identifier;
use crate::parser::parse_result_ext::ParseResultExt;
use crate::parser::statement::{statement, statement_type, typed_var_definition_statement};
use crate::parser::token_list::{TokenIter};
use crate::parser::token_list_ext::TokenListExt;
use crate::parser::type_::type_;
use crate::parser::ParseResult;
use crate::token::{TerminalToken, Token};
use crate::ContextType;

pub fn if_statement_type<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, IfStatementType<'a>, Tokens> {
    let (tokens, body) = statement_type(tokens).maybe(tokens)?;

    if body.is_none() {
        return Ok((tokens, IfStatementType::NoElseTailless));
    }

    // Consume a `;` if this is followed by an `else`.
    let (next_tokens, semicolon) = tokens.terminal(TerminalToken::Semicolon).maybe(tokens)?;

    let Ok((tokens, else_)) = next_tokens.terminal(TerminalToken::Else) else {
        // Return the body WITHOUT consuming the `;`.
        return Ok((tokens, IfStatementType::NoElse { body: Box::new(body.unwrap()) }))
    };

    let body = Box::new(Statement {
        ty: body.unwrap(),
        semicolon,
    });

    // let (tokens, else_body) = statement_type(tokens).definite()?;
    let (tokens, else_body) = statement_type(tokens).maybe(tokens)?;
    if let Some(else_body) = else_body {
        return Ok((
            tokens,
            IfStatementType::Else {
                body,
                else_,
                else_body: Box::new(else_body),
            },
        ));
    }
    Ok((tokens, IfStatementType::ElseTailless { body, else_ }))
}

pub fn switch_case<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, SwitchCase<'a>, Tokens> {
    switch_case_condition(tokens).determines(|tokens, condition| {
        let (tokens, colon) = tokens.terminal(TerminalToken::Colon)?;
        let (tokens, body) = tokens.many(statement)?;
        Ok((
            tokens,
            SwitchCase {
                condition,
                colon,
                body,
            },
        ))
    })
}

pub fn switch_case_condition<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, SwitchCaseCondition<'a>, Tokens> {
    default_switch_case_condition(tokens).or_try(|| case_switch_case_condition(tokens))
}

fn default_switch_case_condition<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, SwitchCaseCondition<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Default)
        .map_val(|default| SwitchCaseCondition::Default { default })
}

fn case_switch_case_condition<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, SwitchCaseCondition<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Case)
        .determines(|tokens, case| {
            let (tokens, value) = expression(tokens, Precedence::None)?;
            Ok((tokens, SwitchCaseCondition::Case { case, value }))
        })
}

pub fn for_definition<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ForDefinition<'a>, Tokens> {
    var_for_definition(tokens).or_try(|| expression_for_definition(tokens))
}

fn var_for_definition<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ForDefinition<'a>, Tokens> {
    type_(tokens)
        .not_definite()
        .and_then(|(tokens, type_)| typed_var_definition_statement(tokens, type_))
        .with_context_from(ContextType::VarDefinition, tokens)
        .map_val(ForDefinition::Definition)
}

fn expression_for_definition<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ForDefinition<'a>, Tokens> {
    expression(tokens, Precedence::None)
        .with_context_from(ContextType::Expression, tokens)
        .map_val(ForDefinition::Expression)
}

pub fn foreach_index<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ForeachIndex<'a>, Tokens> {
    untyped_foreach_index(tokens).or_try(|| typed_foreach_index(tokens))
}

fn untyped_foreach_index<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ForeachIndex<'a>, Tokens> {
    let (tokens, name) = identifier(tokens)?;
    let (tokens, comma) = tokens.terminal(TerminalToken::Comma)?;
    Ok((
        tokens,
        ForeachIndex {
            type_: None,
            name,
            comma,
        },
    ))
}

fn typed_foreach_index<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ForeachIndex<'a>, Tokens> {
    let (tokens, type_) = type_(tokens)?;
    let (tokens, name) = identifier(tokens)?;
    let (tokens, comma) = tokens.terminal(TerminalToken::Comma)?;
    Ok((
        tokens,
        ForeachIndex {
            type_: Some(type_),
            name,
            comma,
        },
    ))
}

pub fn foreach_value<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, (Option<Type<'a>>, Identifier<'a>, &'a Token<'a>), Tokens> {
    untyped_foreach_value(tokens).or_try(|| typed_foreach_value(tokens))
}

fn typed_foreach_value<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, (Option<Type<'a>>, Identifier<'a>, &'a Token<'a>), Tokens> {
    type_(tokens)
        .and_then(|(tokens, type_)| identifier(tokens).map_val(|name| (type_, name)))
        .determines(|tokens, (type_, name)| {
            let (tokens, in_) = tokens.terminal(TerminalToken::In)?;
            Ok((tokens, (Some(type_), name, in_)))
        })
}

fn untyped_foreach_value<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, (Option<Type<'a>>, Identifier<'a>, &'a Token<'a>), Tokens> {
    let (tokens, name) = identifier(tokens)?;
    let (tokens, in_) = tokens.terminal(TerminalToken::In)?;
    Ok((tokens, (None, name, in_)))
}
