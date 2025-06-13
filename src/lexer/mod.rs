use crate::lexer::token_iter::TokenIter;
use crate::token::{TerminalToken, Token, TokenType};
use crate::Flavor;
use std::collections::VecDeque;

mod comment;
mod error;
mod identifier;
mod literal;
mod parse_str;
mod symbol;
mod token_iter;

pub use self::error::{LexerError, LexerErrorType};

/// A token with attached metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenItem<'s> {
    /// The actual token.
    pub token: Token<'s>,

    /// The index of the corresponding closing delimiter token, if this token is an opening
    /// delimiter.
    ///
    /// # Example
    /// ```text
    /// { some other tokens }
    /// ^ open              ^ close
    /// ```
    /// In this example, the opening `{` token would have a `close_index` of 5, the index of the
    /// closing delimiter.
    pub close_index: Option<usize>,
}

// Returns the token that closes a tree, if the provided token is a valid opening token.
fn closing_token(opening: TokenType) -> Option<TokenType> {
    match opening {
        TokenType::Terminal(TerminalToken::OpenBrace) => {
            Some(TokenType::Terminal(TerminalToken::CloseBrace))
        }
        TokenType::Terminal(TerminalToken::OpenSquare) => {
            Some(TokenType::Terminal(TerminalToken::CloseSquare))
        }
        TokenType::Terminal(TerminalToken::OpenBracket) => {
            Some(TokenType::Terminal(TerminalToken::CloseBracket))
        }
        TokenType::Terminal(TerminalToken::OpenAttributes) => {
            Some(TokenType::Terminal(TerminalToken::CloseAttributes))
        }
		TokenType::Terminal(TerminalToken::PreprocessorIf) => {
			Some(TokenType::Terminal(TerminalToken::PreprocessorEndIf))
		}
        _ => None,
    }
}

struct Layer<'s> {
    open_index: usize,
    close_ty: TokenType<'s>,
}
//type TokenizeFailure<'s> = (Vec<TokenItem<'s>>, Vec<LexerError<'s>>);
#[derive(Debug, Clone)]
pub struct TokenizeFailure<'s>(pub Vec<TokenItem<'s>>,pub Vec<LexerError<'s>>);
impl<'s> TokenizeFailure<'s> {
    pub fn items(&self) -> &Vec<TokenItem<'s>> {
        &self.0
    }

    pub fn errors(&self) -> &[LexerError<'s>] {
        &self.1
    }
    pub fn display<'a>(
        &'a self,
        source: &'a str,
        file_name: Option<&'s str>,
    ) -> impl std::fmt::Display + 'a {
        self.1
            .iter()
            .map(move |err| err.display(source, file_name)).map(move |display| {
                format!(
                    "{}",
                    display
                )
            }).collect::<Vec<_>>().join("\n")//See below comment, I don't intend to use these messages (But knowing me, i will lol)
    }
}
#[derive(Debug, Clone)]
pub enum TokenizeResult<'s> {
    /// Tokenization was successful.
    Ok(Vec<TokenItem<'s>>),
    /// Tokenization failed with a list of tokens parsed so far and a list of errors.
    Err(TokenizeFailure<'s>),
}
impl <'s> TokenizeResult<'s> {
    pub fn tokens(&self) -> &Vec<TokenItem<'s>> {
        match self {
            TokenizeResult::Ok(tokens) => tokens,
            TokenizeResult::Err(failure) => &failure.0,
        }
    }
    pub fn into_tokens(self) -> Vec<TokenItem<'s>> {
        match self {
            TokenizeResult::Ok(tokens) => tokens,
            TokenizeResult::Err(failure) => failure.0
        }
    }
    pub fn errors(&self) -> &[LexerError<'s>] {
        match self {
            TokenizeResult::Ok(_) => &[],
            TokenizeResult::Err(failure) => failure.errors()
        }
    }
    pub fn split(self) -> (Vec<TokenItem<'s>>, Vec<LexerError<'s>>) {
        match self {
            TokenizeResult::Ok(tokens) => (tokens, vec![]),
            TokenizeResult::Err(failure) => (failure.0, failure.1),
        }
    }
    pub fn unwrap(self) -> Vec<TokenItem<'s>> {//Why would you want to do this?
        match self {
            TokenizeResult::Ok(tokens) => tokens,
            TokenizeResult::Err(failure) => panic!("Tokenization failed: {:?}", failure),
        }
    }
    pub fn unwrap_err(self) -> TokenizeFailure<'s> {
        match self {
            TokenizeResult::Ok(_) => panic!("This literally only gets used in examples :)"),
            TokenizeResult::Err(failure) => failure,
        }
    }
}
//Look this branch is for my own use
//I dont care that much about making this pretty :)

/// Parses an input string into a list of tokens.
///
/// # Example
/// ```
/// use sqparse::{Flavor, tokenize};
///
/// let source = r#"
/// global function MyFunction
///
/// struct {
///     int a
/// } file
///
/// string function MyFunction( List<number> values ) {
///     values.push(1 + 2)
/// }
/// "#;
///
/// let tokens = tokenize(source, Flavor::SquirrelRespawn).unwrap();
/// assert_eq!(tokens.len(), 29);
/// ```
pub fn tokenize(val: &str, flavor: Flavor) -> TokenizeResult {
    let mut items: Vec<TokenItem<'_>> = Vec::<TokenItem>::new();
    let mut layers = VecDeque::<Layer>::new();
    let mut errs = Vec::<LexerError>::new();

    for maybe_token in TokenIter::new(val, flavor) {
        let token = maybe_token;
        if let Err(err) = token {
            //return Err((err, items));//Hack to preserve what you parsed so far, I would rather proper recovery :(
            errs.push(err);
            continue; // <- (Clueless)
        }
        let token = token.unwrap();
        let token_index = items.len();

        // If this token matches the top layer's close token, pop the layer.
        if let Some(top_layer) = layers.back() {
            if top_layer.close_ty == token.ty {
                items[top_layer.open_index].close_index = Some(token_index);
                layers.pop_back();
            }
        }

        // If this token is a valid opening token, push a new layer.
        if let Some(close_ty) = closing_token(token.ty) {
            layers.push_back(Layer {
                open_index: token_index,
                close_ty,
            });
        }

        items.push(TokenItem {
            token,
            close_index: None,
        });
    }

    // If there are remaining layers, there are one or more unmatched opening tokens. Otherwise
    // at this point tokenization is successful.
    match layers.back() {
        None => (),
        Some(layer) => {
            let open_token = &items[layer.open_index].token;
            let err = LexerError::new(
                LexerErrorType::UnmatchedOpener {
                    open: open_token.ty,
                    close: layer.close_ty,
                },
                open_token.range.clone(),
            );
            errs.push(err);
        }
    };
    if errs.is_empty() {
        TokenizeResult::Ok(items)
    } else {
        TokenizeResult::Err(TokenizeFailure(items, errs))
    }
}
