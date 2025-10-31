use crate::ast::{
    BlockStatement, BreakStatement, ClassDefinitionStatement, ConstDefinitionStatement,
    ConstructorDefinitionStatement, ContinueStatement, DelayThreadStatement, DoWhileStatement,
    EmptyStatement, EnumDefinitionStatement, ExpressionStatement, ForStatement, ForeachStatement,
    FunctionDefinitionStatement, GlobalStatement, GlobalizeAllFunctionsStatement, IfStatement,
    Precedence, PreprocessedDocumentation, PreprocessorIfExpression, ReturnStatement,
    SeparatedList1, Statement, StatementType, StructDefinitionStatement, SwitchStatement,
    ThreadStatement, ThrowStatement, TryCatchStatement, Type, TypeDefinitionStatement,
    UntypedStatement, VarDefinition, VarDefinitionStatement, WaitStatement,
    WaitThreadSoloStatement, WaitThreadStatement, WhileStatement, YieldStatement,
};
use crate::parser::class::class_definition;
use crate::parser::control::{
    for_definition, foreach_index, foreach_value, if_statement_type, switch_case,
};
use crate::parser::enum_::enum_entry;
use crate::parser::expression::{expression, var};
use crate::parser::function::function_definition;
use crate::parser::global::global_definition;
use crate::parser::identifier::identifier;
use crate::parser::parse_result_ext::ParseResultExt;
use crate::parser::struct_::struct_definition;
use crate::parser::token_list::TokenIter;
use crate::parser::token_list_ext::TokenListExt;
use crate::parser::type_::type_;
use crate::parser::variable::{var_definition, var_initializer};
use crate::parser::ParseResult;
use crate::token::{TerminalToken, Token, TokenType};
use crate::{ContextType, ParseErrorType};

use super::preprocessed::{preprocessed_if, preprocessed_if_contents_terminal};
use super::table::string_literal;

pub fn statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, Statement<'a>, Tokens> {
    let (next_tokens, statement) = inner_statement(tokens)?;

    // Statement must end with a semicolon, newline, or end of input.
    if statement.semicolon.is_some() || next_tokens.is_newline() || next_tokens.is_ended() {
        return Ok((next_tokens, statement));
    }

    // Statement can end if the last token was a `}`.
    if let Some(last_item) = next_tokens.previous() {
        if let TokenType::Terminal(TerminalToken::CloseBrace) = last_item.token.ty {
            return Ok((next_tokens, statement));
        }
    }

    // Statement can end if the last token is an empty statement.
    if let Some(last_item) = next_tokens.next() {
        if let TokenType::Empty = last_item.token.ty {
            return Ok((next_tokens, statement));
        }
    }

    //Errec: Statement can maybe end if the next token is a }?? i think? maybe? I'm scared
    //CHECK: _scripttest.gnut 639, does this work right??????
    if let Some((_, next_item)) = next_tokens.split_first() {
        if let TokenType::Terminal(TerminalToken::CloseBrace) = next_item.token.ty {
            return Ok((next_tokens, statement));
        }
    }

    Err(next_tokens.error_before(ParseErrorType::ExpectedEndOfStatement))
        .with_context_from(ContextType::Statement, tokens)
}

fn inner_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, Statement<'a>, Tokens> {
    // Handle a completely empty statement that just has a semicolon
    if let Ok((tokens, semicolon)) = tokens.terminal(TerminalToken::Semicolon) {
        return Ok((
            tokens,
            Statement {
                ty: StatementType::Empty(EmptyStatement { empty: None }),
                semicolon: Some(semicolon),
            },
        ));
    }

    let (tokens, ty) = statement_type(tokens)?;
    let (tokens, semicolon) = tokens.terminal(TerminalToken::Semicolon).maybe(tokens)?;

    Ok((tokens, Statement { ty, semicolon }))
}

pub fn statement_type<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, StatementType<'a>, Tokens> {
    // Look ahead to allow empty statements.
    if tokens.terminal(TerminalToken::Semicolon).is_ok() {
        return Ok((tokens, StatementType::Empty(EmptyStatement { empty: None })));
    }

    // An `empty` token can appear at the end of the token list.
    if let Some((tokens, empty)) = tokens.empty() {
        return Ok((
            tokens,
            StatementType::Empty(EmptyStatement { empty: Some(empty) }),
        ));
    }

    // `typed_function_or_var_definition_statement` must be first to ensure the return type is
    // parsed properly.
    typed_function_or_var_definition_statement(tokens)
        .or_try(|| block_statement(tokens).map_val(StatementType::Block))
        .or_try(|| if_statement(tokens).map_val(StatementType::If))
        .or_try(|| while_statement(tokens).map_val(StatementType::While))
        .or_try(|| do_while_statement(tokens).map_val(StatementType::DoWhile))
        .or_try(|| switch_statement(tokens).map_val(StatementType::Switch))
        .or_try(|| for_statement(tokens).map_val(StatementType::For))
        .or_try(|| foreach_statement(tokens).map_val(StatementType::Foreach))
        .or_try(|| try_catch_statement(tokens).map_val(StatementType::TryCatch))
        .or_try(|| break_statement(tokens).map_val(StatementType::Break))
        .or_try(|| continue_statement(tokens).map_val(StatementType::Continue))
        .or_try(|| return_statement(tokens).map_val(StatementType::Return))
        .or_try(|| yield_statement(tokens).map_val(StatementType::Yield))
        .or_try(|| throw_statement(tokens).map_val(StatementType::Throw))
        .or_try(|| const_definition_statement(tokens).map_val(StatementType::Const))
        .or_try(|| class_definition_statement(tokens).map_val(StatementType::ClassDefinition))
        .or_try(|| enum_definition_statement(tokens).map_val(StatementType::EnumDefinition))
        .or_try(|| void_function_definition_statement(tokens))
        .or_try(|| struct_definition_statement(tokens).map_val(StatementType::StructDefinition))
        .or_try(|| type_definition_statement(tokens).map_val(StatementType::TypeDefinition))
        .or_try(|| thread_statement(tokens).map_val(StatementType::Thread))
        .or_try(|| delay_thread_statement(tokens).map_val(StatementType::DelayThread))
        .or_try(|| wait_thread_statement(tokens).map_val(StatementType::WaitThread))
        .or_try(|| wait_thread_solo_statement(tokens).map_val(StatementType::WaitThreadSolo))
        .or_try(|| wait_statement(tokens).map_val(StatementType::Wait))
        .or_try(|| global_statement(tokens).map_val(StatementType::Global))
        .or_try(|| {
            globalize_all_functions_statement(tokens).map_val(StatementType::GlobalizeAllFunctions)
        })
        .or_try(|| untyped_statement(tokens).map_val(StatementType::Untyped))
        .or_try(|| preprocessed_if_statement(tokens).map_val(StatementType::Preprocessed))
        .or_try(|| {
            preprocessed_documentation_statement(tokens)
                .map_val(StatementType::PreprocessedDocumentation)
        })
        .or_try(|| expression_statement(tokens).map_val(StatementType::Expression))
        .with_context_from(ContextType::Statement, tokens)
        .or_error(|| tokens.error(ParseErrorType::ExpectedStatement))
}

pub fn expression_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ExpressionStatement<'a>, Tokens> {
    expression(tokens, Precedence::None)
        .map_val(|value| ExpressionStatement { value })
        .with_context_from(ContextType::Expression, tokens)
}

pub fn block_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, BlockStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::OpenBrace)
        .determines_and_opens(
            ContextType::BlockStatement,
            |tokens| tokens.terminal(TerminalToken::CloseBrace),
            |tokens| {
                tokens.many(statement)//Errec todo: This is bad and kind of a result of my changing things i shouldnt
            },
            |tokens, open, statements, close| {
                Ok((tokens, BlockStatement { open, statements, close }))
            },
        )
}

pub fn preprocessed_if_statement<'a, Tokens: TokenIter<'a>>(
    tokens: Tokens,
) -> ParseResult<'a, Box<PreprocessorIfExpression<'a, Vec<Statement<'a>>>>, Tokens> {
    preprocessed_if(tokens, |tokens| {
        tokens.many_until(preprocessed_if_contents_terminal, statement)
    })
}

pub fn preprocessed_documentation_statement<'a, Tokens: TokenIter<'a>>(
    tokens: Tokens,
) -> ParseResult<'a, PreprocessedDocumentation<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::PreprocessorDocument)
        .determines(|tokens, document| {
            let (
                tokens,
                (open, close, property, property_token, seperator, help_text, help_text_token),
            ) = tokens
                .terminal(TerminalToken::OpenBracket)
                .determines_and_opens(
                    ContextType::PreProcessorDocumentationStatement,
                    |tokens| tokens.terminal(TerminalToken::CloseBracket),
                    |tokens| {
                        let optional_class = var(tokens);
                        let tokens = match optional_class {
                            Ok((new_tokens, _)) => {
                                match new_tokens.terminal(TerminalToken::Comma) {
                                    Ok((new_tokens, _)) => new_tokens,
                                    Err(_) => tokens,
                                }
                            }
                            Err(_) => tokens,
                        };
                        let (tokens, (property, property_token)) = string_literal(tokens)?;
                        let (tokens, seperator) = tokens.terminal(TerminalToken::Comma)?;
                        let (tokens, (help_text, help_text_token)) = string_literal(tokens)?;
                        Ok((tokens, 
                            (property, property_token, seperator, help_text, help_text_token),
                        ))
                    },
                    |tokens, open, (property, property_token, seperator, help_text, help_text_token), close| {
                        Ok((
                            tokens,
                            (
                                open,
                                close,
                                property,
                                property_token,
                                seperator,
                                help_text,
                                help_text_token,
                            ),
                        ))
                    },
                )?;

            Ok((
                tokens,
                PreprocessedDocumentation {
                    document,
                    open,
                    property,
                    property_token,
                    seperator,
                    help_text,
                    help_text_token,
                    close,
                },
            ))
        })
}

pub fn if_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, IfStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::If)
        .determines(|tokens, if_| {
            let (tokens, (open, condition, close)) =
                tokens.terminal(TerminalToken::OpenBracket).opens(
                    ContextType::IfStatementCondition,
                    |tokens| tokens.terminal(TerminalToken::CloseBracket),
                    |tokens| {
                        expression(tokens, Precedence::None)
                    },
                    |tokens, open, condition, close| {
                        Ok((tokens, (open, condition, close)))
                    },
                )?;
            let (tokens, ty) = if_statement_type(tokens)?;

            Ok((
                tokens,
                IfStatement {
                    if_,
                    open,
                    condition,
                    close,
                    ty,
                },
            ))
        })
        .with_context_from(ContextType::IfStatement, tokens)
}

pub fn while_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, WhileStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::While)
        .determines(|tokens, while_| {
            let (tokens, (open, condition, close)) =
                tokens.terminal(TerminalToken::OpenBracket).opens(
                    ContextType::WhileStatementCondition,
                    |tokens| tokens.terminal(TerminalToken::CloseBracket),
                    |tokens| {
                        expression(tokens, Precedence::None)
                    },
                    |tokens, open, condition, close| {
                        Ok((tokens, (open, condition, close)))
                    },
                )?;
            let (tokens, body) = statement_type(tokens).replace_context_from(
                ContextType::BlockStatement,
                ContextType::Span,
                tokens,
            )?;

            Ok((
                tokens,
                WhileStatement {
                    while_,
                    open,
                    condition,
                    close,
                    body: Box::new(body),
                },
            ))
        })
        .with_context_from(ContextType::WhileStatement, tokens)
}

pub fn do_while_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, DoWhileStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Do)
        .determines(|tokens, do_| {
            let (tokens, body) = inner_statement(tokens).replace_context_from(
                ContextType::BlockStatement,
                ContextType::Span,
                tokens,
            )?;
            let (tokens, while_) = tokens.terminal(TerminalToken::While)?;

            let (tokens, (open, condition, close)) =
                tokens.terminal(TerminalToken::OpenBracket).opens(
                    ContextType::DoWhileStatementCondition,
                    |tokens| tokens.terminal(TerminalToken::CloseBracket),
                    |tokens| {
                        expression(tokens, Precedence::None)
                    },
                    |tokens, open, condition, close| {
                        Ok((tokens, (open, condition, close)))
                    },
                )?;

            Ok((
                tokens,
                DoWhileStatement {
                    do_,
                    body: Box::new(body),
                    while_,
                    open,
                    condition,
                    close,
                },
            ))
        })
        .with_context_from(ContextType::DoWhileStatement, tokens)
}

pub fn switch_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, SwitchStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Switch)
        .determines(|tokens, switch| {
            let (tokens, (open_condition, condition, close_condition)) =
                tokens.terminal(TerminalToken::OpenBracket).opens(
                    ContextType::SwitchStatementCondition,
                    |tokens| tokens.terminal(TerminalToken::CloseBracket),
                    |tokens| {
                        expression(tokens, Precedence::None)
                    },
                    |tokens, open, condition, close| {
                        Ok((tokens, (open, condition, close)))
                    }
                )?;

            let (tokens, (open_cases, cases, close_cases)) =
                tokens.terminal(TerminalToken::OpenBrace).opens(
                    ContextType::Span,
                    |tokens| tokens.terminal(TerminalToken::CloseBrace),
                    |tokens| {
                        tokens
                            .many(switch_case)
                    },
                    |tokens, open, cases, close| {
                        Ok((tokens, (open, cases, close)))
                    }
                )?;

            Ok((
                tokens,
                SwitchStatement {
                    switch,
                    open_condition,
                    condition,
                    close_condition,
                    open_cases,
                    cases,
                    close_cases,
                },
            ))
        })
        .with_context_from(ContextType::SwitchStatement, tokens)
}

pub fn for_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ForStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::For)
        .determines(|tokens, for_| {
            let (
                tokens,
                (open, initializer, semicolon_1, condition, semicolon_2, increment, close),
            ) = tokens.terminal(TerminalToken::OpenBracket).opens(
                ContextType::ForStatementCondition,
                |tokens| tokens.terminal(TerminalToken::CloseBracket),
                |tokens| {
                    let (tokens, initializer) = for_definition(tokens).maybe(tokens)?;
                    let (tokens, semicolon_1) = tokens.terminal(TerminalToken::Semicolon)?;
                    let (tokens, condition) = expression(tokens, Precedence::None).maybe(tokens)?;
                    let (tokens, semicolon_2) = tokens.terminal(TerminalToken::Semicolon)?;
                    let (tokens, increment) = expression(tokens, Precedence::None).maybe(tokens)?;
                    Ok((
                        tokens,
                        (initializer, semicolon_1, condition, semicolon_2, increment),
                    ))
                },
                |tokens, open, (initializer, semicolon_1, condition, semicolon_2, increment), close| {
                    Ok((
                        tokens,
                        (open, initializer, semicolon_1, condition, semicolon_2, increment, close),
                    ))
                },
            )?;

            let (tokens, body) = statement_type(tokens).replace_context_from(
                ContextType::BlockStatement,
                ContextType::Span,
                tokens,
            )?;

            Ok((
                tokens,
                ForStatement {
                    for_,
                    open,
                    initializer,
                    semicolon_1,
                    condition,
                    semicolon_2,
                    increment,
                    close,
                    body: Box::new(body),
                },
            ))
        })
        .with_context_from(ContextType::ForStatement, tokens)
}

pub fn foreach_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ForeachStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Foreach)
        .determines(|tokens, foreach| {
            let (tokens, (open, index, value_type, value_name, in_, array, close)) =
                tokens.terminal(TerminalToken::OpenBracket).opens(
                    ContextType::ForeachStatementCondition,
                    |tokens| tokens.terminal(TerminalToken::CloseBracket),
                    |tokens| {
                        let (tokens, index) = foreach_index(tokens).maybe(tokens)?;
                        let (tokens, (value_type, value_name, in_)) = foreach_value(tokens)?;
                        let (tokens, array) = expression(tokens, Precedence::None)?;

                        Ok((
                            tokens,
                            (index, value_type, value_name, in_, array),
                        ))
                    },
                    |tokens, open, (index, value_type, value_name, in_, array), close| {
                        Ok((
                            tokens,
                            (open, index, value_type, value_name, in_, array, close),
                        ))
                    },
                )?;

            let (tokens, body) = statement_type(tokens).replace_context_from(
                ContextType::BlockStatement,
                ContextType::Span,
                tokens,
            )?;

            Ok((
                tokens,
                ForeachStatement {
                    foreach,
                    open,
                    index,
                    value_type,
                    value_name,
                    in_,
                    array,
                    close,
                    body: Box::new(body),
                },
            ))
        })
        .with_context_from(ContextType::ForeachStatement, tokens)
}

pub fn try_catch_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, TryCatchStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Try)
        .determines(|tokens, try_| {
            let (tokens, body) = inner_statement(tokens).replace_context_from(
                ContextType::BlockStatement,
                ContextType::Span,
                tokens,
            )?;
            let (tokens, catch) = tokens.terminal(TerminalToken::Catch)?;
            let (tokens, (open, catch_name, close)) =
                tokens.terminal(TerminalToken::OpenBracket).opens(
                    ContextType::TryCatchStatementCatchName,
                    |tokens| tokens.terminal(TerminalToken::CloseBracket),
                    |tokens| {
                        identifier(tokens)
                    },
                    |tokens, open, catch_name, close| {
                        Ok((tokens, (open, catch_name, close)))
                    },
                )?;
            let (tokens, catch_body) = statement_type(tokens).replace_context_from(
                ContextType::BlockStatement,
                ContextType::Span,
                tokens,
            )?;

            Ok((
                tokens,
                TryCatchStatement {
                    try_,
                    body: Box::new(body),
                    catch,
                    open,
                    catch_name,
                    close,
                    catch_body: Box::new(catch_body),
                },
            ))
        })
}

pub fn break_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, BreakStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Break)
        .map_val(|break_| BreakStatement { break_ })
}

pub fn continue_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ContinueStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Continue)
        .map_val(|continue_| ContinueStatement { continue_ })
}

pub fn return_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ReturnStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Return)
        .determines(|tokens, return_| {
            // Value may appear after as long as it is on the same line.
            let (tokens, value) = if tokens.is_newline() {
                (tokens, None)
            } else {
                expression(tokens, Precedence::None).maybe(tokens)?
            };

            Ok((tokens, ReturnStatement { return_, value }))
        })
        .with_context_from(ContextType::ReturnStatement, tokens)
}

pub fn yield_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, YieldStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Yield)
        .determines(|tokens, yield_| {
            // Value may appear after as long as it is on the same line.
            let (tokens, value) = if tokens.is_newline() {
                (tokens, None)
            } else {
                expression(tokens, Precedence::None).maybe(tokens)?
            };

            Ok((tokens, YieldStatement { yield_, value }))
        })
        .with_context_from(ContextType::YieldStatement, tokens)
}

pub fn throw_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ThrowStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Throw)
        .determines(|tokens, throw| {
            expression(tokens, Precedence::None).map_val(|value| ThrowStatement { throw, value })
        })
        .with_context_from(ContextType::ThrowStatement, tokens)
}

pub fn const_definition_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ConstDefinitionStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Const)
        .determines(|tokens, const_| {
            untyped_const_definition_statement(tokens, const_)
                .or_try(|| typed_const_definition_statement(tokens, const_))
        })
        .with_context_from(ContextType::ConstDefinition, tokens)
}

fn untyped_const_definition_statement<'s, Tokens: TokenIter<'s>>(
    tokens: Tokens,
    const_: &'s Token<'s>,
) -> ParseResult<'s, ConstDefinitionStatement<'s>, Tokens> {
    let (tokens, name) = identifier(tokens)?;
    let (tokens, initializer) = var_initializer(tokens)?;
    Ok((
        tokens,
        ConstDefinitionStatement {
            const_,
            const_type: None,
            name,
            initializer,
        },
    ))
}

fn typed_const_definition_statement<'s, Tokens: TokenIter<'s>>(
    tokens: Tokens,
    const_: &'s Token<'s>,
) -> ParseResult<'s, ConstDefinitionStatement<'s>, Tokens> {
    let (tokens, const_type) = type_(tokens)?;
    let (tokens, name) = identifier(tokens)?;
    let (tokens, initializer) = var_initializer(tokens)?;
    Ok((
        tokens,
        ConstDefinitionStatement {
            const_,
            const_type: Some(const_type),
            name,
            initializer,
        },
    ))
}

pub fn class_definition_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ClassDefinitionStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Class)
        .determines(|tokens, class| {
            let (tokens, name) = expression(tokens, Precedence::None)?;
            let (tokens, definition) = class_definition(tokens)?;

            Ok((
                tokens,
                ClassDefinitionStatement {
                    class,
                    name,
                    definition,
                },
            ))
        })
        .with_context_from(ContextType::ClassDefinition, tokens)
}

pub fn enum_definition_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, EnumDefinitionStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Enum)
        .determines(|tokens, enum_| {
            let (tokens, name) = identifier(tokens)?;
            let (tokens, (open, entries, close)) =
                tokens.terminal(TerminalToken::OpenBrace).opens(
                    ContextType::Span,
                    |tokens| tokens.terminal(TerminalToken::CloseBrace),
                    |tokens| {
                        tokens
                            .many(enum_entry)
                    },
                    |tokens, open, entries, close| {
                        Ok((tokens, (open, entries, close)))
                    },
                )?;

            Ok((
                tokens,
                EnumDefinitionStatement {
                    enum_,
                    name,
                    open,
                    entries,
                    close,
                },
            ))
        })
        .with_context_from(ContextType::EnumDefinition, tokens)
}

pub fn typed_function_or_var_definition_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, StatementType<'a>, Tokens> {
    let (next_tokens, type_) = type_(tokens).not_definite()?;

    match (
        next_tokens.is_newline(),
        next_tokens.terminal(TerminalToken::Function),
    ) {
        // Linebreak between a function return type and the function definition is not allowed.
        (false, Ok((next_tokens, function))) => {
            typed_function_definition_statement(next_tokens, type_, function)
                .with_context_from(ContextType::FunctionDefinition, tokens)
                .map_val(StatementType::FunctionDefinition)
        }
        (true, Ok(_)) => Err(next_tokens.error(ParseErrorType::IllegalLineBreak)),

        // Linebreak between a variable type and the variable definition IS allowed. (??!)
        (_, Err(_)) => typed_var_definition_statement(next_tokens, type_)
            .with_context_from(ContextType::VarDefinition, tokens)
            .map_val(StatementType::VarDefinition),
    }
}

pub fn typed_function_definition_statement<'s, Tokens: TokenIter<'s>>(
    tokens: Tokens,
    return_type: Type<'s>,
    function: &'s Token<'s>,
) -> ParseResult<'s, FunctionDefinitionStatement<'s>, Tokens> {
    tokens
        .separated_list1(identifier, |tokens| {
            tokens.terminal(TerminalToken::Namespace)
        })
        .determines(|tokens, name| {
            let (tokens, definition) = function_definition(tokens)?;

            Ok((
                tokens,
                FunctionDefinitionStatement {
                    return_type: Some(return_type),
                    function,
                    name,
                    definition,
                },
            ))
        })
}

pub fn typed_var_definition_statement<'s, Tokens: TokenIter<'s>>(
    tokens: Tokens,
    type_: Type<'s>,
) -> ParseResult<'s, VarDefinitionStatement<'s>, Tokens> {
    identifier(tokens).determines(|tokens, name| {
        let (tokens, initializer) = var_initializer(tokens).maybe(tokens)?;
        let first_definition = VarDefinition { name, initializer };
        let (tokens, definitions) =
            tokens.separated_list_trailing1_init(first_definition, var_definition, |tokens| {
                tokens.terminal(TerminalToken::Comma)
            })?;

        Ok((tokens, VarDefinitionStatement { type_, definitions }))
    })
}

pub fn void_function_definition_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, StatementType<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Function)
        .and_then(|(tokens, function)| {
            identifier(tokens).map_val(|first_name| (function, first_name))
        })
        .determines(|tokens, (function, first_name)| {
            let (tokens, name) =
                tokens.separated_list_trailing1_init(first_name, identifier, |tokens| {
                    tokens.terminal(TerminalToken::Namespace)
                })?;

            // If the name has a trailing `::`, it must be followed by the constructor keyword.
            // This allows out-of-band constructor definitions, e.g.:
            // function MyClass::constructor() { ... }
            if let Some(last_namespace) = name.trailing {
                let (tokens, constructor) = tokens.terminal(TerminalToken::Constructor)?;
                let (tokens, definition) = function_definition(tokens)?;
                Ok((
                    tokens,
                    StatementType::ConstructorDefinition(ConstructorDefinitionStatement {
                        function,
                        namespaces: name.items,
                        last_name: *name.last_item,
                        last_namespace,
                        constructor,
                        definition,
                    }),
                ))
            } else {
                let (tokens, definition) = function_definition(tokens)?;
                Ok((
                    tokens,
                    StatementType::FunctionDefinition(FunctionDefinitionStatement {
                        return_type: None,
                        function,
                        name: SeparatedList1 {
                            items: name.items,
                            last_item: name.last_item,
                        },
                        definition,
                    }),
                ))
            }
        })
        .with_context_from(ContextType::FunctionDefinition, tokens)
}

pub fn struct_definition_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, StructDefinitionStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Struct)
        .determines(|tokens, struct_| {
            let (tokens, name) = identifier(tokens)?;
            let (tokens, definition) = struct_definition(tokens)?;
            Ok((
                tokens,
                StructDefinitionStatement {
                    struct_,
                    name,
                    definition,
                },
            ))
        })
        .with_context_from(ContextType::StructDefinition, tokens)
}

pub fn type_definition_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, TypeDefinitionStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Typedef)
        .determines(|tokens, typedef| {
            let (tokens, name) = identifier(tokens)?;
            let (tokens, type_) = type_(tokens)?;
            Ok((
                tokens,
                TypeDefinitionStatement {
                    typedef,
                    name,
                    type_,
                },
            ))
        })
        .with_context_from(ContextType::TypeDefinition, tokens)
}

pub fn thread_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, ThreadStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Thread)
        .determines(|tokens, thread| {
            expression(tokens, Precedence::None).map_val(|value| ThreadStatement { thread, value })
        })
        .with_context_from(ContextType::ThreadStatement, tokens)
}

pub fn delay_thread_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, DelayThreadStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::DelayThread)
        .determines(|tokens, delay_thread| {
            let (tokens, (open, duration, close)) =
                tokens.terminal(TerminalToken::OpenBracket).opens(
                    ContextType::Span,
                    |tokens| tokens.terminal(TerminalToken::CloseBracket),
                    |tokens| {
                        expression(tokens, Precedence::None)
                    },
                    |tokens, open, duration, close| {
                        Ok((tokens, (open, duration, close)))
                    },
                )?;
            let (tokens, value) = expression(tokens, Precedence::None)?;
            Ok((
                tokens,
                DelayThreadStatement {
                    delay_thread,
                    open,
                    duration,
                    close,
                    value,
                },
            ))
        })
        .with_context_from(ContextType::DelayThreadStatement, tokens)
}

pub fn wait_thread_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, WaitThreadStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::WaitThread)
        .determines(|tokens, wait_thread| {
            expression(tokens, Precedence::None)
                .map_val(|value| WaitThreadStatement { wait_thread, value })
        })
        .with_context_from(ContextType::WaitThreadStatement, tokens)
}

pub fn wait_thread_solo_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, WaitThreadSoloStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::WaitThreadSolo)
        .determines(|tokens, wait_thread_solo| {
            expression(tokens, Precedence::None).map_val(|value| WaitThreadSoloStatement {
                wait_thread_solo,
                value,
            })
        })
        .with_context_from(ContextType::WaitThreadSoloStatement, tokens)
}

pub fn wait_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, WaitStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Wait)
        .determines(|tokens, wait| {
            expression(tokens, Precedence::None).map_val(|value| WaitStatement { wait, value })
        })
        .with_context_from(ContextType::WaitStatement, tokens)
}

pub fn global_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, GlobalStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Global)
        .determines(|tokens, global| {
            global_definition(tokens).map_val(|definition| GlobalStatement { global, definition })
        })
        .with_context_from(ContextType::GlobalStatement, tokens)
}

pub fn globalize_all_functions_statement<'a, Tokens: TokenIter<'a>>(
    tokens: Tokens,
) -> ParseResult<'a, GlobalizeAllFunctionsStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::GlobalizeAllFunctions)
        .map_val(|globalize_all_functions| GlobalizeAllFunctionsStatement {
            globalize_all_functions,
        })
}

pub fn untyped_statement<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, UntypedStatement<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Untyped)
        .map_val(|untyped| UntypedStatement { untyped })
}
