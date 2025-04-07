use clap::Parser;
use colored::Colorize;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Instant;
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};
use tree_sitter::{Language, Node};
use tree_sitter_graph::Variables;
use tree_sitter_stack_graphs::{NoCancellation, StackGraphLanguage};

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    source_folder: Option<PathBuf>,
}

static LANGUAGES: OnceLock<HashMap<String, Language>> = OnceLock::new();

fn main() {
    let args = Args::parse();
    let source_folder = args.source_folder.unwrap_or_else(|| PathBuf::from("."));

    if !Path::exists(&source_folder) {
        panic!("The provided folder {:?} does not exist.", source_folder);
    }

    LANGUAGES.get_or_init(init_languages);

    println!(
        "Scanning for source code in path: {:?}",
        source_folder.canonicalize().unwrap()
    );

    let start_time = Instant::now();

    visit_dirs(&source_folder).expect("Failed processing source files");

    println!(
        "Analysis completed in {} ms",
        start_time.elapsed().as_millis().to_string().bold()
    );
}

fn init_languages() -> HashMap<String, Language> {
    let mut map = HashMap::new();
    map.insert(
        "ts".to_string(),
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
    );
    map.insert(
        "tsx".to_string(),
        tree_sitter_typescript::LANGUAGE_TSX.into(),
    );
    map.insert("rs".to_string(), tree_sitter_rust::LANGUAGE.into());
    map.insert("cs".to_string(), tree_sitter_c_sharp::LANGUAGE.into());
    map
}

fn visit_dirs(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let path = entry?.path();
            visit_dirs(&path)?;
        }
    } else {
        analyze_file(&path)?;
    }

    Ok(())
}

fn analyze_file(path: &Path) -> io::Result<()> {
    let start = Instant::now();
    let mut parser = tree_sitter::Parser::new();
    let file_type = path.extension().and_then(|ext| ext.to_str());

    if let Some(file_type) = file_type {
        if let Some(language) = tree_sitter_language_by_file_type(file_type) {
            parser
                .set_language(&language)
                .expect("Failed to set language");

            analyze_source_file(path.to_path_buf(), language, &mut parser)?;
        }
    }

    Ok(())
}

fn tree_sitter_language_by_file_type(file_type: &str) -> Option<Language> {
    let languages = LANGUAGES.get()?;

    match file_type {
        "rs" => languages.get("rs").cloned(),
        "ts" => languages.get("ts").cloned(),
        "tsx" => languages.get("tsx").cloned(),
        "cs" => languages.get("cs").cloned(),
        _ => None,
    }
}

fn analyze_source_file(
    file_path: PathBuf,
    language: Language,
    parser: &mut tree_sitter::Parser,
) -> io::Result<()> {
    let mut file = fs::File::open(&file_path)?;
    let mut source_code = String::new();
    file.read_to_string(&mut source_code)?;

    let tsg_source = tree_sitter_stack_graphs_typescript::STACK_GRAPHS_TSG_TS_SOURCE;
    let stack_language = StackGraphLanguage::from_str(language, tsg_source).unwrap();
    let mut stack_graph = stack_graphs::graph::StackGraph::new();
    let file_handle = stack_graph.get_or_create_file(file_path.to_str().unwrap());
    let globals = Variables::new();

    stack_language
        .build_stack_graph_into(
            &mut stack_graph,
            file_handle,
            &source_code,
            &globals,
            &NoCancellation,
        )
        .unwrap();

    stack_graph.iter_nodes().for_each(|node| {
        println!("Node: {:?}", node);
    });

    // let tree = parser.parse(&source_code, None).unwrap();
    // let root_node = tree.root_node();

    // eprintln!(
    //     "File: {}",
    //     file_path.to_string_lossy().bright_yellow().bold()
    // );

    // visit_node(&source_code, root_node, 0);
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
