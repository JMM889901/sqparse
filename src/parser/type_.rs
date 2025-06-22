use crate::ast::{
    ArrayType, FunctionRefType, GenericType, LocalType, NullableType, PlainType, Precedence,
    ReferenceType, StructType, Type,
};
use crate::parser::expression::expression;
use crate::parser::function::function_ref_param;
use crate::parser::identifier::identifier;
use crate::parser::parse_result_ext::ParseResultExt;
use crate::parser::struct_::struct_definition;
use crate::parser::token_list::TokenIter;
use crate::parser::token_list_ext::TokenListExt;
use crate::parser::ParseResult;
use crate::token::TerminalToken;
use crate::{ContextType, Flavor, ParseErrorType};

pub fn type_<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, Type<'a>, Tokens> {
    // Only SquirrelRespawn supports types. Other flavors only support `local` and `var`.
    if tokens.flavor() != Flavor::SquirrelRespawn {
        return local(tokens).map_val(Type::Local);
    }

    let (mut next_tokens, mut type_) = base(tokens)?;

    loop {
        let mut type_container = Some(type_);
        match modifier(next_tokens, TypeRef(&mut type_container))
            .with_context_from(ContextType::Type, tokens)
            .maybe(next_tokens)?
        {
            (new_tokens, Some(new_type)) => {
                next_tokens = new_tokens;
                type_ = new_type;
            }
            (new_tokens, None) => return Ok((new_tokens, type_container.unwrap())),
        }
    }
}

fn base<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, Type<'a>, Tokens> {
    local(tokens)
        .map_val(Type::Local)
        .or_try(|| plain(tokens).map_val(Type::Plain))
        .or_try(|| void_function_ref(tokens).map_val(Type::FunctionRef))
        .or_try(|| struct_(tokens).map_val(Type::Struct))
        .or_error(|| tokens.error(ParseErrorType::ExpectedType))
}

pub fn local<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, LocalType<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Local)
        .map_val(|local| LocalType { local })
}

pub fn plain<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, PlainType<'a>, Tokens> {
    identifier(tokens).map_val(|name| PlainType { name })
}

pub fn void_function_ref<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, FunctionRefType<'a>, Tokens> {
    function_ref(tokens, || None)
}

pub fn struct_<'a, Tokens: TokenIter<'a>>(tokens: Tokens) -> ParseResult<'a, StructType<'a>, Tokens> {
    tokens
        .terminal(TerminalToken::Struct)
        .determines(|tokens, struct_| {
            struct_definition(tokens).map_val(|definition| StructType {
                struct_,
                definition,
            })
        })
}

struct TypeRef<'a, 's>(&'a mut Option<Type<'s>>);
impl<'s> TypeRef<'_, 's> {
    fn take(self) -> Type<'s> {
        self.0.take().unwrap()
    }
}

fn modifier<'s, Tokens: TokenIter<'s>>(tokens: Tokens, left: TypeRef<'_, 's>) -> ParseResult<'s, Type<'s>, Tokens> {
    let left_ref = left.0;

    array(tokens, TypeRef(left_ref))
        .map_val(Type::Array)
        .or_try(|| generic(tokens, TypeRef(left_ref)).map_val(Type::Generic))
        .or_try(|| typed_function_ref(tokens, TypeRef(left_ref)).map_val(Type::FunctionRef))
        .or_try(|| reference(tokens, TypeRef(left_ref)).map_val(Type::Reference))
        .or_try(|| nullable(tokens, TypeRef(left_ref)).map_val(Type::Nullable))
        .or_error(|| tokens.error(ParseErrorType::ExpectedTypeModifier))
}

fn array<'s, Tokens: TokenIter<'s>>(tokens: Tokens, left: TypeRef<'_, 's>) -> ParseResult<'s, ArrayType<'s>, Tokens> {
    tokens.terminal(TerminalToken::OpenSquare).opens(
        ContextType::Expression,
        |tokens| tokens.terminal(TerminalToken::CloseSquare),
        |tokens, open, close| {
            expression(tokens, Precedence::None).map_val(|len| ArrayType {
                base: Box::new(left.take()),
                open,
                len,
                close,
            })
        },
    )
}

fn generic<'s, Tokens: TokenIter<'s>>(tokens: Tokens, left: TypeRef<'_, 's>) -> ParseResult<'s, GenericType<'s>, Tokens> {
    tokens
        .terminal(TerminalToken::Less)
        .determines(|tokens, open| {
            let (tokens, params) = tokens.separated_list_trailing1(
                |tokens| {
                    let res = type_(tokens);
                    if tokens.terminal(TerminalToken::Greater).is_ok() {
                        res
                    } else {
                        res.definite()
                    }
                },
                |tokens| tokens.terminal(TerminalToken::Comma),
            )?;
            let (tokens, close) = tokens.terminal(TerminalToken::Greater)?;
            Ok((
                tokens,
                GenericType {
                    base: Box::new(left.take()),
                    open,
                    params,
                    close,
                },
            ))
        })
        .with_context_from(ContextType::GenericArgumentList, tokens)
}

fn typed_function_ref<'s, Tokens: TokenIter<'s>>(
    tokens: Tokens,
    left: TypeRef<'_, 's>,
) -> ParseResult<'s, FunctionRefType<'s>, Tokens> {
    function_ref(tokens, || Some(left.take()))
}

fn reference<'s, Tokens: TokenIter<'s>>(
    tokens: Tokens,
    left: TypeRef<'_, 's>,
) -> ParseResult<'s, ReferenceType<'s>, Tokens> {
    tokens
        .terminal(TerminalToken::BitwiseAnd)
        .map_val(|reference| ReferenceType {
            base: Box::new(left.take()),
            reference,
        })
}

fn nullable<'s, Tokens: TokenIter<'s>>(tokens: Tokens, left: TypeRef<'_, 's>) -> ParseResult<'s, NullableType<'s>, Tokens> {
    tokens
        .terminal(TerminalToken::OrNull)
        .map_val(|ornull| NullableType {
            base: Box::new(left.take()),
            ornull,
        })
}

fn function_ref<'s, Tokens: TokenIter<'s>>(
    tokens: Tokens,
    return_type: impl FnOnce() -> Option<Type<'s>>,
) -> ParseResult<'s, FunctionRefType<'s>, Tokens> {
    tokens
        .terminal(TerminalToken::FunctionRef)
        .determines(|tokens, functionref| {
            let (tokens, (open, params, close)) =
                tokens.terminal(TerminalToken::OpenBracket).opens(
                    ContextType::Span,
                    |tokens| tokens.terminal(TerminalToken::CloseBracket),
                    |tokens, open, close| {
                        tokens
                            .separated_list_trailing0(function_ref_param, |tokens| {
                                tokens.terminal(TerminalToken::Comma)
                            })
                            .map_val(|params| (open, params, close))
                    },
                )?;

            Ok((
                tokens,
                FunctionRefType {
                    return_type: return_type().map(Box::new),
                    functionref,
                    open,
                    params,
                    close,
                },
            ))
        })
}
