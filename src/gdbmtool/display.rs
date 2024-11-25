use std::io::Write;
use std::process::Stdio;

pub fn display(lines: Vec<String>) {
    let terminal_height = termsize::get().expect("we are in a terminal").rows;

    if lines.len() > terminal_height as usize {
        pager(lines);
    } else {
        write_lines(&mut std::io::stdout(), lines);
    };
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
