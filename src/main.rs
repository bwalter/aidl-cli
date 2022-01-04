use std::{
    collections::HashMap,
    io::{Read, Write},
    path::Path,
    path::PathBuf,
};

use aidl_parser::ast;
use aidl_parser::ParseFileResult;
use anyhow::Result;
use codespan_reporting::{
    files::SimpleFiles,
    term::{self, termcolor},
};
use structopt::StructOpt;
use walkdir::WalkDir;

#[derive(Debug, StructOpt)]
struct Opt {
    /// Make pretty (but longer) messages
    #[structopt(long)]
    pretty: bool,

    /// Do not show diagnostics
    #[structopt(short = "q", long = "hide-diagnostics")]
    hide_diagnostics: bool,

    /// Display items
    #[structopt(short = "i", long = "items")]
    display_items: bool,

    /// Convert the whole AST to JSON
    #[structopt(short = "j", long)]
    to_json: bool,

    /// Output file
    #[structopt(short = "o", long, parse(from_os_str))]
    output_path: Option<PathBuf>,

    /// The directory where the AIDL files are located
    #[structopt(parse(from_os_str))]
    dir: PathBuf,
}

fn main() -> Result<()> {
    // Command line options
    let opt = Opt::from_args();

    // Tracing
    let subscriber_builder = tracing_subscriber::fmt().with_thread_names(true);
    if opt.pretty {
        subscriber_builder.pretty().init()
    } else {
        subscriber_builder.compact().init()
    }

    // Parse files
    let files = SimpleFiles::new();
    parse(files, &opt)?;

    Ok(())
}

fn parse(mut files: SimpleFiles<String, String>, opt: &Opt) -> Result<()> {
    let root_path = opt.dir.as_path();

    // Walk through the directory and find all AIDL files
    let dir_entries = WalkDir::new(&opt.dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_lowercase() == "aidl")
                .unwrap_or(false)
        });

    // Parse all files
    let mut parser = aidl_parser::Parser::new();
    for e in dir_entries {
        eprint!(".");
        std::io::stdout().flush().unwrap();

        let mut file = std::fs::File::open(e.path()).unwrap();
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).unwrap();

        let id = files.add(
            e.path()
                .strip_prefix(root_path)?
                .to_string_lossy()
                .to_string(),
            buffer.clone(),
        );
        parser.add_content(id, &buffer);
    }
    let parse_results = parser.validate();
    eprintln!();

    // Display diagnostics
    if !opt.hide_diagnostics {
        report(&files, &parse_results, opt)?;
    }

    // Display all items
    if opt.display_items {
        for (id, res) in &parse_results {
            let item = match res.ast.as_ref().map(|f| &f.item) {
                Some(i) => i,
                None => continue,
            };

            let file_path: PathBuf = files.get(*id)?.name().into();

            display_item(item, &file_path);
        }
    }

    // Convert to JSON
    if opt.to_json {
        convert_to_json(&files, &parse_results, opt)?;
    }

    Ok(())
}

// Display 1 item
fn display_item(item: &ast::Item, file_path: &Path) {
    let (item_name, item_line) = match item {
        ast::Item::Interface(i @ ast::Interface { name, .. }) => (
            format!("interface {}", name),
            i.symbol_range.start.line_col.0,
        ),
        ast::Item::Parcelable(p @ ast::Parcelable { name, .. }) => (
            format!("parcelable {}", name),
            p.symbol_range.start.line_col.0,
        ),
        ast::Item::Enum(e @ ast::Enum { name, .. }) => {
            (format!("enum {}", name), e.symbol_range.start.line_col.0)
        }
    };

    println!(
        "{} (in {}:{})",
        item_name,
        file_path
            .file_name()
            .map(|s| s.to_string_lossy())
            .unwrap_or_else(|| "<invalid file name>".into()),
        item_line,
    );
}
// Display all diagnostics with colors
fn report(
    files: &SimpleFiles<String, String>,
    results: &HashMap<usize, ParseFileResult<usize>>,
    _opt: &Opt,
) -> Result<()> {
    // Flat-map all diagnostics
    let errors = results.iter().flat_map(|(id, r)| {
        r.diagnostics
            .iter()
            .map(|d| to_codespan_diagnostic(*id, d))
            .collect::<Vec<_>>()
    });

    // Term writer and config
    let writer = termcolor::StandardStream::stderr(termcolor::ColorChoice::Always);
    let config = term::Config::default();

    // Display diagnostics in the terminal
    for s in errors {
        term::emit(&mut writer.lock(), &config, files, &s)?;
    }

    Ok(())
}

// Convert aidl-parser Diagnostic into codespan_reporting Diagnostic
fn to_codespan_diagnostic(
    id: usize,
    d: &aidl_parser::diagnostic::Diagnostic,
) -> codespan_reporting::diagnostic::Diagnostic<usize> {
    let mut main_label = codespan_reporting::diagnostic::Label::primary(
        id,
        d.range.start.offset..d.range.end.offset,
    );
    if let Some(ref context_msg) = d.context_message {
        main_label = main_label.with_message(context_msg.clone());
    }
    let mut labels = Vec::from([main_label]);

    for info in d.related_infos.iter() {
        labels.push(
            codespan_reporting::diagnostic::Label::secondary(
                id,
                info.range.start.offset..info.range.end.offset,
            )
            .with_message(info.message.clone()),
        )
    }

    let diagnostic = match d.kind {
        aidl_parser::diagnostic::DiagnosticKind::Error => {
            codespan_reporting::diagnostic::Diagnostic::error()
        }
        aidl_parser::diagnostic::DiagnosticKind::Warning => {
            codespan_reporting::diagnostic::Diagnostic::warning()
        }
    };

    diagnostic
        .with_message(&d.message)
        .with_labels(labels)
        .with_notes(match d.hint.clone() {
            Some(h) => Vec::from([h]),
            None => Vec::new(),
        })
}

fn convert_to_json(
    files: &SimpleFiles<String, String>,
    parse_results: &HashMap<usize, ParseFileResult<usize>>,
    opt: &Opt,
) -> Result<()> {
    #[derive(serde_derive::Serialize)]
    struct AidlJson<'a> {
        root: String,
        items: HashMap<String, AidlJsonItem<'a>>,
    }

    #[derive(serde_derive::Serialize)]
    #[serde(transparent)]
    struct AidlJsonItem<'a> {
        item: Option<&'a aidl_parser::ast::Item>,
    }

    let items = parse_results
        .iter()
        .map(|(id, res)| {
            let path = PathBuf::from(files.get(*id).unwrap().name())
                .to_string_lossy()
                .to_string();
            let item = AidlJsonItem {
                item: res.ast.as_ref().map(|f| &f.item),
            };
            (path, item)
        })
        .collect();

    let aidl_json = AidlJson {
        root: std::env::current_dir()?.to_string_lossy().to_string(),
        items,
    };

    // Simplify JSON (e.g. remove range info and nested types, map arrays to maps, ...)
    // TODO: make it possible to keep full info!
    let json = match map_json_value(serde_json::to_value(&aidl_json)?) {
        Some(json) => serde_json::to_string_pretty(&json)?,
        None => return Ok(()),
    };

    if let Some(path) = opt.output_path.as_ref() {
        // Write JSON to output file
        let path = std::fs::canonicalize(path)?;
        let mut file = std::fs::File::create(&path)?;
        writeln!(file, "{}\n", json)?;
    } else {
        // Write JSON to stdout
        println!("{}\n", json)
    };

    Ok(())
}

fn map_json_value(value: serde_json::Value) -> Option<serde_json::Value> {
    match value {
        serde_json::Value::Null => Some(value),
        serde_json::Value::Bool(_) => Some(value),
        serde_json::Value::Number(_) => Some(value),
        serde_json::Value::String(_) => Some(value),
        serde_json::Value::Array(a) => Some(serde_json::Value::Array(
            a.into_iter().map(map_json_value).flatten().collect(),
        )),
        serde_json::Value::Object(o) => Some(serde_json::Value::Object(
            o.into_iter()
                .filter_map(|(k, v)| map_json_field(&k, v).map(|v| (k, v)))
                .map(|(k, v)| map_json_value(v).map(|v| (k, v)))
                .flatten()
                .collect(),
        )),
    }
}

fn map_json_field(key: &str, value: serde_json::Value) -> Option<serde_json::Value> {
    if key.ends_with("_range") {
        return None;
    }

    match (key, value) {
        ("direction", serde_json::Value::Object(o)) => {
            if o.len() == 1 {
                let direction_name = o.into_iter().next().unwrap().0;
                Some(serde_json::Value::String(direction_name))
            } else {
                None
            }
        }
        ("parcelable", serde_json::Value::Object(o)) => map_json_parcelable(o),
        ("interface", serde_json::Value::Object(o)) => map_json_interface(o),
        ("type" | "return_type", serde_json::Value::Object(o)) => map_json_type(o),
        (_, value) => Some(value),
    }
}

fn map_json_parcelable(
    mut o: serde_json::Map<String, serde_json::Value>,
) -> Option<serde_json::Value> {
    if let Ok(ast_parcelable) =
        serde_json::from_value::<aidl_parser::ast::Parcelable>(serde_json::Value::Object(o.clone()))
    {
        // Deserialize parcelable
        let new_members: HashMap<String, aidl_parser::ast::Member> = ast_parcelable
            .members
            .iter()
            .map(|el| (el.name.to_owned(), el.clone()))
            .collect();

        let members = o.get_mut("members")?;
        *members = serde_json::to_value(&new_members).ok()?;
    }

    Some(serde_json::Value::Object(o))
}

fn map_json_interface(
    mut o: serde_json::Map<String, serde_json::Value>,
) -> Option<serde_json::Value> {
    if let Ok(ast_interface) =
        serde_json::from_value::<aidl_parser::ast::Interface>(serde_json::Value::Object(o.clone()))
    {
        // Deserialize interface
        let new_elements: HashMap<String, aidl_parser::ast::InterfaceElement> = ast_interface
            .elements
            .iter()
            .map(|el| (el.get_name().to_owned(), el.clone()))
            .collect();

        let elements = o.get_mut("elements")?;
        *elements = serde_json::to_value(&new_elements).ok()?;
    }

    Some(serde_json::Value::Object(o))
}

fn map_json_type(o: serde_json::Map<String, serde_json::Value>) -> Option<serde_json::Value> {
    if let Ok(ast_type) =
        serde_json::from_value::<aidl_parser::ast::Type>(serde_json::Value::Object(o.clone()))
    {
        // Deserialize type
        let qualified_name = aidl_parser::symbol::Symbol::Type(&ast_type).get_signature()?;
        Some(serde_json::Value::String(qualified_name))
    } else {
        // Default to given one
        Some(serde_json::Value::Object(o))
    }
}
