use clap::Parser;
use colored::Colorize;
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};
use tree_sitter::{Language, Node};

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    source_folder: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    let source_folder = args.source_folder.unwrap_or_else(|| PathBuf::from("."));

    if !Path::exists(&source_folder) {
        panic!("Ensure that the source folder exists.");
    }

    println!(
        "Scanning for source code in path: {:?}",
        source_folder.canonicalize().unwrap()
    );

    visit_dirs(&source_folder).expect("Failed processing source files");
}

fn visit_dirs(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let path = entry?.path();

            if path.is_dir() {
                visit_dirs(&path)?;
            } else {
                analyze_file(&path)?;
            }

            // if path.is_dir() {
            //     visit_dirs(&path)?;
            // } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            //     analyze_source_file(path)?;
            // }
        }
    } else {
        analyze_file(&path)?;
    }

    Ok(())
}

fn analyze_file(path: &Path) -> io::Result<()> {
    let mut parser = tree_sitter::Parser::new();
    let file_type = path.extension().and_then(|ext| ext.to_str());

    if let Some(file_type) = file_type {
        if let Some(language) = tree_sitter_language_by_file_type(file_type) {
            parser
                .set_language(&language)
                .expect("Failed to set language");

            analyze_source_file(path.to_path_buf(), &mut parser)?;
        }
    }

    Ok(())
}

fn tree_sitter_language_by_file_type(file_type: &str) -> Option<Language> {
    match file_type {
        "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
        "ts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        "cs" => Some(tree_sitter_c_sharp::LANGUAGE.into()),
        _ => None,
    }
}

fn analyze_source_file(file_path: PathBuf, parser: &mut tree_sitter::Parser) -> io::Result<()> {
    let mut file = fs::File::open(&file_path)?;
    let mut source_code = String::new();
    file.read_to_string(&mut source_code)?;

    let tree = parser.parse(&source_code, None).unwrap();
    let root_node = tree.root_node();

    eprintln!("File: {:?}", file_path.to_string_lossy().bold());

    visit_node(&source_code, root_node, 0);
    Ok(())
}

fn visit_node(source: &str, node: Node<'_>, depth: usize) {
    if !node.is_named() {
        return;
    }

    print!("{}{}", "  ".repeat(depth), node.kind().to_string().yellow());

    match node.kind() {
        "identifier" | "type_identifier" => {
            let start_byte = node.start_byte();
            let end_byte = node.end_byte();
            let node_text = &source[start_byte..end_byte];
            println!(" -> {}", node_text.bright_cyan());
        }
        _ => println!(""),
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_node(source, child, depth + 1);
    }
}
