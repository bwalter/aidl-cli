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

mod model;

#[derive(Debug, StructOpt)]
struct Opt {
    /// Make pretty (but longer) output
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

    /// Convert the whole AST to YAML
    #[structopt(short = "y", long)]
    to_yaml: bool,

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
    convert(&files, &parse_results, opt)?;

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

fn convert(
    files: &SimpleFiles<String, String>,
    parse_results: &HashMap<usize, ParseFileResult<usize>>,
    opt: &Opt,
) -> Result<()> {
    enum OutputKind {
        Json,
        Yaml,
    }
    let output_kind = if opt.to_json {
        OutputKind::Json
    } else if opt.to_yaml {
        OutputKind::Yaml
    } else {
        return Ok(());
    };

    let items = parse_results
        .iter()
        .filter_map(|(id, res)| {
            res.ast.as_ref().map(|ast| {
                let path = PathBuf::from(files.get(*id).unwrap().name())
                    .to_string_lossy()
                    .to_string();
                let item = match &ast.item {
                    ast::Item::Interface(i) => convert_interface(path, i),
                    ast::Item::Parcelable(p) => convert_parcelable(path, p),
                    ast::Item::Enum(e) => convert_enum(path, e),
                };
                (
                    format!("{}.{}", ast.package.name, ast.item.get_name()),
                    item,
                )
            })
        })
        .collect();

    let aidl = model::Aidl {
        root: std::env::current_dir()?.to_string_lossy().to_string(),
        items,
    };

    let output = match output_kind {
        OutputKind::Json => {
            if opt.pretty {
                serde_json::to_string_pretty(&aidl)?
            } else {
                serde_json::to_string(&aidl)?
            }
        }
        OutputKind::Yaml => serde_yaml::to_string(&aidl)?,
    };

    if let Some(path) = opt.output_path.as_ref() {
        // Write JSON to output file
        let path = std::fs::canonicalize(path)?;
        let mut file = std::fs::File::create(&path)?;
        writeln!(file, "{}\n", output)?;
    } else {
        // Write JSON to stdout
        println!("{}\n", output)
    };

    Ok(())
}

fn convert_interface(path: String, i: &ast::Interface) -> model::Item {
    let elements = i
        .elements
        .iter()
        .map(|el| match el {
            ast::InterfaceElement::Const(c) => (
                c.name.clone(),
                model::Element::Const {
                    name: c.name.clone(),
                    const_type: model::ast_type_to_string(&c.const_type),
                    value: c.value.clone(),
                },
            ),
            ast::InterfaceElement::Method(m) => (
                m.name.clone(),
                model::Element::Method {
                    oneway: m.oneway,
                    name: m.name.clone(),
                    return_type: model::ast_type_to_string(&m.return_type),
                    args: m
                        .args
                        .iter()
                        .map(|a| model::Arg {
                            direction: model::ast_arg_direction_to_direction(&a.direction),
                            name: a.name.as_ref().cloned(),
                            arg_type: model::ast_type_to_string(&a.arg_type),
                            doc: a.doc.as_ref().cloned(),
                        })
                        .collect(),
                    value: m.value,
                    doc: m.doc.as_ref().cloned(),
                },
            ),
        })
        .collect();

    model::Item {
        path,
        name: i.name.clone(),
        item_type: model::ItemType::Interface,
        elements,
        doc: i.doc.as_ref().cloned(),
    }
}

fn convert_parcelable(path: String, p: &ast::Parcelable) -> model::Item {
    let elements = p
        .fields
        .iter()
        .map(|f| {
            let element = model::Element::Field {
                name: f.name.clone(),
                field_type: model::ast_type_to_string(&f.field_type),
                doc: f.doc.as_ref().cloned(),
            };
            (f.name.clone(), element)
        })
        .collect();

    model::Item {
        path,
        name: p.name.clone(),
        item_type: model::ItemType::Parcelable,
        elements,
        doc: p.doc.as_ref().cloned(),
    }
}

fn convert_enum(path: String, e: &ast::Enum) -> model::Item {
    let elements = e
        .elements
        .iter()
        .map(|el| {
            let element = model::Element::EnumElement {
                name: el.name.clone(),
                value: el.value.clone(),
                doc: el.doc.as_ref().cloned(),
            };
            (el.name.clone(), element)
        })
        .collect();

    model::Item {
        path,
        name: e.name.clone(),
        item_type: model::ItemType::Enum,
        elements,
        doc: e.doc.as_ref().cloned(),
    }
}
