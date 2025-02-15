use mio::Token;

pub trait TokenBearer {
    fn get_token(&self) -> Token;
}

impl<T: TokenBearer + ?Sized> TokenBearer for &T {
    fn get_token(&self) -> Token {
        (**self).get_token()
    }
}

impl<T: TokenBearer + ?Sized> TokenBearer for &mut T {
    fn get_token(&self) -> Token {
        (**self).get_token()
    }
}

impl<T: TokenBearer + ?Sized> TokenBearer for Box<T> {
    fn get_token(&self) -> Token {
        (**self).get_token()
    }
}
