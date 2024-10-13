#![feature(portable_simd)]

use std::io::Write;

use parse::{parse_template, Block, NumberedBlock};

pub mod parse {
    use nom::{
        branch::alt,
        bytes::complete::{tag, take_until},
        error::{Error, ErrorKind},
        multi::many0,
        sequence::tuple,
        IResult, InputLength, InputTake, Parser,
    };

    #[derive(Clone, Debug)]
    pub struct NumberedInput<'a> {
        pub line_number: usize,
        pub i: &'a [u8],
    }

    impl<'a> InputLength for NumberedInput<'a> {
        fn input_len(&self) -> usize {
            self.i.input_len()
        }
    }
    fn make_special_parser<'a>(
        left_sep: &'static [u8],
        right_sep: &'static [u8],
        constructor: fn(&'a [u8]) -> Special<'a>,
    ) -> impl Fn(NumberedInput<'a>) -> IResult<NumberedInput<'a>, NumberedBlock<'a>> {
        move |ni: NumberedInput<'a>| {
            parse_special_with_separator(left_sep, right_sep, ni.i)
                .map(|(rest, result)| {
                    (
                        NumberedInput {
                            line_number: ni.line_number
                                + result.iter().filter(|&&x| x == b'\n').count(),
                            i: rest,
                        },
                        NumberedBlock {
                            line_number: ni.line_number,
                            block: Block::Special(constructor(result.trim_ascii())),
                        },
                    )
                })
                .map_err(|e| match e {
                    nom::Err::Incomplete(needed) => nom::Err::Incomplete(needed),
                    nom::Err::Error(e) => nom::Err::Error(nom::error::Error::new(
                        NumberedInput {
                            line_number: ni.line_number,
                            i: e.input,
                        },
                        e.code,
                    )),
                    nom::Err::Failure(e) => nom::Err::Failure(nom::error::Error::new(
                        NumberedInput {
                            line_number: ni.line_number,
                            i: e.input,
                        },
                        e.code,
                    )),
                })
        }
    }

    pub fn parse_template<'a>(
        input: &'a [u8],
    ) -> IResult<NumberedInput<'a>, Vec<NumberedBlock<'a>>> {
        let percent = make_special_parser(b"{%", b"%}", Special::TagPercent);
        let curly = make_special_parser(b"{{", b"}}", Special::TagCurly);
        let hash = make_special_parser(b"{#", b"#}", Special::TagHash);

        let plain_parser = plain.map(|x| NumberedBlock {
            line_number: x.line_number,
            block: Block::Plain(x.i),
        });

        let block_parser = alt((percent, curly, hash, plain_parser));

        many0(block_parser)(NumberedInput {
            line_number: 0,
            i: input,
        })
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

    #[derive(Clone, Copy, Debug)]
    pub struct NumberedBlock<'a> {
        pub line_number: usize,
        pub block: Block<'a>,
    }

    /// Match a left and right delimited section
    /// For example `"{{ hello }}"` will turn into `result(" hello ")`
    fn parse_special_with_separator<'a>(
        left_sep: &'static [u8],
        right_sep: &'static [u8],
        input: &'a [u8],
    ) -> IResult<&'a [u8], &'a [u8]> {
        tuple((tag(left_sep), take_until(right_sep), tag(right_sep)))
            .map(|(_l, m, _r)| m)
            .parse(input)
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
    fn plain(input: NumberedInput<'_>) -> IResult<NumberedInput<'_>, NumberedInput<'_>> {
        let mut line_number = input.line_number;
        for i in 0..input.i.len() {
            if any_separator(&input.i[i..]).is_ok() {
                // must not parse any separators as part of raw output
                if i == 0 {
                    return Err(nom::Err::Failure(nom::error::Error::new(
                        input,
                        ErrorKind::Tag,
                    )));
                }
                let (rest, parsed) = input.i.take_split(i);
                return Ok((
                    NumberedInput {
                        line_number,
                        i: rest,
                    },
                    NumberedInput {
                        line_number: input.line_number,
                        i: parsed,
                    },
                ));
            }
            if input.i[i] == b'\n' {
                line_number += 1;
            }
        }
        if input.i.len() == 0 {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Fail)));
        }
        let (rest, parsed) = input.i.take_split(input.i.len());
        Ok((
            NumberedInput {
                line_number,
                i: rest,
            },
            NumberedInput {
                line_number: input.line_number,
                i: parsed,
            },
        ))
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

    #[cfg(test)]
    mod test {
        use insta::assert_debug_snapshot;

        use crate::parse::parse_template;

        #[test]
        fn test_simple() {
            assert_debug_snapshot!(parse_template(b"{{ hello }}"));
        }

        #[test]
        fn test_separator_in_special() {
            assert_debug_snapshot!(parse_template(b"{{ {{ }}"));
            assert_debug_snapshot!(parse_template(b"{% {{ %}"));
            assert_debug_snapshot!(parse_template(b"{# {{ #}"));

            assert_debug_snapshot!(parse_template(b"{{ {% }}"));
            assert_debug_snapshot!(parse_template(b"{% {% %}"));
            assert_debug_snapshot!(parse_template(b"{# {% #}"));

            assert_debug_snapshot!(parse_template(b"{{ {# }}"));
            assert_debug_snapshot!(parse_template(b"{% {# %}"));
            assert_debug_snapshot!(parse_template(b"{# {# #}"));

            assert_debug_snapshot!(parse_template(b"{{ }} }}"));
            assert_debug_snapshot!(parse_template(b"{% }} %}"));
            assert_debug_snapshot!(parse_template(b"{# }} #}"));

            assert_debug_snapshot!(parse_template(b"{{ %} }}"));
            assert_debug_snapshot!(parse_template(b"{% %} %}"));
            assert_debug_snapshot!(parse_template(b"{# %} #}"));

            assert_debug_snapshot!(parse_template(b"{{ #} }}"));
            assert_debug_snapshot!(parse_template(b"{% #} %}"));
            assert_debug_snapshot!(parse_template(b"{# #} #}"));
        }

        #[test]
        fn test_newlines() {
            assert_debug_snapshot!(parse_template(b"{{\n\n}}\n\nfoo"));
            assert_debug_snapshot!(parse_template(b"\n\n{{\n\n}}\n\nfoo"));
            assert_debug_snapshot!(parse_template(b"\n\nbar{{\n bar \n}}\n\nfoo"));
        }
    }
}

#[cfg(feature = "simd")]
mod parse_simd {}

pub struct ParsedTemplate<'a> {
    parsed: Vec<NumberedBlock<'a>>,
}

impl<'a> ParsedTemplate<'a> {
    pub fn new(template: &'a [u8]) -> Option<Self> {
        parse_template(&template)
            .ok()
            .filter(|x| x.0.i.len() == 0)
            .map(|(_, parsed)| Self { parsed })
    }

    pub fn instantiate(&self, wr: &mut impl Write) -> Result<(), std::io::Error> {
        for ins in self.parsed.iter() {
            match ins.block {
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
