use omnom::prelude::*;
use std::io::{self, BufRead};

pub fn is_eof<R: BufRead>(mut s: R) -> bool {
    let l = s.fill_buf().unwrap().len();
    l == 0
}

pub fn take1<R: BufRead>(mut s: R) -> Option<u8> {
    if is_eof(&mut s) {
        return None;
    }

    let mut token = vec![0; 1];
    s.read_exact(&mut token).ok()?;

    Some(token[0])
}

pub fn peek1<R: BufRead>(mut s: R) -> Option<u8> {
    if is_eof(&mut s) {
        return None;
    }

    let mut token = vec![0; 1];
    s.fill_exact(&mut token).ok()?;

    Some(token[0])
}

pub fn take_string_while<R: BufRead, P>(mut s: R, predicate: P) -> io::Result<String>
where
    P: FnMut(u8) -> bool,
{
    let mut val = vec![];
    s.read_while(&mut val, predicate)?;
    String::from_utf8(val)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err.to_string()))
}
