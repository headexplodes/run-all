use std::env;
use std::io::BufReader;
use std::io::prelude::*;
use std::process::{Child, Command, exit};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

use ansi_term::Colour;

const ANSI_COLORS: [u8; 12] = [14, 13, 12, 11, 10, 9, 1, 2, 3, 4, 5, 6];

#[derive(Debug)]
struct Cmd {
    alias: String,
    program: String,
    args: Vec<String>,
}

#[derive(Debug)]
struct Options {
    cmds: Vec<Cmd>
}

enum ParseState {
    Initial,
    PreAlias,
    WithAlias(String),
}

#[derive(Debug)]
struct ParseError {
    message: String
}

struct RunningCmd {
    alias: String,
    process: Child,
    colour: ansi_term::Colour,
}

fn parse_cmd(alias: Option<String>, cmd: &str) -> Result<Cmd, ParseError> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();

    return match parts.split_first() {
        Some((&program, args)) => Ok(Cmd {
            alias: alias.unwrap_or(program.to_owned()),
            program: program.to_owned(),
            args: args.iter().map(|&x| x.to_owned()).collect(),
        }),
        _ => Err(ParseError { message: format!("Could not parse command string: {}", cmd) })
    };
}

fn parse_options<I: Iterator<Item=String>>(args: I) -> Result<Options, ParseError> {
    let mut cmds: Vec<Cmd> = Vec::new();
    let mut state = ParseState::Initial;

    for arg in args {
        if arg == "-a" || arg == "--alias" {
            match state {
                ParseState::Initial =>
                    state = ParseState::PreAlias,
                ParseState::PreAlias =>
                    return Err(ParseError { message: "Alias expected".to_owned() }),
                ParseState::WithAlias(_) =>
                    return Err(ParseError { message: "Command expected".to_owned() })
            }
        } else if arg.starts_with("-") {
            return Err(ParseError { message: format!("Unexpected argument: {}", arg) });
        } else {
            match state {
                ParseState::Initial =>
                    cmds.push(parse_cmd(None, &arg)?),
                ParseState::WithAlias(alias) => {
                    cmds.push(parse_cmd(Some(alias.clone()), &arg)?);
                    state = ParseState::Initial;
                }
                ParseState::PreAlias =>
                    state = ParseState::WithAlias(arg.to_owned())
            }
        }
    }

    return match state {
        ParseState::Initial =>
            Ok(Options { cmds }),
        ParseState::PreAlias =>
            Err(ParseError { message: "Alias expected".to_owned() }),
        ParseState::WithAlias(_) =>
            Err(ParseError { message: "Command expected".to_owned() })
    };
}

//enum Either<L, R> {
//    Left(L),
//    Right(R),
//}
//
//struct OutStream {
//    inner: Either<Stdout, Stderr>
//}
//
//struct OutStreamLock<'a> {
//    inner: Either<StdoutLock<'a>, StderrLock<'a>>
//}
//
//impl OutStream {
//    fn stdout() -> OutStream {
//        return OutStream { inner: Either::Left(std::io::stdout()) };
//    }
//
//    fn stderr() -> OutStream {
//        return OutStream { inner: Either::Right(std::io::stderr()) };
//    }
//
//    fn lock(&self) -> OutStreamLock {
//        match &self.inner {
//            Either::Left(stdout) =>
//                OutStreamLock { inner: Either::Left(stdout.lock()) },
//            Either::Right(stderr) =>
//                OutStreamLock { inner: Either::Right(stderr.lock()) },
//        }
//    }
//}
//
//impl<'a> Write for OutStreamLock<'a> {
//    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
//        match &mut self.inner {
//            Either::Left(stdout) =>
//                stdout.write(buf),
//            Either::Right(stderr) =>
//                stderr.write(buf)
//        }
//    }
//
//    fn flush(&mut self) -> Result<(), Error> {
//        match &mut self.inner {
//            Either::Left(stdout) =>
//                stdout.flush(),
//            Either::Right(stderr) =>
//                stderr.flush()
//        }
//    }
//}

fn pad_left(src: &str, min: usize) -> String {
    let mut buf = String::with_capacity(std::cmp::max(src.len(), min));
    buf.insert_str(0, src);
    for _ in src.len()..min {
        buf.push(' ');
    }
    return buf;
}

//fn stream_thread<S: Read>(alias: &str, max_alias: usize, colour: Colour, src: S, dest: OutStream) -> () {
//    let reader = BufReader::new(src);
//    for line in reader.lines() {
//        let mut lock = dest.lock();
//        let alias = pad_left(&format!("[{}]", alias), max_alias + 2); // include brackets
//        writeln!(lock, "{}", colour.paint(format!("{} {}", alias, line.unwrap()))).unwrap();
//    }
//}

fn stream_thread<S: Read, W: Write>(prefix: &str, colour: Colour, src: S, mut dest: W, mutex: Arc<Mutex<()>>) -> () {
    let reader = BufReader::new(src);
    for line in reader.lines() {
        let _lock = mutex.lock().unwrap(); // one thread writing at a time
        writeln!(dest, "{}", colour.paint(format!("{} {}", prefix, line.unwrap()))).unwrap();
    }
}

fn main() {
    let mut args = env::args();

    let program = args.next();

    let options = match parse_options(args) {
        Ok(options) => options,
        Err(err) => {
            eprintln!("Error: {}", err.message);
            exit(1);
        }
    };

    if options.cmds.is_empty() {
        eprintln!("Error: Expected at least one command to run");
        eprintln!("Usage: {} [{{-a|--alias}} <alias>] <command> ...", program.unwrap_or("run-all".to_owned()));
        exit(1);
    }

    let max_alias = options.cmds.iter()
        .map(|x| x.alias.len())
        .max()
        .unwrap(); // cannot panic (non-empty list asserted above)

    let children: Vec<RunningCmd> = options.cmds
        .into_iter()
        .enumerate()
        .map(|(index, cmd)| {
            let child = Command::new(&cmd.program)
                .args(&cmd.args)
                .stdin(Stdio::null())
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn();

            match child {
                Ok(child) =>
                    RunningCmd {
                        alias: cmd.alias.clone(),
                        process: child,
                        colour: Colour::Fixed(ANSI_COLORS[index % ANSI_COLORS.len()]),
                    },
                Err(err) => {
                    panic!("Error: Could not start program '{}' ({:?})", cmd.program, err); // TODO: handle this more gracefully
                }
            }
        })
        .collect();

    // locking stdout/stderr may not be enough as some terminals seem to interleave the two streams (ie, colour escape sequences can break)
    let output_mutex = Arc::new(Mutex::new(()));

    let threads: Vec<(JoinHandle<()>, JoinHandle<()>)> = children
        .into_iter()
        .map(|mut child| {
            // TODO: don't panic (just don't capture, maybe with warning)
            let child_stdout = child.process.stdout.take().expect("Expected child stdout");
            let child_stderr = child.process.stderr.take().expect("Expected child stderr");

            let prefix = pad_left(&format!("[{}]", child.alias), max_alias + 2); // include brackets

            return (
                thread::spawn({
                    let prefix = prefix.clone();
                    let colour = child.colour.clone();
                    let output_mutex = output_mutex.clone();
                    move || {
//                        stream_thread(&alias, max_alias, colour, child_stdout, OutStream::stdout())
                        stream_thread(&prefix, colour, child_stdout, std::io::stdout(), output_mutex)
                    }
                }),
                thread::spawn({
                    let prefix = prefix.clone();
                    let colour = child.colour.clone();
                    let output_mutex = output_mutex.clone();
                    move || {
//                        stream_thread(&alias, max_alias, colour, child_stderr, OutStream::stderr())
                        stream_thread(&prefix, colour, child_stderr, std::io::stderr(), output_mutex)
                    }
                })
            );

            // TODO: maybe return a struct that contains both threads... (remember to wait for thread _and_ process exit)
        })
        .collect();

    for result in threads {
        result.0.join().expect("Error waiting for thread");
        result.1.join().expect("Error waiting for thread");
    }

    // TODO: log child exit status
}

// TODO: need to detect when not run within an interactive terminal and disable colours
// TODO: allow colours to be disabled via command line/environment variable

// TODO: signal handling (send SIGINT to children...), see https://rust-lang-nursery.github.io/cli-wg/in-depth/signals.html
