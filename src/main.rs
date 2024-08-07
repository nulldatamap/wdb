use std::error::Error;
use std::path::{Path, PathBuf};
use std::*;

use serde::Deserialize;
use clap::{Args, Parser, Subcommand};
use rusqlite::{params, Connection, Row};

type DbResult<E> = rusqlite::Result<E>;

#[derive(Deserialize)]
struct Settings {
    db_file: String,
    db_dev_file: String,
    word_list_folder: String,
    rule_list_folder: String,
}

struct Config {
    root: PathBuf,
    debug_mode: bool,
    settings: Settings,
    word_list_folder: cell::OnceCell<PathBuf>,
    rule_list_folder: cell::OnceCell<PathBuf>,
}

impl Config {
    fn new(root: PathBuf, settings: Settings, debug_mode: bool) -> Config {
        Config {
            root,
            settings,
            debug_mode,
            word_list_folder: cell::OnceCell::new(),
            rule_list_folder: cell::OnceCell::new(),
        }
    }

    fn word_list_folder(&self) -> &Path {
        let p = self.word_list_folder.get_or_init(|| {
            let mut b = PathBuf::new();
            b.push(&self.root);
            b.push(&self.settings.word_list_folder);
            b
        });
        p.as_path()
    }

    fn rule_list_folder(&self) -> &Path {
        let p = self.rule_list_folder.get_or_init(|| {
            let mut b = PathBuf::new();
            b.push(&self.root);
            b.push(&self.settings.rule_list_folder);
            b
        });
        p.as_path()
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Disable automatically checking word statuses (like missing IPA)
    #[arg(short, long)]
    disable_checks: bool,
    #[arg(long)]
    debug_mode: bool,
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
    /// Generate phonetic annotations for words based on thier romanization
    Phon(PhonArgs),
}

#[derive(Args)]
struct DumpArgs {
    /// ID of the target language
    language: String,
}

#[derive(Args, Debug)]
struct AddArgs {
    /// The language to add the word to
    language: String,
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
    /// The phonetic transcription of the word
    #[arg(short, long)]
    ipa: Option<String>,
    /// Disable auto-deromanization
    #[arg(short = 'D', long)]
    disable_autorom: bool,
    /// Allow definining the word to be a homophone of any existing words
    #[arg(short = 'H', long)]
    homophone: bool,
}

#[derive(Args, Debug)]
struct PhonArgs {
    #[arg(short, long = "lang")]
    language: Option<String>,
    /// Regenerate ALL phonetic annotation, not just the missing ones
    #[arg(short, long)]
    force: bool,
}

#[derive(Debug)]
struct WordEntry {
    id: u32,
    lang: String,
    romanization: String,
    ipa: Option<String>,
    meaning: String,
    kind: String,
    origin: Option<String>,
    flags: Option<String>,
    note: Option<String>,
}

impl WordEntry {
    fn from_row(row: &Row) -> DbResult<WordEntry> {
        Ok(WordEntry {
            id: row.get(0)?,
            lang: row.get(1)?,
            romanization: row.get(2)?,
            ipa: row.get(3)?,
            meaning: row.get(4)?,
            kind: row.get(5)?,
            origin: row.get(6)?,
            flags: row.get(7)?,
            note: row.get(8)?,
        })
    }
}

#[derive(Debug)]
struct LangEntry {
    id: String,
    name: String,
    origin: Option<String>,
    rule: String,
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
            name: row.get(1)?,
            origin: row.get(2)?,
            rule: row.get(3)?,
        })
    }
}

fn normalize_text(s: &str) -> String {
    s.trim().to_string()
}

struct Wdb {
    db: Connection,
    cfg: Config,
}

enum LexurgyMode {
    Evolve,
    Deromanize,
    Romanize,
}

struct LexurgyCmd {
    input: String,
    rule: String,
    output: Option<String>,
    mode: LexurgyMode,
}

impl LexurgyCmd {
    fn deromanize(lang: &LangEntry) -> LexurgyCmd {
        LexurgyCmd {
            mode: LexurgyMode::Deromanize,
            input: format!("{}_rom", &lang.id),
            output: None,
            rule: lang.rule.clone(),
        }
    }

    fn run<'a>(self, cfg: &Config, words: impl Iterator<Item = &'a str>) -> Result<Vec<String>, Box<dyn Error>> {
        use std::fs::File;
        use std::io::{BufRead, BufReader, BufWriter, Write};
        use std::process::*;

        let mut wli = PathBuf::new();
        wli.push(cfg.word_list_folder());
        wli.push(&self.input);
        wli.set_extension("wli");

        {
            let mut f = File::create(&wli)?;
            let mut buf = BufWriter::new(f);

            for word in words {
                buf.write_all(word.as_bytes())?;
                buf.write_all(b"\n")?;
            }
        }

        let mut lsc = PathBuf::new();
        lsc.push(cfg.rule_list_folder());
        lsc.push(self.rule);
        lsc.set_extension("lsc");

        let mut out = PathBuf::new();
        out.push(cfg.word_list_folder());
        out.push("out");

        let mut lexurgy = Command::new(if cfg!(windows) {
            "lexurgy.bat"
        } else {
            "lexurgy"
        });
        lexurgy
            .arg("sc")
            .arg(&lsc)
            .arg(&wli)
            .arg("--out-dir")
            .arg(&out);

        match self.mode {
            LexurgyMode::Evolve => {
                panic!("TODO!")
            }
            LexurgyMode::Romanize => {
                panic!("TODO!")
            }
            LexurgyMode::Deromanize => {
                lexurgy.arg("-p").arg("-b").arg("init");
            }
        }

        let cmd = format!("{:?}", &lexurgy);
        let output = lexurgy.output().unwrap();
        if !output.status.success() {
            return Err(format!(
                "{:?} failed.\nSTDOUT:\n{}\nSTDERR:\n{}\n",
                cmd,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
                       .into());
        }

        let mut ev_wli = PathBuf::new();
        ev_wli.push(cfg.word_list_folder());
        ev_wli.push("out");
        ev_wli.push(format!("{}_ev", &self.input));
        ev_wli.set_extension("wli");
        let f = File::open(ev_wli)?;
        let mut reader = BufReader::new(f);
        Ok(reader.lines().map(|l| l.unwrap()).collect())
    }
}

#[cfg(debug_assertions)]
const DB_FILE: &'static str = "../langs-dev.db";
#[cfg(not(debug_assertions))]
const DB_FILE: &'static str = "../langs.db";

impl Wdb {
    fn new(cfg: Config) -> DbResult<Wdb> {
        let db_file =
            if cfg.debug_mode {
                cfg.root.join(&cfg.settings.db_dev_file)
            } else {
                cfg.root.join(&cfg.settings.db_file)
            };
        Ok(Wdb {
            db: Connection::open(db_file)?,
            cfg,
        })
    }

    fn get_lang(&self, lang: &str) -> rusqlite::Result<LangEntry> {
        self.db.query_row(
            "SELECT * FROM langs WHERE id = ?",
            [lang],
            LangEntry::from_row,
        )
    }

    fn get_langs(&self) -> DbResult<Vec<LangEntry>> {
        let mut stmt = self.db.prepare("SELECT * FROM langs").unwrap();
        let mut entries = stmt.query_map([], LangEntry::from_row).unwrap();
        Ok(entries.map(|x| x.unwrap()).collect())
    }

    fn dump(&mut self, args: DumpArgs) {
        let lang = self.get_lang(&args.language).expect("Invalid language");
        let mut stmt = self
            .db
            .prepare("SELECT * FROM words WHERE lang = ? ORDER BY romanization")
            .unwrap();
        let mut entries = stmt.query_map([&lang.id], WordEntry::from_row).unwrap();

        println!("{} words:", &lang);
        for entry in entries {
            println!("  {:?}", entry);
        }
    }

    fn list(&mut self) {
        println!("Languages:");
        for entry in self.get_langs().unwrap() {
            let words: u32 = self
                .db
                .query_row(
                    "SELECT COUNT(id) FROM words WHERE lang = ?",
                    [&entry.id],
                    |row| row.get(0),
                )
                .unwrap();
            println!(" {}: {} ({} words)", entry.id, entry.name, words);
        }
    }

    fn add(&mut self, args: AddArgs) {
        println!("{:?}", args);
        let lang = self.get_lang(&args.language).unwrap();
        let rom = normalize_text(&args.word);
        // Make sure there isn't already another word in the db if it's not supposed to be a homophone
        if !args.homophone {
            let mut stmt = self
                .db
                .prepare("SELECT * FROM words WHERE romanization = ? AND lang = ?")
                .unwrap();
            let mut homophones: Vec<_> = stmt
                .query_map([&rom, &lang.id], WordEntry::from_row)
                .unwrap()
                .map(|x| x.unwrap())
                .collect();
            if !homophones.is_empty() {
                println!("Error adding word `{}` to language {}. \n\nThe following homophone(s) exist already:", &rom, lang);
                for h in homophones {
                    println!(" - {}: {}, {}", h.romanization, h.meaning, h.kind);
                }

                println!("\nIf you want to add it as a homophone, use the -H flag.");
                std::process::exit(1);
            }
        }

        let mut phon: Option<String> = args.ipa;

        if phon.is_none() && !args.disable_autorom {
            println!("Reromanization...");
            let mut phons = LexurgyCmd::deromanize(&lang).run(&self.cfg, std::iter::once(&rom[..])).unwrap();
            if phons.len() != 1 {
                println!("Error: expected a single word back, got {}", phons.len());
                std::process::exit(1);
            }
            println!("  {} => {}", &rom, &phons[0]);
            phon = Some(phons.remove(0));
        }

        let _ = self
            .db
            .execute(
                "INSERT INTO words
               (lang, romanization, ipa, meaning, kind, note, origin, flags)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    &lang.id,
                    &rom,
                    &phon,
                    &args.meaning,
                    &normalize_text(&args.kind),
                    &args.note.unwrap_or(String::new()),
                    &args.origin.unwrap_or(String::new()),
                    "",
                ],
            )
            .unwrap();
        println!("Added `{}` to {}", &args.word, lang);
    }

    fn check_missing_ipa(&mut self) {
        let mut stmt = self
            .db
            .prepare("SELECT * FROM words WHERE ipa IS NULL ORDER BY lang DESC, romanization")
            .unwrap();
        let mut words = stmt
            .query_map([], WordEntry::from_row)
            .unwrap()
            .map(|x| x.unwrap())
            .collect::<Vec<_>>();

        if words.is_empty() {
            return;
        }

        println!("\nNOTE: The following words are missing phonetic annotation:");
        for lang_words in words.chunk_by(|w0, w1| w0.lang == w1.lang) {
            println!("{}", self.get_lang(&lang_words[0].lang).unwrap());
            for word in lang_words {
                println!("- {}: {} ({})", word.romanization, word.meaning, word.kind);
            }
        }
        println!("\nRun `wdb phon` to generate phonetic annotations based on their romanization.");
    }

    fn deromanize(&mut self, args: PhonArgs) {
        let languages = args
            .language
            .as_ref()
            .map(|l| vec![self.get_lang(l).unwrap()])
            .unwrap_or_else(|| self.get_langs().unwrap());
        let mut any_change = false;
        for lang in languages {
            let words = {
                let mut stmt = self
                    .db
                    .prepare(&format!(
                        "SELECT * FROM words WHERE lang = ? {}",
                        if args.force { "" } else { "AND ipa IS NULL" }
                    ))
                    .unwrap();
                stmt.query_map([&lang.id], WordEntry::from_row)
                    .unwrap()
                    .map(|x| x.unwrap())
                    .collect::<Vec<_>>()
            };

            if words.is_empty() {
                continue;
            }

            any_change = true;

            let lexurgy = LexurgyCmd::deromanize(&lang);
            println!("Running `{}` deromanization rule...", &lang.rule);
            let phons = lexurgy
                .run(&self.cfg, words.iter().map(|w| &w.romanization[..]))
                .unwrap();
            if phons.len() != words.len() {
                println!(
                    "ERROR: Number of words out ({}) doesn't match number of words in({})!'",
                    phons.len(),
                    words.len()
                );
            }
            for (word, phon) in words.iter().zip(phons.iter()) {
                println!(" {} => {}", &word.romanization, phon);
            }

            let mut write_phons = ||
            {
                let tr = self.db.transaction()?;
                for (word, phon) in words.iter().zip(phons.iter()) {
                    tr.execute(
                        "UPDATE words SET ipa = ? WHERE id = ?",
                        params![phon, word.id],
                    )?;
                }
                tr.commit() };

            if let Err(err) = write_phons() {
                println!("Error: failed to update phonetic annotation. No words changed.\n{}", err);
                std::process::exit(1);
            } else {
                println!("Updated {} word entries", words.len());
            }
        }

        if !any_change {
            println!("No word updated, every word present has a phonetic annotation.\nIf you want to update all anyway, use the -f flag.");
        }
    }
}

fn find_obsidian_root() -> PathBuf {
    let cur = env::current_dir().unwrap();
    let mut obsidian = PathBuf::new();
    for root in cur.ancestors() {
        obsidian.clear();
        obsidian.push(root);
        obsidian.push(".obsidian");
        if obsidian.exists() {
            return root.into();
        }
    }
    println!("Error: you must run this command from inside of an Obsidian vault!");
    process::exit(1);
}

fn load_settings(root: &path::Path) -> Settings {
    toml::from_str(&fs::read_to_string(root.join("Wdb.toml")).expect("No `Wdb.toml` settings file present")).unwrap()
}

fn main() {
    if cfg!(debug_assertions) {
        println!(
            "NOTE: Running in debug, changes are done to the `lang-dev.db` instead of `lang.db`\n"
        );
    }
    let mut cli = Cli::parse();
    let root = find_obsidian_root();
    let settings = load_settings(&root);
    let mut cfg = Config::new(root, settings, cli.debug_mode | cfg!(debug_assertions));

    let mut wdb = Wdb::new(cfg).unwrap();

    match cli.command {
        Some(Command::Dump(args)) => wdb.dump(args),
        Some(Command::List) => wdb.list(),
        Some(Command::Add(args)) => wdb.add(args),
        Some(Command::Phon(args)) => {
            cli.disable_checks = args.language.is_none();
            wdb.deromanize(args)
        }
        None => {}
    }

    if !cli.disable_checks {
        wdb.check_missing_ipa();
    }
}
