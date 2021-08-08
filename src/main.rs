use rand::Rng;
use std::{fs::File, io::{BufReader, Read}, path::PathBuf};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    #[structopt(parse(from_os_str))]
    input: Option<PathBuf>,
}

fn main() {
    let opt = Opt::from_args();
    println!("{:?}", opt);

    let bytes = if let Some(path) = opt.input {
        let mut file = BufReader::new(File::open(path).expect("Could not find input file."));
        let mut data = Vec::new();
        file.read_to_end(&mut data).expect("Could not read input file.");
        data
    } else {
        // Set up example data.
        let mut rng = rand::thread_rng();
        let input: Vec<f32> = (0..1048576).map(|_| rng.gen_range(-100.0..1.0)).collect();
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
    let mut count = 0;
    let mut it = nom::combinator::iterator(bytes.as_slice(), &|x| {
        parser::float_run(x, 100, |x| x > 0.0 && x < 0.5)
    });
    it.for_each(|v| {
        count += 1;
        println!(
            "Values at {:?}: {:?}",
            v.index_from_base(bytes.as_ptr()),
            v.values
        );
    });
    println!("Found {} ranges.", count);
    match it.finish() {
        Ok((remaining_input, _)) => {
            println!(
                "Finished with {} bytes of unconsumed input.",
                remaining_input.len()
            )
        }
        Err(err) => {
            println!("Error in final parser state: {:?}", err);
        }
    }
}

mod parser;
