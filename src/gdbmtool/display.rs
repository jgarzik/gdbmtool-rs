use std::io::{IsTerminal, Write};
use std::process::Stdio;

pub fn display(lines: Vec<String>) {
    // page output that does not fit on the terminal
    let terminal_height = std::io::stdout()
        .is_terminal()
        .then(termsize::get)
        .flatten()
        .map(|size| size.rows as usize);

    match terminal_height {
        Some(height) if lines.len() > height => pager(lines),
        _ => write_lines(&mut std::io::stdout(), lines),
    }
}

fn write_lines(out: &mut impl Write, lines: Vec<String>) {
    lines
        .into_iter()
        .for_each(|l| writeln!(out, "{l}").unwrap());
}

fn pager(lines: Vec<String>) {
    let pager = match std::env::var("PAGER") {
        Ok(path) => path,
        _ => "pager".to_string(),
    };

    match std::process::Command::new(pager)
        .stdin(Stdio::piped())
        .spawn()
    {
        Ok(mut child) => {
            write_lines(&mut child.stdin.take().unwrap(), lines);
            child.wait().ok();
        }
        _ => write_lines(&mut std::io::stdout(), lines),
    }
}
