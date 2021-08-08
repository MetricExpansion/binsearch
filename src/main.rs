use alloc_counter::{count_alloc, AllocCounterSystem};
use rand::Rng;
use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};
use structopt::StructOpt;

#[global_allocator]
static A: AllocCounterSystem = AllocCounterSystem;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "pcheck",
    about = "A tool to find single-precision floating point numbers inside of binary data."
)]
struct Opt {
    #[structopt(parse(from_os_str), help = "The file to search.")]
    input: Option<PathBuf>,

    #[structopt(
        long,
        default_value = "1048576",
        conflicts_with("input"),
        help = "If no file provided, the number of floats to generate as sample data."
    )]
    sample_data_length: usize,

    #[structopt(long, help = "Minimum value of floats to search for (inclusive).")]
    min: Option<f32>,

    #[structopt(
        long,
        required_unless("min"),
        help = "Maximum value of floats to search for (inclusive)."
    )]
    max: Option<f32>,

    #[structopt(long, default_value = "0", help = "Minimum length of run to print.")]
    min_length: usize,
}

fn main() {
    let opt = Opt::from_args();

    let bytes = if let Some(path) = &opt.input {
        println!("Loading {}", path.to_str().unwrap_or_else(|| "file."));
        let mut file = BufReader::new(File::open(path).expect("Could not find input file."));
        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .expect("Could not read input file.");
        data
    } else {
        // Set up example data.
        println!(
            "No input provided. Generating {} bytes of sample data.",
            4 * opt.sample_data_length
        );
        let mut rng = rand::thread_rng();
        let input: Vec<f32> = (0..opt.sample_data_length)
            .map(|_| rng.gen_range(-100.0..1.0))
            .collect();
        // let input: Vec<f32> = vec![2.0, -32.4, -32.4, 0.5, 0.4, -3.0, 4.2, 4.4];
        // let input: Vec<f32> = vec![-20.0];
        let bytes: Vec<_> = input
            .into_iter()
            .map(|f| unsafe { std::mem::transmute::<f32, [u8; 4]>(f) })
            .flatten()
            .collect();
        bytes
    };

    // And let's test the parser...
    println!("Searching in {} bytes of data.", bytes.len());
    let (counts, v) = count_alloc(|| {
        main_data_search(bytes.as_slice(), opt.min, opt.max, opt.min_length);
    });
    eprintln!("Allocations: {:?}", counts);
    return v;
}

fn main_data_search(data: &[u8], min: Option<f32>, max: Option<f32>, min_length: usize) {
    let mut count = 0;
    let mut it = nom::combinator::iterator(data, move |x| {
        parser::float_run(x, min_length, |x| {
            min.map(|min| x >= min).unwrap_or(true) && max.map(|max| x <= max).unwrap_or(true)
        })
    });
    it.for_each(|v| {
        count += 1;
        println!(
            "Values at {:?}: {:?}",
            v.index_from_base(data.as_ptr()),
            v.values
        );
    });
    println!("Found {} ranges.", count);
    match it.finish() {
        Ok((remaining_input, _)) => {
            eprintln!(
                "Finished with {} bytes of unconsumed input.",
                remaining_input.len()
            )
        }
        Err(err) => {
            eprintln!("Error in final parser state: {:?}", err);
        }
    };
}

mod parser;
