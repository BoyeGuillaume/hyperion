use std::io::Write;

use clap::Parser;
use hyformal::prelude::*;
use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};

/// Simple program to parse an HyFormal expression
/// and print the resulting AST
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// File to parse
    #[arg(short, long)]
    pub file: Option<String>,

    /// Whether to parse stdin (if set, file is ignored)
    #[arg(short, long, default_value_t = false)]
    pub stdin: bool,

    /// Specify code in the command line (overrides file and stdin)
    #[arg(short, long)]
    pub code: Option<String>,
}

fn main() {
    let args = Args::parse();

    let src = if let Some(code) = args.code {
        code
    } else if args.stdin {
        use std::io::Read;
        let mut buffer = String::new();
        std::io::stdin()
            .read_to_string(&mut buffer)
            .expect("Failed to read from stdin");
        buffer
    } else if let Some(file) = args.file {
        std::fs::read_to_string(file).expect("Failed to read file")
    } else {
        eprintln!("No input provided. Use --file <FILE> or --stdin.");
        std::process::exit(1);
    };

    let mut error_color = ColorSpec::new();
    error_color.set_fg(Some(termcolor::Color::Red));
    error_color.set_intense(true);

    let mut ok_color = ColorSpec::new();
    ok_color.set_fg(Some(termcolor::Color::Green));
    ok_color.set_intense(true);

    let stdout = StandardStream::stdout(ColorChoice::Auto);

    let mut stdout = stdout.lock();
    match parse(&src) {
        Ok(expr) => {
            stdout.set_color(&ok_color).unwrap();
            writeln!(stdout, "Expression parsed successfully:").unwrap();
            stdout.reset().unwrap();
            // println!("Parsed tokens: {:?}", expr);
            expr.pretty_print().unwrap();
            println!();
        }
        Err(e) => {
            stdout.set_color(&error_color).unwrap();
            writeln!(stdout, "Some errors were found during parsing:").unwrap();
            for err in e {
                writeln!(stdout, "  - {}", err).unwrap();
            }
            stdout.reset().unwrap();
        }
    }
    stdout.flush().unwrap();
}
