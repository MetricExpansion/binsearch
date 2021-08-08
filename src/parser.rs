use floatrun::FloatRun;
use nom::branch::alt;
use nom::bytes::complete::take;
use nom::combinator::{consumed, not, verify};
use nom::error::{ErrorKind, ParseError};
use nom::multi::{fold_many0, fold_many1, many1, many1_count};
use nom::number::complete::le_f32;
use nom::sequence::preceded;
use nom::Err::Error;
use nom::IResult;
use std::mem::size_of;

pub fn value_run_proc<V, E, F>(
    input: &[u8],
    extractor: E,
    min_length: usize,
    condition: F,
) -> (Option<FloatRun<V>>, &[u8])
where
    V: Sized,
    E: Fn(&[u8]) -> V,
    F: Fn(V) -> bool,
{
    let mut valid_range = None;
    let mut pos = 0;
    let value_size = size_of::<V>();
    while pos + value_size <= input.len() {
        let value = extractor(&input[pos..pos + value_size]);
        if condition(value) {
            if let None = valid_range {
                valid_range = Some(&input[pos..]);
            }
        } else {
            if let Some(valid_range_unwrapped) = valid_range {
                let length =
                    pos - (valid_range_unwrapped.as_ptr() as usize - input.as_ptr() as usize);
                if length >= size_of::<f32>() * min_length {
                    let values = valid_range_unwrapped[..length]
                        .chunks(value_size)
                        .map(extractor)
                        .collect();
                    return (
                        Some(FloatRun {
                            address: valid_range_unwrapped.as_ptr(),
                            values,
                        }),
                        &valid_range_unwrapped[length..],
                    );
                } else {
                    valid_range = None;
                }
            }
        }
        pos += size_of::<f32>();
    }
    if let Some(valid_range) = valid_range {
        let length = pos - (valid_range.as_ptr() as usize - input.as_ptr() as usize);
        if length >= size_of::<f32>() * min_length {
            let values = valid_range[..length]
                .chunks(value_size)
                .map(extractor)
                .collect();
            return (
                Some(FloatRun {
                    address: valid_range.as_ptr(),
                    values,
                }),
                &valid_range[length..],
            );
        }
    } else {
        // valid_range = None;
    }
    (None, input)
}

pub fn float_run<F: Fn(f32) -> bool>(
    input: &[u8],
    min_length: usize,
    condition: F,
) -> IResult<&[u8], FloatRun<f32>, FloatSearchError<&[u8]>> {
    preceded(
        |x| run_of_invalid_items(x, min_length, &condition),
        consumed(|x| run_of_valid_floats(x, &condition)),
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

fn run_of_valid_floats<F: Fn(f32) -> bool>(
    input: &[u8],
    condition: F,
) -> IResult<&[u8], Vec<f32>, FloatSearchError<&[u8]>> {
    many1(|x| valid_f32(x, &condition))(input)
}

fn run_of_invalid_items<F: Fn(f32) -> bool>(
    input: &[u8],
    min_length: usize,
    condition: F,
) -> IResult<&[u8], (), FloatSearchError<&[u8]>> {
    fold_many0(
        alt((
            |x| optional_too_short_run_of_valid_floats(x, min_length, &condition),
            |x| optional_run_of_invalid_floats(x, &condition),
        )),
        (),
        |_, _| (),
    )(input)
}

fn optional_too_short_run_of_valid_floats<F: Fn(f32) -> bool>(
    input: &[u8],
    min_length: usize,
    condition: F,
) -> IResult<&[u8], (), FloatSearchError<&[u8]>> {
    many1_count(|x| valid_f32(x, &condition))(input).and_then(|(remaining, length)| {
        if length < min_length {
            Ok((remaining, ()))
        } else {
            Err(Error(FloatSearchError::LongRun))
        }
    })
}

fn optional_run_of_invalid_floats<F: Fn(f32) -> bool>(
    input: &[u8],
    condition: F,
) -> IResult<&[u8], (), FloatSearchError<&[u8]>> {
    // Use fold with fake init and accumulator because we just want to consume the input and throw it away without heap allocations.
    fold_many1(
        preceded(not(|x| valid_f32(x, &condition)), take(4 as usize)),
        (),
        |_, _| (),
    )(input)
}

pub fn valid_f32<F: Fn(f32) -> bool>(
    input: &[u8],
    condition: F,
) -> IResult<&[u8], f32, FloatSearchError<&[u8]>> {
    verify(le_f32, |x| condition(*x))(input)
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
        let result = float_run(bytes.as_slice(), 2, |x| x > 0.0);
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
        let result = float_run(bytes.as_slice(), 0, |x| x > 0.0);
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
        let result = float_run(bytes.as_slice(), 0, |x| x > 0.0);
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
        let result = float_run(remaining_input, 0, |x| x > 0.0);
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
        let remaining_input = match float_run(bytes.as_slice(), 0, |x| x > 0.0) {
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
