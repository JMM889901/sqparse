use crate::lexer::TokenItem;
use crate::Flavor;

#[derive(Debug, Clone, Copy)]
pub struct TokenList<'s> {
    flavor: Flavor,
    tokens: &'s [TokenItem<'s>],
    index: usize,
}

impl TokenList<'_> {
    pub fn new<'s>(flavor: Flavor, tokens: &'s [TokenItem<'s>]) -> TokenList<'s> {
        TokenList {
            flavor,
            tokens,
            index: 0,
        }
    }
}


pub trait TokenIter<'s> : Copy{
    fn flavor(self) -> Flavor;
    fn previous(self) -> Option<&'s TokenItem<'s>>;
    fn next(self) -> Option<&'s TokenItem<'s>>;
    fn previous_index(&self) -> Option<usize>;
    fn is_ended(self) -> bool;
    fn start_index(self) -> usize;
    fn is_newline(self) -> bool;
    fn split_first(self) -> Option<(Self, &'s TokenItem<'s>)>;
    fn split_at(self, index: usize) -> (Self, Self);
}
impl<'s> TokenIter<'s> for TokenList<'s> {

    fn previous_index(&self) -> Option<usize> {
        if self.index > 0 {
            Some(self.index - 1)
        } else {
            None
        }
    }

    fn flavor(self) -> Flavor {
        self.flavor
    }

    fn previous(self) -> Option<&'s TokenItem<'s>> {
        if self.index > 0 {
            self.tokens.get(self.index - 1)
        } else {
            None
        }
    }

    fn next(self) -> Option<&'s TokenItem<'s>> {
        self.tokens.get(self.index)
    }

    fn is_ended(self) -> bool {
        self.index == self.tokens.len()
    }

    fn start_index(self) -> usize {
        self.index
    }

    fn is_newline(self) -> bool {
        self.previous()
            .map(|item| item.token.new_line.is_some())
            .unwrap_or(false)
    }

    fn split_first(self) -> Option<(Self, &'s TokenItem<'s>)> {
        self.next().map(|first| {
            (
                TokenList {
                    flavor: self.flavor,
                    tokens: self.tokens,
                    index: self.index + 1,
                },
                first,
            )
        })
    }

    fn split_at(self, index: usize) -> (TokenList<'s>, TokenList<'s>) {
        assert!(index >= self.index);
        (
            TokenList {
                flavor: self.flavor,
                tokens: &self.tokens[..index],
                index: self.index,
            },
            TokenList {
                flavor: self.flavor,
                tokens: self.tokens,
                index,
            },
        )
    }
}
