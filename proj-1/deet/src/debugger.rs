use crate::debugger_command::DebuggerCommand;
use crate::dwarf_data::{DwarfData, Error as DwarfError};
use crate::inferior::{Inferior, Status};
use rustyline::error::ReadlineError;
use rustyline::Editor;

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<()>,
    inferior: Option<Inferior>,
    debug_data: DwarfData,
    breakpoints: Vec<usize>,
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        // TODO (milestone 3): initialize the DwarfData
        let debug_data = match DwarfData::from_file(target) {
            Ok(val) => val,
            Err(DwarfError::ErrorOpeningFile) => {
                println!("Could not open file {}", target);
                std::process::exit(1);
            }
            Err(DwarfError::DwarfFormatError(err)) => {
                println!("Could not debugging symbols from {}: {:?}", target, err);
                std::process::exit(1);
            }
        };

        debug_data.print();

        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        let mut readline = Editor::<()>::new();
        // Attempt to load history from ~/.deet_history if it exists
        let _ = readline.load_history(&history_path);

        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
            debug_data,
            breakpoints: vec![],
        }
    }

    fn handle_status(&mut self, status: Status) {
        match status {
            Status::Stopped(signal, rip) => {
                let line = self.debug_data.get_line_from_addr(rip);
                let func = self.debug_data.get_function_from_addr(rip);

                if line.is_some() && func.is_some() {
                    let line_unwrap = line.unwrap();
                    let func_unwrap = func.unwrap();
                    println!(
                        "Child stopped by {} at {} ({}:{})",
                        signal, func_unwrap, line_unwrap.file, line_unwrap.number
                    );
                } else {
                    println!("Child stopped by {} at {:#x}", signal, rip);
                }
            }
            Status::Exited(exit_code) => {
                self.inferior = None;
                println!("Child exited (status {})", exit_code)
            }
            Status::Signaled(signal) => {
                self.inferior = None;
                println!("Child exited by {}", signal)
            }
        }
    }

    fn kill_inferior(&mut self) {
        let inferior = self.inferior.as_mut().unwrap();
        match inferior.kill() {
            Some(status) => self.handle_status(status),
            None => {
                println!("Error killing subprocess");
            }
        }
    }

    fn cont_inferior(&mut self) {
        let inferior = self.inferior.as_mut().unwrap();
        match inferior.cont(&self.breakpoints) {
            Ok(status) => self.handle_status(status),
            Err(err) => {
                println!("{}", err);
            }
        }
    }

    fn create_new_inferior(&mut self, args: &Vec<String>) {
        if let Some(inferior) = Inferior::new(&self.target, &args, &self.breakpoints) {
            // Create the inferior
            self.inferior = Some(inferior);
            // You may use self.inferior.as_mut().unwrap() to get a mutable reference
            // to the Inferior object
            self.cont_inferior();
        } else {
            println!("Error starting subprocess");
        }
    }

    fn parse_address(addr: &str) -> Option<usize> {
        let addr_without_0x = if addr.to_lowercase().starts_with("0x") {
            &addr[2..]
        } else {
            &addr
        };
        usize::from_str_radix(addr_without_0x, 16).ok()
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
                DebuggerCommand::Backtrace => match &self.inferior {
                    Some(inferior) => match inferior.print_backtrace(&self.debug_data) {
                        Ok(_) => {}
                        Err(err) => println!("{}", err),
                    },
                    None => {
                        println!("The program is not being run");
                    }
                },
                DebuggerCommand::Break(breakpoint) => {
                    if !breakpoint.starts_with("*") {
                        let line_wrap = usize::from_str_radix(&breakpoint, 10);
                        if line_wrap.is_ok() {
                            let line = line_wrap.unwrap();
                            match self.debug_data.get_addr_for_line(None, line) {
                                Some(addr) => {
                                    let index = self.breakpoints.len();
                                    self.breakpoints.push(addr);
                                    println!("Set breakpoint {} at {:#x} (line {})", index, addr, line);
                                }
                                None => {
                                    println!("Invalid line breakpoint");
                                }
                            }
                        } else {
                            match self.debug_data.get_addr_for_function(None, &breakpoint) {
                                Some(addr) => {
                                    let index = self.breakpoints.len();
                                    self.breakpoints.push(addr);
                                    println!("Set breakpoint {} at {:#x} (function {})", index, addr, &breakpoint);
                                }
                                None => {
                                    println!("Invalid function breakpoint");
                                }
                            }
                        }
                    } else {
                        match Debugger::parse_address(&breakpoint[1..]) {
                            Some(addr) => {
                                let index = self.breakpoints.len();
                                self.breakpoints.push(addr);
                                println!("Set breakpoint {} at {:#x}", index, addr);
                            }
                            None => println!("Invalid address breakpoint"),
                        }
                    }
                }
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
