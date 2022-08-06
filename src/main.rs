use std::io;
use std::path::PathBuf;

use rustyline::error::ReadlineError;
use rustyline::Editor;
use tinydb::engine::Engine;
use tinydb::initdb::init_database;
use tinydb::storage::BufferPool;

use structopt::StructOpt;

/// Command line arguments
#[derive(StructOpt)]
#[structopt()]
struct Flags {
    /// Silence all output
    #[structopt(short = "q", long = "quiet")]
    quiet: bool,

    /// Verbose mode (-v, -vv, -vvv, etc)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,
}

fn main() {
    let flags = Flags::from_args();

    stderrlog::new()
        .module(module_path!())
        .quiet(flags.quiet)
        .verbosity(flags.verbose)
        .init()
        .unwrap();

    let default_db_name = "tinydb";

    // Create a default tinydb database.
    init_database(&PathBuf::from("data"), &default_db_name).expect("Failed init default database");

    let mut rl = Editor::<()>::new();
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }

    let mut stdout = io::stdout();
    let buffer = BufferPool::new(120);
    let mut engine = Engine::new(buffer, "data");

    println!("Connected at {} database", default_db_name);
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                if let Err(err) = engine.exec(&mut stdout, &line, default_db_name) {
                    eprintln!("Error: {:?}", err);
                    continue;
                }
                println!("Ok");
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    rl.save_history("history.txt").unwrap();
}
