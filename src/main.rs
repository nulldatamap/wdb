use clap::{Args, Parser, Subcommand};
use rusqlite::{Connection, Row};

type DbResult<E> = rusqlite::Result<E>;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Add a new word
    Add(AddArgs),
    /// Dump a language's lexical inventory
    Dump(DumpArgs),
    /// List all languages
    List,
}

#[derive(Args)]
struct DumpArgs {
    /// ID of the target language
    language_id: String,
}

#[derive(Args, Debug)]
struct AddArgs {
    /// The language to add the word to
    language_id: String,
    /// The romanized spelling of the word
    word: String,
    /// The meaning of the word
    meaning: String,
    /// The part-of-speech the word belond to (v, n, adv, adj, inj, conj, adp)
    kind: String,
    /// Where the word comes from (unspecified means it's a neoglism)
    #[arg(short, long)]
    origin: Option<String>,
    /// Attach a note to the word (arbitrary text)
    #[arg(short, long)]
    note: Option<String>,
    /// Allow definining the word to be a homophone of any existing words
    #[arg(short = 'H', long)]
    homophone: bool,
}

#[derive(Debug)]
struct WordEntry {
    id: u32,
    ipa: String,
    romanization: String,
    meaning: String,
    kind: String,
    origin: String,
    flags: String,
    note: String,
}

impl WordEntry {
    fn from_row(row: &Row) -> DbResult<WordEntry> {
        Ok(WordEntry {
            id: row.get(0)?,
            ipa: row.get(1)?,
            romanization: row.get(2)?,
            meaning: row.get(3)?,
            kind: row.get(4)?,
            origin: row.get(5)?,
            flags: row.get(6)?,
            note: row.get(7)?,
        })
    }
}

#[derive(Debug)]
struct LangEntry {
    id: String,
    table: String,
    name: String,
}

impl std::fmt::Display for LangEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.id)
    }
}

impl LangEntry {
    fn from_row(row: &Row) -> DbResult<LangEntry> {
        Ok(LangEntry {
            id: row.get(0)?,
            table: row.get(1)?,
            name: row.get(2)?,
        })
    }
}

fn normalize_text(s: &str) -> String {
    s.trim().to_string()
}

struct Wdb {
    db: Connection,
}

impl Wdb {
    fn new() -> DbResult<Wdb> {
        Ok(Wdb {
            db: Connection::open("../langs.db")?,
        })
    }

    fn resolve_lang(&self, lang: &str) -> rusqlite::Result<LangEntry> {
        self.db.query_row(
            "SELECT * FROM langs WHERE id = ?",
            [lang],
            LangEntry::from_row,
        )
    }

    fn dump(&mut self, args: DumpArgs) {
        let lang = self
            .resolve_lang(&args.language_id)
            .expect("Invalid language");
        let mut stmt = self
            .db
            .prepare(&format!("SELECT * FROM {}", lang.table))
            .unwrap();
        let mut entries = stmt.query_map([], WordEntry::from_row).unwrap();

        for entry in entries {
            println!("{:?}", entry);
        }
    }

    fn list(&mut self) {
        let mut stmt = self.db.prepare("SELECT * FROM langs").unwrap();
        let mut entries = stmt.query_map([], LangEntry::from_row).unwrap();

        println!("Languages:");
        for entry in entries {
            let entry = entry.unwrap();
            let words: u32 = self
                .db
                .query_row(
                    &format!("SELECT COUNT(id) FROM {}", entry.table),
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            println!(" {}: {} ({} words)", entry.id, entry.name, words);
        }
    }

    fn add(&mut self, args: AddArgs) {
        println!("{:?}", args);
        let lang = self.resolve_lang(&args.language_id).unwrap();
        let rom = normalize_text(&args.word);
        // Make sure there isn't already another word in the db if it's not supposed to be a homophone
        if !args.homophone {
            let mut stmt = self
                .db
                .prepare(&format!("SELECT * FROM {} WHERE romanization = ?", &lang.table))
                .unwrap();
            let mut homophones: Vec<_> = stmt.query_map([&rom], WordEntry::from_row).unwrap().map(|x| x.unwrap()).collect();
            if !homophones.is_empty() {
                println!("Error adding word `{}` to language {}. \nThe following homophone(s) exist already:", &rom, lang);
                for h in homophones {
                    println!(" - {}: {}, {}", h.romanization, h.meaning, h.kind);
                }

                println!("If you want to add another homophone, use the -H flag.");
                std::process::exit(1);
            }
        }


    }
}

fn main() {
    let cli = Cli::parse();

    let mut wdb = Wdb::new().unwrap();

    match cli.command {
        Some(Command::Dump(args)) => wdb.dump(args),
        Some(Command::List) => wdb.list(),
        Some(Command::Add(args)) => wdb.add(args),
        None => {}
    }
}
