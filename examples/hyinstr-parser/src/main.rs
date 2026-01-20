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
            hyinstr::utils::Error::ParserErrors { errors, tokens } => {
                eprintln!("Failed to parse module from {}:", args.input);
                // if tokens.is_empty() {
                //     eprintln!("No tokens were produced.");
                // } else {
                //     eprintln!("Tokens: {}", tokens.join(" "));
                // }

                for error in errors {
                    let file = error.file.clone().unwrap_or_else(|| "<??>".to_string());
                    let span = (file.clone(), error.start..error.end);
                    let source = std::fs::read_to_string(&file).unwrap_or_default();

                    Report::build(ariadne::ReportKind::Error, span.clone())
                        .with_message(format!("{}", error.message))
                        .with_label(
                            Label::new(span)
                                .with_message("The error occurred here")
                                .with_color(a),
                        )
                        .finish()
                        .print((file.clone(), Source::from(source)))
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
}
