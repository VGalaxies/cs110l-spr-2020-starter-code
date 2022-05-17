use crate::debugger_command::DebuggerCommand;
use crate::inferior::{Inferior, Status};
use rustyline::error::ReadlineError;
use rustyline::Editor;

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<()>,
    inferior: Option<Inferior>,
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        // TODO (milestone 3): initialize the DwarfData

        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        let mut readline = Editor::<()>::new();
        // Attempt to load history from ~/.deet_history if it exists
        let _ = readline.load_history(&history_path);

        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
        }
    }

    fn kill_inferior(&mut self) {
        let inferior = self.inferior.as_mut().unwrap();
        match inferior.kill() {
            Some(status) => match status {
                Status::Stopped(signal, rip) => {
                    println!("Child stopped by {} at 0x{:016x}", signal, rip)
                }
                Status::Exited(exit_code) => {
                    self.inferior = None;
                    println!("Child exited (status {})", exit_code)
                }
                Status::Signaled(signal) => {
                    self.inferior = None;
                    println!("Child exited by {}", signal)
                }
            },
            None => {
                println!("Error killing subprocess");
            }
        }
    }

    fn cont_inferior(&mut self) {
        let inferior = self.inferior.as_ref().unwrap();
        match inferior.cont() {
            Ok(status) => match status {
                Status::Stopped(signal, rip) => {
                    println!("Child stopped by {} at 0x{:016x}", signal, rip)
                }
                Status::Exited(exit_code) => {
                    self.inferior = None;
                    println!("Child exited (status {})", exit_code)
                }
                Status::Signaled(signal) => {
                    self.inferior = None;
                    println!("Child exited by {}", signal)
                }
            },
            Err(err) => {
                println!("{}", err);
            }
        }
    }

    fn create_new_inferior(&mut self, args: &Vec<String>) {
        if let Some(inferior) = Inferior::new(&self.target, &args) {
            // Create the inferior
            self.inferior = Some(inferior);
            // You may use self.inferior.as_mut().unwrap() to get a mutable reference
            // to the Inferior object
            self.cont_inferior();
        } else {
            println!("Error starting subprocess");
        }
    }

    // TODO (milestone 1): make the inferior run
    pub fn run(&mut self) {
        loop {
            match self.get_next_command() {
                DebuggerCommand::Run(args) => match &mut self.inferior {
                    Some(inferior) => {
                        println!(
                            "Killing the running inferior (pid {}) before running new inferior",
                            inferior.pid()
                        );
                        self.kill_inferior();
                        self.create_new_inferior(&args);
                    }
                    None => {
                        self.create_new_inferior(&args);
                    }
                },
                DebuggerCommand::Continue => match &mut self.inferior {
                    Some(_) => self.cont_inferior(),
                    None => {
                        println!("The program is not being run");
                    }
                },
                DebuggerCommand::Quit => match &mut self.inferior {
                    Some(inferior) => {
                        println!(
                            "Killing the running inferior (pid {}) before quitting",
                            inferior.pid()
                        );
                        self.kill_inferior();
                        return;
                    }
                    None => {
                        return;
                    }
                },
            }
        }
    }

    /// This function prompts the user to enter a command, and continues re-prompting until the user
    /// enters a valid command. It uses DebuggerCommand::from_tokens to do the command parsing.
    ///
    /// You don't need to read, understand, or modify this function.
    fn get_next_command(&mut self) -> DebuggerCommand {
        loop {
            // Print prompt and get next line of user input
            match self.readline.readline("(deet) ") {
                Err(ReadlineError::Interrupted) => {
                    // User pressed ctrl+c. We're going to ignore it
                    println!("Type \"quit\" to exit");
                }
                Err(ReadlineError::Eof) => {
                    // User pressed ctrl+d, which is the equivalent of "quit" for our purposes
                    return DebuggerCommand::Quit;
                }
                Err(err) => {
                    panic!("Unexpected I/O error: {:?}", err);
                }
                Ok(line) => {
                    if line.trim().len() == 0 {
                        continue;
                    }
                    self.readline.add_history_entry(line.as_str());
                    if let Err(err) = self.readline.save_history(&self.history_path) {
                        println!(
                            "Warning: failed to save history file at {}: {}",
                            self.history_path, err
                        );
                    }
                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    if let Some(cmd) = DebuggerCommand::from_tokens(&tokens) {
                        return cmd;
                    } else {
                        println!("Unrecognized command.");
                    }
                }
            }
        }
    }
}
