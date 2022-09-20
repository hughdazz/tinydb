use std::cell::RefCell;
use std::env;
use std::io;
use std::path::Path;
use std::rc::Rc;

use rustyline::error::ReadlineError;
use rustyline::Editor;
use tinydb::catalog::pg_database;
use tinydb::engine::Engine;
use tinydb::initdb::init_database;
use tinydb::storage::BufferPool;

use structopt::StructOpt;
use tinydb::storage::smgr::StorageManager;

/// Command line arguments
#[derive(StructOpt)]
#[structopt()]
struct Flags {
    /// Silence all output
    #[structopt(short = "q", long = "quiet")]
    quiet: bool,

    /// Initialize the database directory.
    #[structopt(long = "init")]
    init: bool,

    /// Path to store database files.
    #[structopt(long = "data-dir", default_value = "data")]
    data_dir: String,

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

    let cwd = env::current_dir().expect("Failed to get current working directory");

    let data_dir = cwd.join(&flags.data_dir);

    let mut buffer = BufferPool::new(120, StorageManager::new(&data_dir));

    if flags.init {
        init_database(&mut buffer, &data_dir).expect("Failed init default database");
    }

    let mut rl = Editor::<()>::new();
    if rl.load_history(&cwd.join("history.txt")).is_err() {
        println!("No previous history.");
    }

    env::set_current_dir(Path::new(&flags.data_dir)).unwrap();

    let mut stdout = io::stdout();
    let mut engine = Engine::new(Rc::new(RefCell::new(buffer)));

    println!("Connected at {} database", default_db_name);
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                if let Err(err) = engine.exec(&mut stdout, &line, &pg_database::TINYDB_OID) {
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
    rl.save_history(&cwd.join("history.txt")).unwrap();
}
