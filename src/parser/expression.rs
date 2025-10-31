use crate::ast::{
    ArrayExpression, BinaryExpression, CallExpression, ClassExpression, CommaExpression,
    DelegateExpression, ExpectExpression, Expression, FunctionExpression, IndexExpression,
    LambdaExpression, LiteralExpression, ParensExpression, PostfixExpression, Precedence,
    PrefixExpression, PreprocessorIfExpression, PropertyExpression, RootVarExpression,
    TableExpression, TernaryExpression, VarExpression, VectorExpression,
};
use crate::parser::class::class_definition;
use crate::parser::function::{call_argument, function_definition, function_params};
use crate::parser::identifier::{identifier, method_identifier};
use crate::parser::operator::{binary_operator, postfix_operator, prefix_operator};
use crate::parser::parse_result_ext::ParseResultExt;
use crate::parser::token_list::TokenIter;
use crate::parser::token_list_ext::TokenListExt;
use crate::parser::type_::type_;
use crate::parser::ParseResult;
use crate::token::{TerminalToken, TokenType};
use crate::{ContextType, ParseErrorType};

use super::array::possibly_preprocessed_array_value;
use super::preprocessed::preprocessed_if;
use super::table::possibly_preprocessed_table_slot;

pub fn expression<'a, Tokens: TokenIter<'a>>(tokens: Tokens, precedence: Precedence) -> ParseResult<'a, Box<Expression<'a>>, Tokens> {
    let (mut next_tokens, mut value) = value(tokens)?;

    loop {
        let mut value_container = Some(value);
        match operator(next_tokens, precedence, ExpressionRef(&mut value_container))
            .with_context_from(ContextType::Expression, tokens)
            .maybe(next_tokens)?
        {
            (new_tokens, Some(new_value)) => {
                next_tokens = new_tokens;
                value = new_value;
            }
            (new_tokens, None) => return Ok((new_tokens, value_container.unwrap())),
        }
    }
}

fn value<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, Box<Expression<'a>>, Tokens> {
    function(tokens)
        .map_val(Expression::Function)
        .or_try(|| parens(tokens).map_val(Expression::Parens))
        .or_try(|| literal(tokens).map_val(Expression::Literal))
        .or_try(|| var(tokens).map_val(Expression::Var))
        .or_try(|| root_var(tokens).map_val(Expression::RootVar))
        .or_try(|| table(tokens).map_val(Expression::Table))
        .or_try(|| class(tokens).map_val(Expression::Class))
        .or_try(|| array(tokens).map_val(Expression::Array))
        .or_try(|| vector(tokens).map_val(Expression::Vector))
        .or_try(|| prefix(tokens).map_val(Expression::Prefix))
        .or_try(|| delegate(tokens).map_val(Expression::Delegate))
        .or_try(|| expect(tokens).map_val(Expression::Expect))
        .or_try(|| lambda(tokens).map_val(Expression::Lambda))
        .or_try(|| preprocessed_if_expression(tokens).map_val(Expression::Preprocessed))
        .or_error(|| tokens.error(ParseErrorType::ExpectedValue))
        .map_val(Box::new)
}

pub fn preprocessed_if_expression<'a, Tokens: TokenIter<'a>>(
    tokens: Tokens,
) -> ParseResult<'a, Box<PreprocessorIfExpression<'a, Box<Expression<'a>>>>, Tokens> {
    preprocessed_if(tokens, |tokens| expression(tokens, Precedence::None))
}

pub fn function<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, FunctionExpression<'a>, Tokens> {
    type_(tokens)
        .not_line_ending()
        .maybe(tokens)
        .and_then(|(tokens, return_type)| {
            tokens
                .terminal(TerminalToken::Function)
                .map_val(|function| (return_type, function))
        })
        .determines(|tokens, (return_type, function)| {
            function_definition(tokens).map_val(|definition| FunctionExpression {
                return_type,
                function,
                definition,
            })
        })
        .with_context_from(ContextType::FunctionLiteral, tokens)
}

pub fn parens<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ParensExpression<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::OpenBracket)
        .determines_and_opens(
            ContextType::Expression,
            |tokens| tokens.terminal(TerminalToken::CloseBracket),
            |tokens| {
                expression(tokens, Precedence::None)
            },
            |tokens, open, value, close| {
                Ok((tokens, ParensExpression { open, value, close }))
            },
        )
}

pub fn literal<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, LiteralExpression<'a>, Tokens> {
    if let Some((tokens, item)) = tokens.split_first() {
        if let TokenType::Literal(literal) = item.token.ty {
            return Ok((
                tokens,
                LiteralExpression {
                    literal,
                    token: &item.token,
                },
            ));
        }
    }

    Err(tokens.error(ParseErrorType::ExpectedLiteral))
}

pub fn var<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, VarExpression<'a>, Tokens> {
    identifier(tokens).map_val(|name| VarExpression { name })
}

pub fn root_var<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, RootVarExpression<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Namespace)
        .determines(|tokens, root| {
            identifier(tokens).map_val(|name| RootVarExpression { root, name })
        })
}

pub fn table<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, TableExpression<'a>, Tokens> {
    table_delimited(tokens, TerminalToken::OpenBrace, TerminalToken::CloseBrace)
}

pub fn table_delimited<'a, Tokens: TokenIter<'a>>(
    tokens: Tokens,
    open_terminal: TerminalToken,
    close_terminal: TerminalToken,
) -> ParseResult<'a, TableExpression<'a>, Tokens> {
    tokens.terminal(open_terminal).determines_and_opens(
        ContextType::TableLiteral,
        |tokens| tokens.terminal(close_terminal),
        |tokens| {
            let (tokens, slots) = tokens.many_until(
                |tokens| tokens.terminal(close_terminal).is_ok() || tokens.terminal(TerminalToken::Ellipsis).is_ok(),
                // table_slot,
                possibly_preprocessed_table_slot,
            )?;
            let (tokens, spread) = tokens.terminal(TerminalToken::Ellipsis).maybe(tokens)?;
            Ok((tokens, (slots, spread)))
        },
        |tokens, open, (slots, spread), close| {
            Ok((
                tokens,
                TableExpression {
                    open,
                    slots,
                    spread,
                    close,
                },
            ))
        },
    )
}

pub fn class<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ClassExpression<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Class)
        .determines(|tokens, class| {
            class_definition(tokens).map_val(|definition| ClassExpression { class, definition })
        })
        .with_context_from(ContextType::ClassLiteral, tokens)
}

pub fn array<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ArrayExpression<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::OpenSquare)
        .determines_and_opens(
            ContextType::ArrayLiteral,
            |tokens| tokens.terminal(TerminalToken::CloseSquare),
            |tokens| {
                let (tokens, values) = tokens.many_until(
                    |tokens| tokens.terminal(TerminalToken::CloseSquare).is_ok() || tokens.terminal(TerminalToken::Ellipsis).is_ok(),
                    possibly_preprocessed_array_value,
                )?;
                let (tokens, spread) = tokens.terminal(TerminalToken::Ellipsis).maybe(tokens)?;
                Ok((
                    tokens,
                    (values, spread),
                ))
            },
            |tokens, open, (values, spread), close| {
                Ok((
                    tokens,
                    ArrayExpression {
                        open,
                        values,
                        spread,
                        close,
                    },
                ))
            },
        )
}

pub fn vector<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, VectorExpression<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Less)
        .determines(|tokens, open| {
            let (tokens, x) = expression(tokens, Precedence::Comma)?;
            let (tokens, comma_1) = tokens.terminal(TerminalToken::Comma)?;
            let (tokens, y) = expression(tokens, Precedence::Comma)?;
            let (tokens, comma_2) = tokens.terminal(TerminalToken::Comma)?;
            let (tokens, z) = expression(tokens, Precedence::Bitshift)?;
            let (tokens, close) = tokens.terminal(TerminalToken::Greater)?;
            Ok((
                tokens,
                VectorExpression {
                    open,
                    x,
                    comma_1,
                    y,
                    comma_2,
                    z,
                    close,
                },
            ))
        })
        .with_context_from(ContextType::VectorLiteral, tokens)
}

pub fn prefix<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, PrefixExpression<'a>, Tokens> {
    prefix_operator(tokens).determines(|tokens, operator| {
        expression(tokens, Precedence::Prefix).map_val(|value| PrefixExpression { operator, value })
    })
}

pub fn delegate<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, DelegateExpression<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Delegate)
        .determines(|tokens, delegate| {
            let (tokens, parent) = expression(tokens, Precedence::None)?;
            let (tokens, colon) = tokens.terminal(TerminalToken::Colon)?;
            let (tokens, value) = expression(tokens, Precedence::Comma)?;
            Ok((
                tokens,
                DelegateExpression {
                    delegate,
                    parent,
                    colon,
                    value,
                },
            ))
        })
}

pub fn expect<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ExpectExpression<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Expect)
        .determines(|tokens, expect| {
            let (tokens, ty) = type_(tokens)?;
            tokens.terminal(TerminalToken::OpenBracket).opens(
                ContextType::Expression,
                |tokens| tokens.terminal(TerminalToken::CloseBracket),
                |tokens| {
                    expression(tokens, Precedence::None)
                },
                |tokens, open, value, close| {
                    Ok((
                        tokens,
                        ExpectExpression {
                            expect,
                            ty,
                            open,
                            value,
                            close,
                        },
                    ))
                }
            )
        })
}

pub fn lambda<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, LambdaExpression<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::At)
        .determines(|tokens, at| {
            let (tokens, (open, params, close)) =
                tokens.terminal(TerminalToken::OpenBracket).opens(
                    ContextType::FunctionParamList,
                    |tokens| tokens.terminal(TerminalToken::CloseBracket),
                    |tokens| {
                        function_params(tokens)
                    },
                    |tokens, open, params, close| {
                        Ok((tokens, (open, params, close)))
                    },
                )?;
            let (tokens, value) = expression(tokens, Precedence::Comma)?;
            Ok((
                tokens,
                LambdaExpression {
                    at,
                    open,
                    params,
                    close,
                    value,
                },
            ))
        })
        .with_context_from(ContextType::LambdaLiteral, tokens)
}

struct ExpressionRef<'a, 's>(&'a mut Option<Box<Expression<'s>>>);
impl<'s> ExpressionRef<'_, 's> {
    fn take(self) -> Box<Expression<'s>> {
        self.0.take().unwrap()
    }
}

fn operator<'s, Tokens: TokenIter<'s>>(
    tokens: Tokens,
    precedence: Precedence,
    left: ExpressionRef<'_, 's>,
) -> ParseResult<'s, Box<Expression<'s>>, Tokens> {
    let left_ref = left.0;

    property(tokens, precedence, ExpressionRef(left_ref))
        .map_val(Expression::Property)
        .or_try(|| {
            ternary(tokens, precedence, ExpressionRef(left_ref)).map_val(Expression::Ternary)
        })
        .or_try(|| binary(tokens, precedence, ExpressionRef(left_ref)).map_val(Expression::Binary))
        .or_try(|| index(tokens, precedence, ExpressionRef(left_ref)).map_val(Expression::Index))
        .or_try(|| {
            postfix(tokens, precedence, ExpressionRef(left_ref)).map_val(Expression::Postfix)
        })
        .or_try(|| call(tokens, precedence, ExpressionRef(left_ref)).map_val(Expression::Call))
        .or_try(|| comma(tokens, precedence, ExpressionRef(left_ref)).map_val(Expression::Comma))
        .or_error(|| tokens.error(ParseErrorType::ExpectedOperator))
        .map_val(Box::new)
}

fn property<'s, Tokens: TokenIter<'s>>(
    tokens: Tokens,
    precedence: Precedence,
    left: ExpressionRef<'_, 's>,
) -> ParseResult<'s, PropertyExpression<'s>, Tokens> {
    // left associative
    if precedence >= Precedence::Property {
        return Err(tokens.error(ParseErrorType::Precedence));
    }

    tokens
        .terminal(TerminalToken::Dot)
        .determines(|tokens, dot| {
            method_identifier(tokens).map_val(|property| PropertyExpression {
                base: left.take(),
                dot,
                property,
            })
        })
}

fn ternary<'s, Tokens: TokenIter<'s>>(
    tokens: Tokens,
    precedence: Precedence,
    left: ExpressionRef<'_, 's>,
) -> ParseResult<'s, TernaryExpression<'s>, Tokens> {
    // right associative
    if precedence > Precedence::Ternary {
        return Err(tokens.error(ParseErrorType::Precedence));
    }

    tokens
        .terminal(TerminalToken::Question)
        .determines(|tokens, question| {
            let (tokens, true_value) = expression(tokens, Precedence::None)?;
            let (tokens, separator) = tokens.terminal(TerminalToken::Colon)?;
            let (tokens, false_value) = expression(tokens, Precedence::Ternary)?;

            Ok((
                tokens,
                TernaryExpression {
                    condition: left.take(),
                    question,
                    true_value,
                    separator,
                    false_value,
                },
            ))
        })
}

fn binary<'s, Tokens: TokenIter<'s>>(
    tokens: Tokens,
    precedence: Precedence,
    left: ExpressionRef<'_, 's>,
) -> ParseResult<'s, BinaryExpression<'s>, Tokens> {
    binary_operator(tokens)
        .and_then(|(tokens, operator)| {
            // left associative
            if precedence >= operator.precedence() {
                Err(tokens.error(ParseErrorType::Precedence))
            } else {
                Ok((tokens, operator))
            }
        })
        .determines(|tokens, operator| {
            expression(tokens, operator.precedence()).map_val(|right| BinaryExpression {
                left: left.take(),
                operator,
                right,
            })
        })
}

fn index<'s, Tokens: TokenIter<'s>>(
    tokens: Tokens,
    precedence: Precedence,
    left: ExpressionRef<'_, 's>,
) -> ParseResult<'s, IndexExpression<'s>, Tokens> {
    // left associative
    if precedence >= Precedence::Postfix {
        return Err(tokens.error(ParseErrorType::Precedence));
    }

    tokens
        .terminal(TerminalToken::OpenSquare)
        .determines_and_opens(
            ContextType::Expression,
            |tokens| tokens.terminal(TerminalToken::CloseSquare),
            |tokens| {
                expression(tokens, Precedence::None)
            },
            |tokens, open, value, close| {
                Ok((
                    tokens,
                    IndexExpression {
                        base: left.take(),
                        open,
                        index: value,
                        close,
                    },
                ))
            },
        )
}

fn postfix<'s, Tokens: TokenIter<'s>>(
    tokens: Tokens,
    precedence: Precedence,
    left: ExpressionRef<'_, 's>,
) -> ParseResult<'s, PostfixExpression<'s>, Tokens> {
    // left associative
    if precedence >= Precedence::Postfix {
        return Err(tokens.error(ParseErrorType::Precedence));
    }

    // Newlines are not allowed before postfix operators to prevent this:
    // ```
    // a
    // ++b
    // ```
    // from being parsed as:
    // ```
    // (a++) b
    // ```
    if tokens.is_newline() {
        return Err(tokens.error(ParseErrorType::IllegalLineBreak));
    }

    postfix_operator(tokens)
        .not_definite()
        .map_val(|operator| PostfixExpression {
            value: left.take(),
            operator,
        })
}

fn call<'s, Tokens: TokenIter<'s>>(
    tokens: Tokens,
    precedence: Precedence,
    left: ExpressionRef<'_, 's>,
) -> ParseResult<'s, CallExpression<'s>, Tokens> {
    // left associative
    if precedence >= Precedence::Postfix {
        return Err(tokens.error(ParseErrorType::Precedence));
    }

    tokens
        .terminal(TerminalToken::OpenBracket)
        .determines_and_opens(
            ContextType::CallArgumentList,
            |tokens| tokens.terminal(TerminalToken::CloseBracket),
            |tokens| {
                tokens
                    .many(call_argument)
            },
            |tokens, open, arguments, close| {
                Ok((
                    tokens,
                    (open, arguments, close),
                ))
            },
        )
        .and_then(|(tokens, (open, arguments, close))| {
            // Post-initializer may appear after a call, as long as it is on the same line.
            let (tokens, post_initializer) = if tokens.is_newline() {
                (tokens, None)
            } else {
                table(tokens).maybe(tokens)?
            };

            Ok((
                tokens,
                CallExpression {
                    function: left.take(),
                    open,
                    arguments,
                    close,
                    post_initializer,
                },
            ))
        })
}

fn comma<'s, Tokens: TokenIter<'s>>(
    tokens: Tokens,
    precedence: Precedence,
    left: ExpressionRef<'_, 's>,
) -> ParseResult<'s, CommaExpression<'s>, Tokens> {
    // left associative
    if precedence >= Precedence::Comma {
        return Err(tokens.error(ParseErrorType::Precedence));
    }

    tokens
        .terminal(TerminalToken::Comma)
        .determines(|tokens, first_comma| {
            let (tokens, mut values) = tokens.separated_list1(
                |tokens| expression(tokens, Precedence::Comma).map_val(|expr| *expr),
                |tokens| tokens.terminal(TerminalToken::Comma),
            )?;
            values.items.insert(0, (*left.take(), first_comma));
            Ok((tokens, CommaExpression { values }))
        })
}
