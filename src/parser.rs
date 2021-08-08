use nom::branch::alt;
use nom::bytes::complete::take;
use nom::combinator::{consumed, not, verify};
use nom::error::{ErrorKind, ParseError};
use nom::multi::{fold_many0, fold_many1, many1, many1_count};
use nom::number::complete::le_f32;
use nom::sequence::preceded;
use nom::Err::Error;
use nom::IResult;

#[derive(Debug)]
pub struct FloatRun {
    pub address: *const u8,
    pub values: Vec<f32>,
}

impl FloatRun {
    pub fn index_from_base(&self, base: *const u8) -> usize {
        (self.address as usize) - (base as usize)
    }
}
pub fn float_run(
    input: &[u8],
    min_length: usize,
) -> IResult<&[u8], FloatRun, FloatSearchError<&[u8]>> {
    preceded(
        |x| run_of_invalid_items(x, min_length),
        consumed(run_of_valid_floats),
    )(input)
    .map(|(remain, (consumed, values))| {
        (
            remain,
            FloatRun {
                address: consumed.as_ptr(),
                values,
            },
        )
    })
}

fn run_of_valid_floats(input: &[u8]) -> IResult<&[u8], Vec<f32>, FloatSearchError<&[u8]>> {
    many1(valid_f32)(input)
}

fn run_of_invalid_items(
    input: &[u8],
    min_length: usize,
) -> IResult<&[u8], (), FloatSearchError<&[u8]>> {
    fold_many0(
        alt((
            |x| optional_too_short_run_of_valid_floats(x, min_length),
            optional_run_of_invalid_floats,
        )),
        (),
        |_, _| (),
    )(input)
}

fn optional_too_short_run_of_valid_floats(
    input: &[u8],
    min_length: usize,
) -> IResult<&[u8], (), FloatSearchError<&[u8]>> {
    many1_count(valid_f32)(input).and_then(|(remaining, length)| {
        if length < min_length {
            Ok((remaining, ()))
        } else {
            Err(Error(FloatSearchError::LongRun))
        }
    })
}

fn optional_run_of_invalid_floats(input: &[u8]) -> IResult<&[u8], (), FloatSearchError<&[u8]>> {
    // Use fold with fake init and accumulator because we just want to consume the input and throw it away without heap allocations.
    fold_many1(preceded(not(valid_f32), take(4 as usize)), (), |_, _| ())(input)
}

pub fn valid_f32(input: &[u8]) -> IResult<&[u8], f32, FloatSearchError<&[u8]>> {
    verify(le_f32, |x| 0.0 < *x)(input)
}

#[derive(Debug, PartialEq)]
pub enum FloatSearchError<I> {
    LongRun,
    Nom(I, ErrorKind),
}

impl<I> ParseError<I> for FloatSearchError<I> {
    fn from_error_kind(input: I, kind: ErrorKind) -> Self {
        FloatSearchError::Nom(input, kind)
    }

    fn append(_: I, _: ErrorKind, other: Self) -> Self {
        other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn early_eof_with_some_valid_input() {
        // Initial test.
        let bytes = vec![0x32, 0x32, 0x32, 0x32, 0x32, 0x32, 0x32, 0x32, 0x32];
        let result = float_run(bytes.as_slice(), 2);
        match result {
            Ok((remaining_input, x)) => {
                println!(
                    "TEST: Got {:?} with {} bytes of unconsumed input.",
                    x,
                    remaining_input.len()
                );
            }
            Err(err) => {
                println!("TEST: Error in final parser state: {:?}", err);
                panic!("This test should pass!");
            }
        }
    }

    #[test]
    fn early_eof_no_input() {
        // Initial test.
        let bytes = vec![0x32, 0x32, 0x32];
        let result = float_run(bytes.as_slice(), 0);
        match result {
            Ok((remaining_input, x)) => {
                println!(
                    "TEST: Got {:?} with {} bytes of unconsumed input.",
                    x,
                    remaining_input.len()
                );
                panic!("This test should not pass!");
            }
            Err(err) => {
                println!("TEST: Error in final parser state: {:?}", err);
            }
        }
    }

    #[test]
    fn two_valid_values() {
        // Initial test.
        let bytes = vec![
            0x32, 0x32, 0x32, 0x32, 0, 0, 160, 193, 0x32, 0x32, 0x32, 0x32,
        ];
        let result = float_run(bytes.as_slice(), 0);
        let remaining_input = match result {
            Ok((remaining_input, x)) => {
                println!(
                    "TEST: Got {:?} with {} bytes of unconsumed input.",
                    x,
                    remaining_input.len()
                );
                remaining_input
            }
            Err(err) => {
                println!("TEST: Error in final parser state: {:?}", err);
                panic!("This test should pass!");
            }
        };
        let result = float_run(remaining_input, 0);
        match result {
            Ok((remaining_input, x)) => {
                println!(
                    "TEST: Got {:?} with {} bytes of unconsumed input.",
                    x,
                    remaining_input.len()
                );
            }
            Err(err) => {
                println!("TEST: Error in final parser state: {:?}", err);
                panic!("This test should pass!");
            }
        }
    }

    #[test]
    fn invalid_then_valid_then_invalid() {
        // Initial test.
        let bytes = vec![0, 0, 160, 193, 0x32, 0x32, 0x32, 0x32, 0, 0, 160, 193];
        let remaining_input = match float_run(bytes.as_slice(), 0) {
            Ok((remaining_input, x)) => {
                println!(
                    "TEST: Got {:?} with {} bytes of unconsumed input.",
                    x,
                    remaining_input.len()
                );
                assert_eq!(x.values.len(), 1);
                remaining_input
            }
            Err(err) => {
                println!("TEST: Error in final parser state: {:?}", err);
                panic!("This test should pass!");
            }
        };
        assert_eq!(remaining_input.len(), 4);
    }
}
