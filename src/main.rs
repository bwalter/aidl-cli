use std::{
    collections::HashMap,
    io::{Read, Write},
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
        print!(".");
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
    let parse_results = parser.parse();
    println!();

    // Display diagnostics
    report(&files, &parse_results, opt)?;

    // Display element names
    for (id, res) in &parse_results {
        let item = match res.file.as_ref().map(|f| &f.item) {
            Some(i) => i,
            None => continue,
        };

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

        let file_path: PathBuf = files.get(*id)?.name().into();
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

    Ok(())
}

fn report(
    files: &SimpleFiles<String, String>,
    results: &HashMap<usize, ParseFileResult<usize>>,
    _opt: &Opt,
) -> Result<()> {
    // Flat-map aidl-parser diagnostics to codespan diagnostics
    let errors = results.iter().flat_map(|(id, r)| {
        r.diagnostics
            .iter()
            .map(|d| {
                let mut main_label = codespan_reporting::diagnostic::Label::primary(
                    *id,
                    d.range.start.offset..d.range.end.offset,
                );
                if let Some(ref context_msg) = d.context_message {
                    main_label = main_label.with_message(context_msg.clone());
                }
                let mut labels = Vec::from([main_label]);

                for info in d.related_infos.iter() {
                    labels.push(
                        codespan_reporting::diagnostic::Label::secondary(
                            *id,
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
            })
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
