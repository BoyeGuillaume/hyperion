use chumsky::Parser;
use clap::Parser as ClapParser;
use hyinstr::modules::{operand::Label, parser::parse_operand};

#[derive(ClapParser)]
pub struct Arguments {
    /// Input file path. Use "-" to read from stdin.
    #[arg(short, long)]
    input: Option<String>,

    /// Inline code to parse, if not provided, then [input] must be provided.
    #[arg(short, long)]
    code: Option<String>,
}

fn main() {
    let args = Arguments::parse();

    // Read input code
    let input_code = if let Some(code) = args.code {
        if args.input.is_some() {
            eprintln!("Warning: both --code and input file path provided, using --code.");
        }

        code
    } else if let Some(input_path) = args.input {
        if input_path == "-" {
            // Read from stdin
            std::io::read_to_string(std::io::stdin()).expect("Failed to read from stdin")
        } else {
            // Read from file
            std::fs::read_to_string(input_path).expect("Failed to read input file")
        }
    } else {
        eprintln!("Error: either --code or input file path must be provided.");
        std::process::exit(1);
    };

    // Parse the input code into a module
    let operand_parser = parse_operand(|_| 0, |_| Label::NIL);
    match operand_parser.parse(input_code.as_str()).into_result() {
        Ok(operand) => {
            println!("Parsed operand: {:?}", operand);
        }
        Err(errors) => {
            eprintln!("Failed to parse operand:");
            for error in errors {
                eprintln!("  {}", error);
            }
            std::process::exit(1);
        }
    }
}
