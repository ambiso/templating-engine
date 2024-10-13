use std::io::Write;

use parse::{parse_template, Block};

pub mod parse {
    use nom::{
        branch::alt,
        bytes::complete::{tag, take_until},
        error::{Error, ErrorKind},
        multi::many0,
        sequence::tuple,
        IResult, InputTake, Parser,
    };

    pub fn parse_template(input: &[u8]) -> IResult<&[u8], Vec<Block<'_>>> {
        let percent =
            parse_special_with_separator(b"{%", b"%}", |x| Special::TagPercent(x.trim_ascii()))
                .map(|x| Block::Special(x));
        let curly =
            parse_special_with_separator(b"{{", b"}}", |x| Special::TagCurly(x.trim_ascii()))
                .map(|x| Block::Special(x));
        let hash = parse_special_with_separator(b"{#", b"#}", |x| Special::TagHash(x.trim_ascii()))
            .map(|x| Block::Special(x));
        many0(
            alt((percent, curly, hash, plain.map(|x| Block::Plain(x)))).map(|x| {
                dbg!(&x);
                x
            }),
        )(input)
    }

    #[derive(Clone, Copy)]
    pub enum Special<'a> {
        TagPercent(&'a [u8]), // {% %}
        TagCurly(&'a [u8]),   // {{ }}
        TagHash(&'a [u8]),    // {# #}
    }

    #[derive(Clone, Copy)]
    pub enum Block<'a> {
        Special(Special<'a>),
        Plain(&'a [u8]), // plain text region without any separators
    }

    /// Match a left and right delimited section
    /// For example `"{{ hello }}"` will turn into `result(" hello ")`
    fn parse_special_with_separator(
        left_sep: &'static [u8],
        right_sep: &'static [u8],
        result: impl for<'c> Fn(&'c [u8]) -> Special<'c>,
    ) -> impl for<'c> Fn(&'c [u8]) -> IResult<&'c [u8], Special<'c>> {
        move |input: &[u8]| {
            tuple((tag(left_sep), take_until(right_sep), tag(right_sep)))
                .map(|(_l, m, _r)| result(m))
                .parse(input)
        }
    }

    /// match any separator
    fn any_separator(input: &[u8]) -> IResult<&[u8], &[u8]> {
        alt((
            tag("{{"),
            tag("{%"),
            tag("{#"),
            tag("}}"),
            tag("%}"),
            tag("#}"),
        ))(input)
    }

    /// parse plain text until any separator occurs
    fn plain(input: &[u8]) -> IResult<&[u8], &[u8]> {
        for i in 0..input.len() {
            if any_separator(&input[i..]).is_ok() {
                // must not parse any separators as part of raw output
                if i == 0 {
                    return Err(nom::Err::Failure(nom::error::Error::new(
                        input,
                        ErrorKind::Tag,
                    )));
                }
                return Ok(input.take_split(i));
            }
        }
        if input.len() == 0 {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Fail)));
        }
        Ok(input.take_split(input.len()))
    }

    // Manual impls for Debug to show utf-8 parsed result
    impl<'a> std::fmt::Debug for Special<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::TagPercent(arg0) => f
                    .debug_tuple("TagPercent")
                    .field(&std::str::from_utf8(arg0))
                    .finish(),
                Self::TagCurly(arg0) => f
                    .debug_tuple("TagCurly")
                    .field(&std::str::from_utf8(arg0))
                    .finish(),
                Self::TagHash(arg0) => f
                    .debug_tuple("TagHash")
                    .field(&std::str::from_utf8(arg0))
                    .finish(),
            }
        }
    }

    // Manual impls for Debug to show utf-8 parsed result
    impl<'a> std::fmt::Debug for Block<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Special(arg0) => f.debug_tuple("Special").field(arg0).finish(),
                Self::Plain(arg0) => f
                    .debug_tuple("Plain")
                    .field(&std::str::from_utf8(arg0))
                    .finish(),
            }
        }
    }
}

pub struct ParsedTemplate<'a> {
    parsed: Vec<Block<'a>>,
}

impl<'a> ParsedTemplate<'a> {
    pub fn new(template: &'a [u8]) -> Option<Self> {
        parse_template(&template)
            .ok()
            .filter(|x| x.0.len() == 0)
            .map(|(_, parsed)| Self { parsed })
    }

    pub fn instantiate(&self, wr: &mut impl Write) -> Result<(), std::io::Error> {
        for ins in self.parsed.iter() {
            match ins {
                Block::Plain(x) => wr.write_all(x)?,
                Block::Special(s) => match s {
                    parse::Special::TagPercent(s) => wr.write_all(s)?,
                    parse::Special::TagCurly(s) => wr.write_all(s)?,
                    parse::Special::TagHash(s) => wr.write_all(s)?,
                },
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::str;
    use std::io::Cursor;

    use super::*;

    #[test]
    fn it_works() {
        let mut c = Cursor::new(Vec::new());
        let t = ParsedTemplate::new(b"Hello {{ world }}").unwrap();
        t.instantiate(&mut c).unwrap();
        assert_eq!(str::from_utf8(c.get_ref()).unwrap(), "Hello world");
    }
}
