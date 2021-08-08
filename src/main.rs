use alloc_counter::{count_alloc, AllocCounterSystem};
use memmap::{Mmap, MmapOptions};
use rand::Rng;
use std::{fs::File, ops::Deref, path::PathBuf};
use structopt::clap::arg_enum;
use structopt::StructOpt;

#[global_allocator]
static A: AllocCounterSystem = AllocCounterSystem;

arg_enum! {
    #[derive(Debug, StructOpt)]
    enum Implementation {
        Rust,
        Cpp,
        Nom
    }
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "binsearch",
    about = "A tool to find single-precision floating point numbers inside of binary data."
)]
struct Opt {
    #[structopt(parse(from_os_str), help = "The file to search.")]
    input: Option<PathBuf>,

    #[structopt(
        long,
        default_value = "1048576",
        // conflicts_with("input"),
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

    #[structopt(long, help = "Select an implementation to use. Choices are Rust, Cpp, and Nom.", default_value = "Rust")]
    use_impl: Implementation,

    #[structopt(
        long,
        help = "Select to use the nom-based implementation of the search"
    )]
    use_nom: bool,

    #[structopt(long, default_value = "0", help = "Minimum length of run to print.")]
    min_length: usize,
}

enum DataSource {
    Mmap(Mmap),
    Vec(Vec<u8>),
}

impl Deref for DataSource {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match self {
            DataSource::Mmap(mmap) => &*mmap,
            DataSource::Vec(vec) => &vec[..],
        }
    }
}

fn main() {
    let opt = Opt::from_args();

    let bytes = if let Some(path) = &opt.input {
        println!("Loading {}", path.to_str().unwrap_or_else(|| "file."));
        let file = File::open(path).expect("Could not find input file.");
        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)
                .expect("Could not read input file.")
        };
        DataSource::Mmap(mmap)
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
        DataSource::Vec(bytes)
    };

    // And let's test the parser...
    println!("Searching in {} bytes of data.", bytes.len());
    let (counts, v) = count_alloc(|| {
        match opt.use_impl {
            Implementation::Rust => {
                main_data_search_rust(&*bytes, opt.min, opt.max, opt.min_length);
            }
            Implementation::Cpp => {
                main_data_search_c(&*bytes, opt.min, opt.max, opt.min_length);
            }
            Implementation::Nom => {
                main_data_search_nom(&*bytes, opt.min, opt.max, opt.min_length);
            }
        };
    });
    eprintln!("Allocations: {:?}", counts);
    return v;
}

fn main_data_search_nom(data: &[u8], min: Option<f32>, max: Option<f32>, min_length: usize) {
    let mut count = 0;
    let mut it = nom::combinator::iterator(data, move |x| {
        parser::float_run(x, min_length, |x| {
            min.map(|min| x >= min).unwrap_or(true) && max.map(|max| x <= max).unwrap_or(true)
        })
    });
    it.for_each(|v| {
        count += 1;
        println!(
            "{} values at {:#016x?}: {:?}",
            v.values.len(),
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

fn main_data_search_c(data: &[u8], min: Option<f32>, max: Option<f32>, min_length: usize) {
    let mut count = 0;
    let mut remain = data;
    loop {
        let (value, local_remain) = cversion::search(remain, min, max, min_length);
        remain = local_remain;
        if let Some(v) = value {
            count += 1;
            println!(
                "{} values at {:#016x?}: {:?}",
                v.values.len(),
                v.index_from_base(data.as_ptr()),
                v.values
            );
        } else {
            break;
        }
    }
    println!("Found {} ranges.", count);
}

fn main_data_search_rust(data: &[u8], min: Option<f32>, max: Option<f32>, min_length: usize) {
    let mut count = 0;
    let mut remain = data;
    loop {
        let (value, local_remain) = parser::float_run_proc(remain, min_length, |x| {
            min.map(|min| x >= min).unwrap_or(true) && max.map(|max| x <= max).unwrap_or(true)
        });
        remain = local_remain;
        if let Some(v) = value {
            count += 1;
            println!(
                "{} values at {:#016x?}: {:?}",
                v.values.len(),
                v.index_from_base(data.as_ptr()),
                v.values
            );
        } else {
            break;
        }
    }
    println!("Found {} ranges.", count);
}

mod parser;
