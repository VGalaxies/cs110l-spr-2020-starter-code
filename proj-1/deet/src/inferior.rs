use crate::dwarf_data::{DwarfData, Line};
use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::collections::HashMap;
use std::mem::size_of;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};

pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),
}

pub struct Inferior {
    child: Child,
    breakpoints_mapping: HashMap<usize, u8>,
}

/// This function calls ptrace with PTRACE_TRACEME to enable debugging on a process. You should use
/// pre_exec with Command to call this in the child process.
fn child_traceme() -> Result<(), std::io::Error> {
    ptrace::traceme().or(Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "ptrace TRACEME failed",
    )))
}

fn align_addr_to_word(addr: usize) -> usize {
    addr & (-(size_of::<usize>() as isize) as usize)
}

impl Inferior {
    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(target: &str, args: &Vec<String>, breakpoints: &Vec<usize>) -> Option<Inferior> {
        // TODO: implement me!
        let child;
        unsafe {
            child = Command::new(target)
                .args(args)
                .pre_exec(child_traceme)
                .spawn()
                .ok()?;
        }

        let breakpoints_mapping: HashMap<usize, u8> = Default::default();
        let mut inferior = Inferior {
            child,
            breakpoints_mapping,
        };

        let status = inferior.wait(None).ok()?;
        return match status {
            Status::Stopped(_, _) => {
                for addr in breakpoints {
                    match inferior.write_byte(*addr, 0xcc) {
                        Ok(orig_byte) => {
                            inferior.breakpoints_mapping.insert(*addr, orig_byte);
                        }
                        Err(_) => return None,
                    }
                }
                Some(inferior)
            }
            _ => None,
        };
    }

    fn write_byte(&mut self, addr: usize, val: u8) -> Result<u8, nix::Error> {
        let aligned_addr = align_addr_to_word(addr);
        let byte_offset = addr - aligned_addr;
        let word = ptrace::read(self.pid(), aligned_addr as ptrace::AddressType)? as u64;
        let orig_byte = (word >> 8 * byte_offset) & 0xff;
        let masked_word = word & !(0xff << 8 * byte_offset);
        let updated_word = masked_word | ((val as u64) << 8 * byte_offset);
        ptrace::write(
            self.pid(),
            aligned_addr as ptrace::AddressType,
            updated_word as *mut std::ffi::c_void,
        )?;
        Ok(orig_byte as u8)
    }

    pub fn print_backtrace(&self, debug_data: &DwarfData) -> Result<(), nix::Error> {
        let regs = ptrace::getregs(self.pid())?;
        let mut rip: usize = regs.rip as usize;
        let mut rbp: usize = regs.rbp as usize;
        let mut line: Option<Line>;
        let mut func: Option<String>;
        loop {
            line = debug_data.get_line_from_addr(rip);
            func = debug_data.get_function_from_addr(rip);
            if line.is_some() && func.is_some() {
                let line_unwrap = line.unwrap();
                let func_unwrap = func.unwrap();
                println!(
                    "{} ({}:{})",
                    func_unwrap, line_unwrap.file, line_unwrap.number
                );

                if func_unwrap == "main" {
                    break;
                }

                rip = ptrace::read(self.pid(), (rbp + 8) as ptrace::AddressType)? as usize;
                rbp = ptrace::read(self.pid(), rbp as ptrace::AddressType)? as usize;
            } else {
                println!("??? [rip -> {:#x} | rbp -> {:#x}]", rip, rbp);
                break;
            }
        }
        Ok(())
    }

    pub fn kill(&mut self) -> Result<Status, nix::Error> {
        return match ptrace::kill(self.pid()) {
            Ok(_) => self.wait(None), // reap the killed process
            Err(err) => Err(err),
        };
    }

    pub fn cont(&mut self, breakpoints: &Vec<usize>) -> Result<Status, nix::Error> {
        for addr in breakpoints {
            match self.write_byte(*addr, 0xcc) {
                Ok(orig_byte) => {
                    self.breakpoints_mapping.insert(*addr, orig_byte);
                }
                Err(err) => return Err(err),
            }
        }

        let mut regs = ptrace::getregs(self.pid())?;
        let rip: usize = regs.rip as usize;
        let target_rip = rip - 1;

        let orig_byte_wrap = self.breakpoints_mapping.get(&target_rip);
        if orig_byte_wrap.is_some() {
            let orig_byte = orig_byte_wrap.unwrap().clone();
            match self.write_byte(target_rip, orig_byte) {
                Ok(byte) => {
                    assert_eq!(byte, 0xcc);
                    regs.rip -= 1;
                    ptrace::setregs(self.pid(), regs)?;

                    ptrace::step(self.pid(), None)?;
                    match self.wait(None) {
                        Ok(status) => match status {
                            Status::Stopped(_, _) => match self.write_byte(target_rip, 0xcc) {
                                Ok(byte) => {
                                    assert_eq!(byte, orig_byte);
                                }
                                Err(err) => return Err(err),
                            },
                            _ => return Ok(status),
                        },
                        Err(err) => return Err(err),
                    }
                }
                Err(err) => return Err(err),
            }
        }

        return match ptrace::cont(self.pid(), None) {
            Ok(_) => self.wait(None),
            Err(err) => Err(err),
        };
    }

    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        Pid::from_raw(self.child.id() as i32)
    }

    /// Calls waitpid on this inferior and returns a Status to indicate the state of the process
    /// after the waitpid call.
    pub fn wait(&self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
        Ok(match waitpid(self.pid(), options)? {
            WaitStatus::Exited(_pid, exit_code) => Status::Exited(exit_code),
            WaitStatus::Signaled(_pid, signal, _core_dumped) => Status::Signaled(signal),
            WaitStatus::Stopped(_pid, signal) => {
                let regs = ptrace::getregs(self.pid())?;
                Status::Stopped(signal, regs.rip as usize)
            }
            other => panic!("waitpid returned unexpected status: {:?}", other),
        })
    }
}
