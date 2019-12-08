use std::fmt;
use std::io::{self, BufRead, Cursor};
use std::str::FromStr;

use thiserror::Error;

use crate::util::*;

/// A single semver compliant version.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Version {
    /// The major version.
    pub major: u64,
    /// The minor version.
    pub minor: u64,
    /// The patch version.
    pub patch: u64,
    /// The prerelease version.
    pub prerelease: Vec<Identifier>,
    /// The build version.
    pub build: Vec<Identifier>,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.prerelease.is_empty() {
            write!(f, "-")?;
            for (i, id) in self.prerelease.iter().enumerate() {
                if i < self.prerelease.len() - 1 {
                    write!(f, "{}.", id)?;
                } else {
                    write!(f, "{}", id)?;
                }
            }
        }

        if !self.build.is_empty() {
            write!(f, "+")?;
            for (i, id) in self.build.iter().enumerate() {
                if i < self.build.len() - 1 {
                    write!(f, "{}.", id)?;
                } else {
                    write!(f, "{}", id)?;
                }
            }
        }
        Ok(())
    }
}

/// This the invidual parts in the string `beta.9`, separated by `.`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identifier {
    Number(u64),
    /// Only ASCII symbols.
    String(String),
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Identifier::Number(num) => num.fmt(f),
            Identifier::String(st) => st.fmt(f),
        }
    }
}
macro_rules! id_from_number {
    ($num:ty) => {
        impl From<$num> for Identifier {
            fn from(s: $num) -> Self {
                Identifier::Number(s as u64)
            }
        }
    };
}

id_from_number!(u8);
id_from_number!(i8);
id_from_number!(u16);
id_from_number!(i16);
id_from_number!(u32);
id_from_number!(i32);
id_from_number!(u64);
id_from_number!(i64);

impl Version {
    /// Create a new version.
    ///
    /// ```rust
    /// use semver2::Version;
    ///
    /// assert_eq!(&Version::new(2, 3, 0).to_string(), "2.3.0");
    /// ```
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Version {
            major,
            minor,
            patch,
            prerelease: Vec::new(),
            build: Vec::new(),
        }
    }

    /// Create a new version.
    ///
    /// ```rust
    /// use semver2::Version;
    ///
    /// assert_eq!(
    ///     &Version::new_prerelease(2, 3, 0, vec!["alpha".parse().unwrap()]).to_string(),
    ///     "2.3.0-alpha"
    /// );
    /// ```
    pub fn new_prerelease(major: u64, minor: u64, patch: u64, prerelease: Vec<Identifier>) -> Self {
        Version {
            major,
            minor,
            patch,
            prerelease,
            build: Vec::new(),
        }
    }

    /// Create a new version.
    ///
    /// ```rust
    /// use semver2::Version;
    ///
    /// assert_eq!(
    ///     &Version::new_build(2, 3, 0, vec!["githash".parse().unwrap()]).to_string(),
    ///     "2.3.0+githash"
    /// );
    /// ```
    pub fn new_build(major: u64, minor: u64, patch: u64, build: Vec<Identifier>) -> Self {
        Version {
            major,
            minor,
            patch,
            prerelease: Vec::new(),
            build,
        }
    }
}

// range-set  ::= range ( logical-or range ) *
// logical-or ::= ( ' ' ) * '||' ( ' ' ) *
// range      ::= hyphen | simple ( ' ' simple ) * | ''
// hyphen     ::= partial ' - ' partial
// simple     ::= primitive | partial | tilde | caret
// primitive  ::= ( '<' | '>' | '>=' | '<=' | '=' ) partial
// partial    ::= xr ( '.' xr ( '.' xr qualifier ? )? )?
// xr         ::= 'x' | 'X' | '*' | nr
// nr         ::= '0' | ['1'-'9'] ( ['0'-'9'] ) *
// tilde      ::= '~' partial
// caret      ::= '^' partial
// qualifier  ::= ( '-' pre )? ( '+' build )?
// pre        ::= parts
// build      ::= parts
// parts      ::= part ( '.' part ) *
// part       ::= nr | [-0-9A-Za-z]+

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Invalid input: {:?}", found)]
    Invalid { found: Option<char> },
    #[error("Invalid numeric range")]
    InvalidNumericRange,
    #[error("Unexpected end of input")]
    UnexpectedEof,
    #[error("IO")]
    Io(#[from] io::Error),
}

/// Parses any sequence of digits.
fn parse_numeric_range_loose<R: BufRead>(s: R) -> Result<u64, ParseError> {
    let raw = take_string_while(s, |b| b.is_ascii_digit())?;
    raw.parse().map_err(|_| ParseError::InvalidNumericRange)
}

fn parse_part<R: BufRead>(mut s: R) -> Result<Identifier, ParseError> {
    let part = take_string_while(&mut s, |b| b.is_ascii_alphanumeric())?;
    match part.parse::<u64>() {
        Ok(number) => Ok(number.into()),
        _ => Ok(Identifier::String(part)),
    }
}

fn parse_parts<R: BufRead>(mut s: R) -> Result<Vec<Identifier>, ParseError> {
    let mut res = Vec::new();
    loop {
        if is_eof(&mut s) {
            break;
        }

        res.push(parse_part(&mut s)?);

        let next = peek1(&mut s);
        if next == Some(b'.') {
            s.consume(1)
        } else {
            break;
        }
    }

    if res.is_empty() {
        // return Err(ParseError::UnexpectedEof);
    }

    Ok(res)
}

impl FromStr for Identifier {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = Cursor::new(s);

        parse_part(&mut s)
    }
}

impl FromStr for Version {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = Cursor::new(s);

        let mut version = Version::default();

        // Major
        version.major = parse_numeric_range_loose(&mut s)?;
        if is_eof(&mut s) {
            return Ok(version);
        }

        // .
        let next = take1(&mut s).map(|s| s as char);
        if next != Some('.') {
            return Err(ParseError::Invalid { found: next });
        }

        // Minor (optional)
        version.minor = parse_numeric_range_loose(&mut s)?;
        if is_eof(&mut s) {
            return Ok(version);
        }

        // .
        let next = take1(&mut s).map(|s| s as char);
        if next != Some('.') {
            return Err(ParseError::Invalid { found: next });
        }

        // Patch (optional)
        version.patch = parse_numeric_range_loose(&mut s)?;
        if is_eof(&mut s) {
            return Ok(version);
        }

        let mut next = peek1(&mut s).map(|s| s as char);
        if next == Some('+') || next == Some('-') {
            s.consume(1);
        }

        // prerelease (optional)
        // interpret 1.2.3foo as 1.2.3-foo
        if next.is_some() && next != Some('+') {
            version.prerelease = parse_parts(&mut s)?;
            if is_eof(&mut s) {
                return Ok(version);
            }

            // read the next part, as we consumed our next.
            next = take1(&mut s).map(|s| s as char);
        }

        // build (optional)
        if next == Some('+') {
            version.build = parse_parts(&mut s)?;
            if is_eof(&mut s) {
                return Ok(version);
            }
        }
        Err(ParseError::Invalid { found: next })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_parsing() {
        assert_eq!("1.2.3".parse::<Version>().unwrap(), Version::new(1, 2, 3));
        assert_eq!(
            "1.2.3-alpha.3".parse::<Version>().unwrap(),
            Version::new_prerelease(1, 2, 3, vec!["alpha".parse().unwrap(), 3.into()])
        );
        assert_eq!(
            "1.2.3+alpha.3".parse::<Version>().unwrap(),
            Version::new_build(1, 2, 3, vec!["alpha".parse().unwrap(), 3.into()])
        );

        assert_eq!(
            "1.2.3-beta.9+acd.v3.2".parse::<Version>().unwrap(),
            Version {
                major: 1,
                minor: 2,
                patch: 3,
                prerelease: vec!["beta".parse().unwrap(), 9.into()],
                build: vec!["acd".parse().unwrap(), "v3".parse().unwrap(), 2.into()],
            }
        );
    }

    #[test]
    fn display() {
        assert_eq!(&Version::new(1, 2, 3).to_string(), "1.2.3");
        assert_eq!(
            &Version::new_prerelease(1, 2, 3, vec![0.into(), "alpha".parse().unwrap()]).to_string(),
            "1.2.3-0.alpha"
        );
        assert_eq!(
            &Version::new_build(1, 2, 3, vec![0.into(), "alpha".parse().unwrap()]).to_string(),
            "1.2.3+0.alpha"
        );
        assert_eq!(
            &Version {
                major: 1,
                minor: 2,
                patch: 3,
                prerelease: vec![0.into(), "alpha".parse().unwrap()],
                build: vec!["bla".parse().unwrap(), 9.into()],
            }
            .to_string(),
            "1.2.3-0.alpha+bla.9"
        );
    }

    #[test]
    fn loose_versions() {
        assert_eq!(
            "001.20.0301".parse::<Version>().unwrap(),
            Version::new(1, 20, 301)
        );

        assert_eq!(
            "1.2.3-beta.01".parse::<Version>().unwrap(),
            Version::new_prerelease(1, 2, 3, vec!["beta".parse().unwrap(), 1.into()])
        );

        assert_eq!(
            "1.2.3foo".parse::<Version>().unwrap(),
            Version::new_prerelease(1, 2, 3, vec!["foo".parse().unwrap()])
        );

        assert_eq!(
            "1.2.3foo.8".parse::<Version>().unwrap(),
            Version::new_prerelease(1, 2, 3, vec!["foo".parse().unwrap(), 8.into()])
        );
    }
}
