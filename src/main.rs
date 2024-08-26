use std::path::{Path, PathBuf};
use std::*;

use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use tinytemplate::TinyTemplate;

#[derive(Deserialize)]
struct Settings {
    db_file: String,
    db_dev_file: String,
    word_list_folder: String,
    rule_list_folder: String,
    dictionary_file_template: String,
    dictionary_template: String,
    auto_dump: bool,
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
#[command(version, about, long_about = None, arg_required_else_help(true))]
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
    /// Read mutliple comamnds from STDIN
    Interactive,
    /// Add a new word
    Add(AddArgs),
    /// Edit a word
    Edit(EditArgs),
    /// Inherit a word from the language's predecessor
    Inherit(InheritArgs),
    /// Delete a word
    Del(DelArgs),
    /// Evolve a sentence
    Evolve(EvolveArgs),
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
struct EditArgs {
    /// The language to add the word to
    language: String,
    /// The romanized spelling of the word
    word: String,
    /// The meaning of the word
    #[arg(short, long)]
    meaning: Option<String>,
    /// The part-of-speech the word belond to (v, n, adv, adj, inj, conj, adp)
    #[arg(short, long)]
    kind: Option<String>,
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
}

#[derive(Args, Debug)]
struct InheritArgs {
    /// The language to add the word to
    language: String,
    /// The romanized spelling of the word
    word: String,
    /// The meaning of the word
    #[arg(short, long)]
    meaning: Option<String>,
    /// The part-of-speech the word belond to (v, n, adv, adj, inj, conj, adp)
    #[arg(short, long)]
    kind: Option<String>,
    /// Attach a note to the word (arbitrary text)
    #[arg(short, long)]
    note: Option<String>,
}

#[derive(Args, Debug)]
struct DelArgs {
    /// The language to add the word to
    language: String,
    /// The romanized spelling of the word
    word: String,
}

#[derive(Args, Debug)]
struct EvolveArgs {
    /// The source language
    from_lang: String,
    /// The target language
    to_lang: String,
    /// The sentence to evolve
    sentence: Vec<String>,
    #[arg(short = 'b')]
    stop_before: Option<String>,
    #[arg(short = 'a')]
    start_at: Option<String>,
    #[arg(short = 'p')]
    show_phonetic: bool,
    /// Show intermediate versions
    #[arg(short = 'i', long)]
    show_intermediate: bool,
}

#[derive(Args, Debug)]
struct PhonArgs {
    #[arg(short, long = "lang")]
    language: Option<String>,
    /// Regenerate ALL phonetic annotation, not just the missing ones
    #[arg(short, long)]
    force: bool,
}

/*
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum OriginKind {
    Inherited {
        from_lang: String,
        romanization: String,
        word_id: u32,
    },
    Compound {

    }
}
*/

#[derive(Debug, Serialize)]
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
    fn from_row(row: &Row) -> rusqlite::Result<WordEntry> {
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

#[derive(Debug, Serialize)]
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
    fn from_row(row: &Row) -> rusqlite::Result<LangEntry> {
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum LexurgyInput {
    Romanized,
    Phonetic,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum LexurgyOutput {
    Romanized,
    Phonetic,
    Both,
}

struct LexurgyCmd<'a> {
    target_lang: &'a LangEntry,
    output: Option<String>,
    evolve: bool,
    input_format: LexurgyInput,
    output_format: LexurgyOutput,
    stop_before: Option<String>,
    start_at: Option<String>,
}

enum WordOutput {
    Phon(String),
    Rom(String),
    PhonRom(String, String),
}

impl WordOutput {
    fn get_value_ref(&self) -> Result<&str> {
        match self {
            WordOutput::Phon(x) => Ok(x),
            WordOutput::Rom(x) => Ok(x),
            WordOutput::PhonRom(_, _) => {
                bail!("Expected a single value, but this word output has two")
            }
        }
    }

    fn get_rom_ref(&self) -> Result<&str> {
        match self {
            WordOutput::Phon(_) => bail!("Expected romanized word, got phonetic word"),
            WordOutput::Rom(x) => Ok(x),
            WordOutput::PhonRom(_, x) => Ok(x),
        }
    }

    fn get_phon_ref(&self) -> Result<&str> {
        match self {
            WordOutput::Phon(x) => Ok(x),
            WordOutput::Rom(_) => bail!("Expected phonetic word, got romanized word"),
            WordOutput::PhonRom(x, _) => Ok(x),
        }
    }

    fn get_phon_rom_ref(&self) -> Result<(&str, &str)> {
        match self {
            WordOutput::PhonRom(p, r) => Ok((p, r)),
            _ => bail!("Expected both phonetic and romanized versions of the word"),
        }
    }

    fn get_value(self) -> Result<String> {
        match self {
            WordOutput::Phon(x) => Ok(x),
            WordOutput::Rom(x) => Ok(x),
            WordOutput::PhonRom(_, _) => {
                bail!("Expected a single value, but this word output has two")
            }
        }
    }

    fn get_rom(self) -> Result<String> {
        match self {
            WordOutput::Phon(_) => bail!("Expected romanized word, got phonetic word"),
            WordOutput::Rom(x) => Ok(x),
            WordOutput::PhonRom(_, x) => Ok(x),
        }
    }

    fn get_phon(self) -> Result<String> {
        match self {
            WordOutput::Phon(x) => Ok(x),
            WordOutput::Rom(_) => bail!("Expected phonetic word, got romanized word"),
            WordOutput::PhonRom(x, _) => Ok(x),
        }
    }

    fn get_phon_rom(self) -> Result<(String, String)> {
        match self {
            WordOutput::PhonRom(p, r) => Ok((p, r)),
            _ => bail!("Expected both phonetic and romanized versions of the word"),
        }
    }
}

impl<'a> LexurgyCmd<'a> {
    fn evolve(to: &'a LangEntry, inp: LexurgyInput, out: LexurgyOutput) -> LexurgyCmd<'a> {
        LexurgyCmd {
            target_lang: to,
            output: None,
            evolve: true,
            input_format: inp,
            output_format: out,
            stop_before: None,
            start_at: None,
        }
    }

    fn deromanize(lang: &'a LangEntry) -> LexurgyCmd<'a> {
        LexurgyCmd {
            target_lang: lang,
            output: None,
            evolve: false,
            input_format: LexurgyInput::Romanized,
            output_format: LexurgyOutput::Phonetic,
            stop_before: None,
            start_at: None,
        }
    }

    fn run<'b>(
        self,
        cfg: &Config,
        words: impl Iterator<Item = &'b str>,
    ) -> Result<Vec<WordOutput>> {
        use std::fs::File;
        use std::io::{BufRead, BufReader, BufWriter, Write};
        use std::process::*;

        let input_name = format!(
            "{}_{}",
            &self.target_lang.id,
            if self.input_format == LexurgyInput::Romanized {
                "rom"
            } else {
                "phon"
            }
        );
        let mut wli = PathBuf::new();
        wli.push(cfg.word_list_folder());
        wli.push(&input_name);
        wli.set_extension("wli");

        {
            let f = File::create(&wli)?;
            let mut buf = BufWriter::new(f);

            for word in words {
                buf.write_all(word.as_bytes())?;
                buf.write_all(b"\n")?;
            }
        }

        let mut lsc = PathBuf::new();
        lsc.push(cfg.rule_list_folder());
        lsc.push(&self.target_lang.rule);
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

        let derom = self.input_format == LexurgyInput::Romanized;

        let mut start = None;

        if derom && !self.evolve {
            if self.stop_before.is_some() {
                bail!("Can't specify `--stop-before` together with a pure deromanize command");
            }
            lexurgy.arg("-b").arg("init");
        } else if !derom && self.evolve {
            start = Some("init");
        } else if !derom && !self.evolve {
            bail!(
                "Internal error! It doesn't make sense to neither want to deromanize nor to evovle"
            );
        }

        if let Some(a) = self.start_at.as_ref().map(|x| &x[..]).or(start) {
            lexurgy.arg("-a").arg(a);
        }

        if let Some(b) = self.stop_before.as_ref() {
            lexurgy.arg("-b").arg(b);
        }

        match self.output_format {
            LexurgyOutput::Phonetic => {
                lexurgy.arg("-p");
            }
            LexurgyOutput::Both => {
                lexurgy.arg("-m");
            }
            LexurgyOutput::Romanized => {}
        }

        if cfg.debug_mode {
            println!("Running lexurgy with: {:?}", lexurgy.get_args());
        }
        let cmd = format!("{:?}", &lexurgy);
        let output = lexurgy.output()?;
        if !output.status.success() {
            bail!(
                "{:?} failed.\nSTDOUT:\n{}\nSTDERR:\n{}\n",
                cmd,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let mut ev_wli = PathBuf::new();
        ev_wli.push(cfg.word_list_folder());
        ev_wli.push("out");
        ev_wli.push(format!("{}_ev", &input_name));
        if self.output_format == LexurgyOutput::Both {
            ev_wli.set_extension("wlm");
        } else {
            ev_wli.set_extension("wli");
        }
        let f = File::open(ev_wli)?;
        let reader = BufReader::new(f);
        Ok(reader
            .lines()
            .map(|l| match self.output_format {
                LexurgyOutput::Romanized => Ok(WordOutput::Rom(l?)),
                LexurgyOutput::Phonetic => Ok(WordOutput::Phon(l?)),
                LexurgyOutput::Both => {
                    let l = l?;
                    let mut parts = l.split("=>").map(|p| p.trim()).collect::<Vec<_>>();
                    if parts.len() < 3 {
                        bail!("Expected at least three steps, got: {}", parts.len());
                    }
                    let rom = parts
                        .pop()
                        .ok_or(anyhow!("Expected romanized part of output"))?;
                    let phon = parts
                        .pop()
                        .ok_or(anyhow!("Expecetd phonetic part of ouput"))?;
                    Ok(WordOutput::PhonRom(phon.to_string(), rom.to_string()))
                }
            })
            .collect::<Result<_, _>>()?)
    }
}

impl Wdb {
    fn new(cfg: Config) -> Result<Wdb> {
        let db_file = if cfg.debug_mode {
            cfg.root.join(&cfg.settings.db_dev_file)
        } else {
            cfg.root.join(&cfg.settings.db_file)
        };
        Ok(Wdb {
            db: Connection::open(db_file)?,
            cfg,
        })
    }

    fn get_lang(&self, lang: &str) -> Result<LangEntry> {
        Ok(self.db.query_row(
            "SELECT * FROM langs WHERE id = ?",
            [lang],
            LangEntry::from_row,
        )?)
    }

    fn get_langs(&self) -> Result<Vec<LangEntry>> {
        let mut stmt = self.db.prepare("SELECT * FROM langs")?;
        let entries = stmt.query_map([], LangEntry::from_row)?;
        Ok(entries.collect::<Result<_, _>>()?)
    }

    fn dump(&mut self, args: DumpArgs) -> Result<()> {
        let lang = self.get_lang(&args.language).expect("Invalid language");
        let mut stmt = self
            .db
            .prepare("SELECT * FROM words WHERE lang = ? ORDER BY romanization")?;
        let entries = stmt
            .query_map([&lang.id], WordEntry::from_row)?
            .collect::<Result<_, _>>()?;
        let mut tt = TinyTemplate::new();
        tt.add_template(
            "dictionary_file",
            &self.cfg.settings.dictionary_file_template,
        )?;
        tt.add_template("dictionary", &self.cfg.settings.dictionary_template)?;

        #[derive(Serialize)]
        struct DictionaryTemplateContext {
            lang: LangEntry,
            words: Vec<WordEntry>,
        }

        let context = DictionaryTemplateContext {
            lang,
            words: entries,
        };

        let mut dict_file = self.cfg.root.to_path_buf();
        dict_file.push(tt.render("dictionary_file", &context)?);
        fs::write(&dict_file, tt.render("dictionary", &context)?)
            .with_context(|| format!("Writing dictionary file: {:?}", &dict_file))?;

        Ok(())
    }

    fn list(&mut self) -> Result<()> {
        println!("Languages:");
        for entry in self.get_langs()? {
            let words: u32 = self.db.query_row(
                "SELECT COUNT(id) FROM words WHERE lang = ?",
                [&entry.id],
                |row| row.get(0),
            )?;
            println!(" {}: {} ({} words)", entry.id, entry.name, words);
        }
        Ok(())
    }

    fn add(&mut self, args: AddArgs) -> Result<()> {
        println!("{:?}", args);
        let lang = self.get_lang(&args.language)?;
        let rom = normalize_text(&args.word);
        // Make sure there isn't already another word in the db if it's not supposed to be a homophone
        if !args.homophone {
            let mut stmt = self
                .db
                .prepare("SELECT * FROM words WHERE romanization = ? AND lang = ?")?;
            let homophones: Vec<_> = stmt
                .query_map([&rom, &lang.id], WordEntry::from_row)?
                .collect::<Result<_, _>>()?;
            if !homophones.is_empty() {
                use std::fmt::Write;
                let mut err_msg =
                format!("Error adding word `{}` to language {}. \n\nThe following homophone(s) exist already:\n", &rom, lang);
                for h in homophones {
                    write!(
                        &mut err_msg,
                        " - {}: {}, {}\n",
                        h.romanization, h.meaning, h.kind
                    )?;
                }

                write!(
                    &mut err_msg,
                    "\nIf you want to add it as a homophone, use the -H flag."
                )?;
                bail!(err_msg);
            }
        }

        let mut phon: Option<String> = args.ipa;

        if phon.is_none() && !args.disable_autorom {
            println!("Reromanization...");
            let mut phons =
                LexurgyCmd::deromanize(&lang).run(&self.cfg, std::iter::once(&rom[..]))?;
            if phons.len() != 1 {
                bail!("expected a single word back, got {}", phons.len());
            }
            let p = phons.remove(0).get_phon()?;
            println!("  {} => {}", &rom, &p);
            phon = Some(p);
        }

        let _ = self.db.execute(
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
        )?;
        println!("Added `{}` to {}", &args.word, lang);
        if self.cfg.settings.auto_dump {
            self.dump(DumpArgs {
                language: args.language,
            })?;
        }
        Ok(())
    }

    fn try_get_unique_word(&self, lang: &LangEntry, mut rom: &str) -> Result<Option<WordEntry>> {
        use std::fmt::Write;
        let mut index = None;

        if let Some((r, i)) = rom.split_once('#') {
            index = Some(i.parse::<usize>()?);
            rom = r;
        }

        let mut stmt = self.db.prepare(
            "SELECT * FROM words WHERE lang = ? AND romanization = ? ORDER BY romanization",
        )?;
        let mut words = stmt
            .query_map(params!(&lang.id, &rom), WordEntry::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        if words.is_empty() {
            bail!("No matching words found");
        }

        if let Some(index) = index {
            if index >= words.len() {
                let mut err_msg = format!("Index `{}` out of bounds for homophone list:", index);
                for (i, h) in words.iter().enumerate() {
                    write!(
                        &mut err_msg,
                        " {}:  {}: {}, {}\n",
                        i, h.romanization, h.meaning, h.kind
                    )?;
                }
                bail!(err_msg);
            }
            return Ok(Some(words.swap_remove(index)));
        }

        if words.len() == 1 {
            return Ok(Some(words.swap_remove(0)));
        }

        let mut err_msg = format!("`{0}` has homophones, please specify by passing `{0}#N` where N is one of the following indices:\n", &rom);
        for (i, h) in words.iter().enumerate() {
            write!(
                &mut err_msg,
                " {}:  {}: {}, {}\n",
                i, h.romanization, h.meaning, h.kind
            )?;
        }
        bail!(err_msg);
    }

    fn edit(&mut self, args: EditArgs) -> Result<()> {
        use rusqlite::ToSql;
        use std::fmt::Write;
        let lang = self.get_lang(&args.language)?;
        let rom = normalize_text(&args.word);
        if let Some(entry) = self.try_get_unique_word(&lang, &rom)? {
            let mut changed = format!("Changed the following for `{}`:\n", rom);
            let mut query_str = "UPDATE words SET ".to_string();
            let mut first = true;
            let fields = &[
                ("meaning", Some(&entry.meaning), &args.meaning),
                ("kind", Some(&entry.kind), &args.kind),
                ("origin", entry.origin.as_ref(), &args.origin),
                ("note", entry.note.as_ref(), &args.note),
            ];
            for (fld, old, val) in fields {
                if let Some(v) = val {
                    if !first {
                        query_str.push_str(", ");
                    }
                    write!(&mut query_str, "{} = ?", fld)?;
                    write!(
                        &mut changed,
                        " {}: {} => {}\n",
                        fld,
                        old.unwrap_or(&"<unset>".to_string()),
                        v
                    )?;
                }
                first = false;
            }
            query_str.push_str(" WHERE id = ?");
            let vs = fields
                .iter()
                .filter_map(|(_, _, val)| val.as_ref().map(|v| v.to_sql()))
                .chain(iter::once(entry.id.to_sql()))
                .collect::<Result<Vec<_>, _>>()?;
            let _ = self
                .db
                .execute(&query_str[..], rusqlite::params_from_iter(vs.iter()))?;
            println!("{}", changed);
            if self.cfg.settings.auto_dump {
                self.dump(DumpArgs {
                    language: args.language,
                })?;
            }
        }
        Ok(())
    }

    fn del(&mut self, args: DelArgs) -> Result<()> {
        let lang = self.get_lang(&args.language)?;
        let rom = normalize_text(&args.word);
        if let Some(entry) = self.try_get_unique_word(&lang, &rom)? {
            let _ = self
                .db
                .execute("DELETE FROM words WHERE id = ?", [entry.id])?;
            println!(
                "Deleted: {}: {} ({})",
                entry.romanization, entry.meaning, entry.kind
            );
            if self.cfg.settings.auto_dump {
                self.dump(DumpArgs {
                    language: args.language,
                })?;
            }
        }
        Ok(())
    }

    fn evolve(&mut self, args: EvolveArgs) -> Result<()> {
        let langs = self.get_langs()?;
        let from = langs
            .iter()
            .find(|l| l.id == args.from_lang)
            .ok_or(anyhow!("No such 'from' language: `{}`", args.from_lang))?;
        let to = langs
            .iter()
            .find(|l| l.id == args.to_lang)
            .ok_or(anyhow!("No such 'to' language: `{}`", args.to_lang))?;

        if from.id == to.id {
            bail!("'from' and 'to' language are the same. Nothing to evolve");
        }

        let mut steps = vec![];
        let mut l = to;
        while let Some(ref l_id) = l.origin {
            steps.push(l);

            l = langs.iter().find(|l| &l.id == l_id).ok_or(anyhow!(
                "Internal Error! Language {}({}) has an invalid origin language: `{}`",
                l.name,
                l.id,
                l_id
            ))?;

            if l_id == &from.id {
                break;
            }
        }

        if l.id != from.id {
            bail!(
                "{}({}) is not a descendent of {}({})!",
                to.name,
                to.id,
                from.name,
                from.id
            );
        }

        let mut tokens = vec![];
        for sentence_fragment in args.sentence {
            tokens.extend(sentence_fragment.split(' ').map(|f| f.replace('-', " ")));
        }

        let mut first = true;
        for step in steps.iter().rev() {
            let last = step.id == to.id;
            let mut cmd = LexurgyCmd::evolve(
                step,
                if first {
                    LexurgyInput::Romanized
                } else {
                    LexurgyInput::Phonetic
                },
                if last {
                    if args.show_phonetic {
                        LexurgyOutput::Phonetic
                    } else {
                        LexurgyOutput::Romanized
                    }
                } else {
                    LexurgyOutput::Phonetic
                },
            );
            if last {
                cmd.stop_before = args.stop_before.clone();
            }
            if first {
                cmd.start_at = args.start_at.clone();
            }
            let new_tokens = cmd.run(&self.cfg, tokens.iter().map(|x| &x[..]))?;
            tokens.clear();
            for tok in new_tokens {
                tokens.push(tok.get_value()?);
            }
            first = false;
            if last || args.show_intermediate {
                print!("{}: ", step.id);
                for tok in &tokens {
                    print!("{} ", tok);
                }
                println!();
            }
        }

        Ok(())
    }

    fn inherit(&mut self, args: InheritArgs) -> Result<()> {
        let dest_lang = self.get_lang(&args.language)?;
        if dest_lang.origin.is_none() {
            bail!("There no parent language to inherit from!");
        }
        let src_lang = self.get_lang(dest_lang.origin.as_ref().unwrap())?;
        let mut words: Vec<WordEntry> = Vec::new();
        if args.word == "*" {
            let mut stmt = self.db.prepare("SELECT * FROM words WHERE lang = ?")?;
            words = stmt
                .query_map(params![&src_lang.id], WordEntry::from_row)?
                .collect::<Result<Vec<_>, _>>()?;
        } else {
            if let Some(word) = self.try_get_unique_word(&src_lang, &args.word)? {
                words.push(word);
            } else {
                return Ok(());
            }
        }

        let phon = words
            .iter()
            .map(|w| {
                w.ipa.as_ref().map(|p| &p[..]).ok_or(anyhow!(
                    "The inherited words must have a phonetic annotation"
                ))
            })
            .collect::<Result<Vec<&str>>>()?;
        println!("Applying sound changes..");
        let evolved = LexurgyCmd::evolve(&dest_lang, LexurgyInput::Phonetic, LexurgyOutput::Both)
            .run(&self.cfg, phon.into_iter())?;
        if evolved.len() != words.len() {
            bail!(
                "Expected {} resulting word, got: {}",
                words.len(),
                evolved.len()
            );
        }
        let tr = self.db.transaction()?;
        for (word, output) in words.iter().zip(evolved.into_iter()) {
            let (phon, rom) = output.get_phon_rom()?;
            println!(
                "  {} ({}) => {} ({})",
                &word.romanization,
                word.ipa.as_ref().unwrap(),
                &rom,
                &phon
            );
            let _ = tr.execute(
                "INSERT INTO words
                (lang, romanization, ipa, meaning, kind, note, origin, flags)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    dest_lang.id,
                    rom,
                    phon,
                    args.meaning.as_ref().unwrap_or(&word.meaning),
                    args.kind.as_ref().unwrap_or(&word.kind),
                    args.note.as_ref().or(word.note.as_ref()),
                    format!(
                        "(inherited {} {} {})",
                        src_lang.id, word.romanization, word.id
                    ),
                    word.flags
                ],
            )?;
        }
        let _ = tr.commit()?;
        if self.cfg.settings.auto_dump {
            self.dump(DumpArgs {
                language: args.language,
            })?;
        }
        Ok(())
    }

    fn check_missing_ipa(&mut self) -> Result<()> {
        let mut stmt = self
            .db
            .prepare("SELECT * FROM words WHERE ipa IS NULL ORDER BY lang DESC, romanization")?;
        let words = stmt
            .query_map([], WordEntry::from_row)?
            .collect::<rusqlite::Result<Vec<WordEntry>>>()?;

        if words.is_empty() {
            return Ok(());
        }

        println!("\nNOTE: The following words are missing phonetic annotation:");
        for lang_words in words.chunk_by(|w0, w1| w0.lang == w1.lang) {
            println!("{}", self.get_lang(&lang_words[0].lang)?);
            for word in lang_words {
                println!("- {}: {} ({})", word.romanization, word.meaning, word.kind);
            }
        }
        println!("\nRun `wdb phon` to generate phonetic annotations based on their romanization.");
        Ok(())
    }

    fn deromanize(&mut self, args: PhonArgs) -> Result<()> {
        let languages = args
            .language
            .as_ref()
            .map(|l| self.get_lang(l).map(|x| vec![x]))
            .unwrap_or_else(|| self.get_langs())?;
        let mut any_change = false;
        for lang in languages {
            let words: Vec<WordEntry> = {
                let mut stmt = self.db.prepare(&format!(
                    "SELECT * FROM words WHERE lang = ? {}",
                    if args.force { "" } else { "AND ipa IS NULL" }
                ))?;
                let ws = stmt
                    .query_map([&lang.id], WordEntry::from_row)?
                    .collect::<Result<_, _>>()?;
                ws
            };

            if words.is_empty() {
                continue;
            }

            any_change = true;

            let lexurgy = LexurgyCmd::deromanize(&lang);
            println!("Running `{}` deromanization rule...", &lang.rule);
            let phons = lexurgy
                .run(&self.cfg, words.iter().map(|w| &w.romanization[..]))?
                .into_iter()
                .map(|o| o.get_phon())
                .collect::<Result<Vec<_>>>()?;
            if phons.len() != words.len() {
                println!(
                    "Number of words out ({}) doesn't match number of words in({})!'",
                    phons.len(),
                    words.len()
                );
            }
            for (word, phon) in words.iter().zip(phons.iter()) {
                println!(" {} => {}", &word.romanization, phon);
            }

            let mut write_phons = || {
                let tr = self.db.transaction()?;
                for (word, phon) in words.iter().zip(phons.iter()) {
                    tr.execute(
                        "UPDATE words SET ipa = ? WHERE id = ?",
                        params![phon, word.id],
                    )?;
                }
                tr.commit()
            };

            if let Err(err) = write_phons() {
                bail!(
                    "failed to update phonetic annotation. No words changed.\n{}",
                    err
                );
            } else {
                println!("Updated {} word entries", words.len());
                if words.len() != 0 {
                    if self.cfg.settings.auto_dump {
                        self.dump(DumpArgs { language: lang.id })?;
                    }
                }
            }
        }

        if !any_change {
            println!("No word updated, every word present has a phonetic annotation.\nIf you want to update all anyway, use the -f flag.");
        }
        Ok(())
    }
}

fn find_obsidian_root() -> Result<PathBuf> {
    let cur = env::current_dir()?;
    let mut obsidian = PathBuf::new();
    for root in cur.ancestors() {
        obsidian.clear();
        obsidian.push(root);
        obsidian.push(".obsidian");
        if obsidian.exists() {
            return Ok(root.into());
        }
    }
    bail!("you must run this command from inside of an Obsidian vault!");
}

fn load_settings(root: &path::Path) -> Result<Settings> {
    Ok(toml::from_str(
        &fs::read_to_string(root.join("Wdb.toml")).expect("No `Wdb.toml` settings file present"),
    )?)
}

fn main() -> Result<()> {
    let mut cli = Cli::parse();
    let root = find_obsidian_root()?;
    let settings = load_settings(&root)?;
    let cfg = Config::new(root, settings, cli.debug_mode | cfg!(debug_assertions));
    if cfg.debug_mode {
        println!(
            "NOTE: Running in debug, changes are done to the `{}` instead of `{}`\n",
            &cfg.settings.db_dev_file, &cfg.settings.db_file,
        );
    }

    let mut wdb = Wdb::new(cfg)?;
    let mut cmd = cli.command;
    let interactive = if let Some(Command::Interactive) = cmd {
        true
    } else {
        false
    };
    let mut buf = String::new();

    loop {
        match cmd {
            Some(Command::Dump(args)) => wdb.dump(args)?,
            Some(Command::List) => wdb.list()?,
            Some(Command::Add(args)) => wdb.add(args)?,
            Some(Command::Edit(args)) => wdb.edit(args)?,
            Some(Command::Del(args)) => wdb.del(args)?,
            Some(Command::Evolve(args)) => wdb.evolve(args)?,
            Some(Command::Inherit(args)) => wdb.inherit(args)?,
            Some(Command::Phon(args)) => {
                cli.disable_checks = args.language.is_none();
                wdb.deromanize(args)?
            }
            _ => {}
        }
        if !interactive {
            break;
        }
        loop {
            std::io::stdin().read_line(&mut buf)?;
            match Cli::try_parse_from(buf.split(' ')) {
                Err(err) => {
                    println!("Failed to parse command: {:?}", err)
                }
                Ok(Cli { command: c, .. }) => {
                    cmd = c;
                    break;
                }
            }
        }
    }

    if !cli.disable_checks {
        wdb.check_missing_ipa()?;
    }
    Ok(())
}
