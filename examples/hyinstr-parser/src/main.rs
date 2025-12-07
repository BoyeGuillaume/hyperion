use ariadne::{ColorGenerator, Label, Report, Source};
use clap::Parser as ClapParser;
use hyinstr::{
    modules::{Module, parser::extend_module_from_path},
    types::TypeRegistry,
};

#[derive(ClapParser)]
pub struct Arguments {
    /// Path to the input file
    input: String,
}

fn main() {
    let args = Arguments::parse();

    // Parse the input code into a module
    let type_registry = TypeRegistry::new([0; 6]);
    let mut module = Module::default();

    let mut colors = ColorGenerator::new();
    let a = colors.next();

    let path = std::path::Path::new(&args.input);
    match extend_module_from_path(&mut module, &type_registry, path) {
        Ok(_) => {
            println!("Successfully parsed module from {}", args.input);

            for (uuid, function) in &module.functions {
                println!("{}:\n{}\n", uuid, function.fmt(&type_registry, None));
            }
        }
        Err(error) => match error {
            hyinstr::utils::Error::ParserErrors { errors } => {
                eprintln!("Failed to parse module from {}:", args.input);
                for error in errors {
                    let span = (error.file.clone(), error.start..error.end);
                    let source = std::fs::read_to_string(&error.file).unwrap_or_default();

                    Report::build(ariadne::ReportKind::Error, span.clone())
                        .with_message(format!("{}", error.message))
                        .with_label(
                            Label::new(span)
                                .with_message("The error occurred here")
                                .with_color(a),
                        )
                        .finish()
                        .print((error.file.clone(), Source::from(source)))
                        .unwrap();
                }
                std::process::exit(1);
            }

            _ => {
                eprintln!("Error: {}", error);
                std::process::exit(1);
            }
        },
    }

    // Convert

    // match function_parser.parse(input_code.as_str()).into_result() {
    //     Ok(func) => {
    //         println!("Parsed function: {}", func.fmt(&type_registry, None));
    //     }
    //     Err(errors) => {
    //         eprintln!("Failed to parse function:");
    //         for error in errors {
    //             let span = error.span();
    //             Report::build(ariadne::ReportKind::Error, span.into_range())
    //                 .with_message(format!("{}", error))
    //                 .with_label(
    //                     Label::new(span.into_range())
    //                         .with_message("The error occurred here")
    //                         .with_color(a),
    //                 )
    //                 .finish()
    //                 .print(Source::from(input_code.as_str()))
    //                 .unwrap();
    //         }
    //         std::process::exit(1);
    //     }
    // }

    // let operand_parser = parse_operand(|_, _| Some(Uuid::nil()), |_| 0, |_| Label::NIL);
    // match operand_parser.parse(input_code.as_str()).into_result() {
    //     Ok(operand) => {
    //         println!("Parsed operand: {:?}", operand);
    //     }
    //     Err(errors) => {
    //         eprintln!("Failed to parse operand:");
    //         for error in errors {
    //             eprintln!("  {}", error);
    //         }
    //         std::process::exit(1);
    //     }
    // }
}
