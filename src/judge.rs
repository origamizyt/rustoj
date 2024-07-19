use std::fs::File;
use std::os::fd::AsRawFd;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use std::time::Instant;

use crate::models::*;

pub fn standard_judge(got: &str, expected: &str) -> Status {
    let got = 
        got.trim_end()
        .split("\n")
        .collect::<Vec<&str>>();
    let expected = 
        expected.trim_end()
        .split("\n")
        .collect::<Vec<&str>>();
    if got.len() != expected.len() { return Status::WrongAnswer; }
    for (mut got_line, mut expected_line) in got.into_iter().zip(expected.into_iter()) {
        got_line = got_line.trim_end();
        expected_line = expected_line.trim_end();
        if got_line != expected_line {
            return Status::WrongAnswer;
        }
    }
    Status::Accepted
}

pub fn strict_judge(got: &str, expected: &str) -> Status {
    if got == expected { Status::Accepted } else { Status::WrongAnswer }
}

impl Language {
    pub fn compile(&self, src: &str, dst: &str) -> bool {
        let command = self.expand_command(src, dst);
        let status = 
            Command::new(&command[0])
            .args(&command[1..])
            .status();
        return status.is_ok_and(|code| code.success());
    }
}

const BANNED_SYSCALLS: &[u64] = &[435];

#[derive(Clone, Copy)]
struct Timer {
    start_time: Instant,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }
    pub fn elapsed(&self) -> u64 {
        let duration = Instant::now() - self.start_time;
        duration.as_micros() as _
    }
}

pub struct Resources {
    pub time: u64,
    pub memory: u64,
}

impl Case {
    pub fn run(&self, exe_file: &str, in_file: &str, out_file: &str) -> (std::result::Result<(), Status>, Resources) {
        unsafe {
            let pid = libc::fork();
            if pid > 0 { // parent
                let timer = Timer::new();
                let timeout = Arc::new(Mutex::new(false));
                let timeout_clone = Arc::clone(&timeout);
                let timer_stopped = Arc::new(Mutex::new(false));
                let timer_stopped_clone = Arc::clone(&timer_stopped);
                let mut join_handle = None;
                if self.time_limit > 0 {
                    let time_limit = self.time_limit;
                    join_handle = Some(spawn(move || {
                        while !*timer_stopped.lock().unwrap() && timer.elapsed() <= time_limit {}
                        if !*timer_stopped.lock().unwrap() {
                            {
                                *timeout.lock().unwrap() = true;
                            }
                            libc::kill(pid, libc::SIGTERM);
                            //println!("Killed");
                        }
                    }));
                }
                let mut status = 0;
                let case_status: Status;
                let mut memory_used : u64;
                let mut regs = libc::user_regs_struct { 
                    r15: 0, r14: 0, r13: 0, r12: 0, rbp: 0, rbx: 0, r11: 0, r10: 0, r9: 0, r8: 0, 
                    rax: 0, rcx: 0, rdx: 0, rsi: 0, rdi: 0, orig_rax: 0, rip: 0, cs: 0, eflags: 0, 
                    rsp: 0, ss: 0, fs_base: 0, gs_base: 0, ds: 0, es: 0, fs: 0, gs: 0 
                };
                let mut ru = libc::rusage { 
                    ru_utime: libc::timeval { tv_sec: 0, tv_usec: 0 }, 
                    ru_stime: libc::timeval { tv_sec: 0, tv_usec: 0 }, 
                    ru_maxrss: 0, ru_ixrss: 0, ru_idrss: 0, ru_isrss: 0, ru_minflt: 0, 
                    ru_majflt: 0, ru_nswap: 0, ru_inblock: 0, ru_oublock: 0, ru_msgsnd: 0, 
                    ru_msgrcv: 0, ru_nsignals: 0, ru_nvcsw: 0, ru_nivcsw: 0 
                };
                libc::wait(&mut status);
                loop {
                    libc::ptrace(libc::PTRACE_SYSCALL, pid, 0, 0);
                    libc::wait4(pid, &mut status, libc::WUNTRACED, &mut ru);
                    memory_used = (ru.ru_maxrss * 1000) as _;

                    if libc::WIFEXITED(status) {
                        if libc::WEXITSTATUS(status) == 0 {
                            case_status = Status::Accepted;
                        }
                        else {
                            case_status = Status::RuntimeError;
                        }
                        break;
                    }
                    if libc::WIFSIGNALED(status) {
                        libc::ptrace(libc::PTRACE_KILL, pid, 0, 0);
                        if libc::WTERMSIG(status) == libc::SIGXCPU {
                            case_status = Status::TimeLimitExceeded;
                        }
                        else if 
                            libc::WTERMSIG(status) == libc::SIGSEGV && 
                            self.memory_limit > 0 && memory_used > self.memory_limit 
                        {
                            case_status = Status::MemoryLimitExceeded;
                        }
                        else {
                            case_status = Status::RuntimeError;
                        }
                        break;
                    }

                    if 
                        libc::WIFSTOPPED(status) && 
                        libc::WSTOPSIG(status) != libc::SIGTRAP && 
                        libc::WSTOPSIG(status) != libc::SIGCHLD 
                    {
                        libc::ptrace(libc::PTRACE_KILL, pid, 0, 0);
                        if libc::WSTOPSIG(status) == libc::SIGXCPU {
                            case_status = Status::TimeLimitExceeded;
                        }
                        else if 
                            libc::WSTOPSIG(status) == libc::SIGSEGV && 
                            self.memory_limit > 0 && memory_used > self.memory_limit 
                        {
                            case_status = Status::MemoryLimitExceeded;
                        }
                        else {
                            case_status = Status::RuntimeError;
                        }
                        break;
                    }

                    if self.memory_limit > 0 && memory_used > self.memory_limit {
                        libc::ptrace(libc::PTRACE_KILL, pid, 0, 0);
                        case_status = Status::MemoryLimitExceeded;
                        break;
                    }

                    libc::ptrace(libc::PTRACE_GETREGS, pid, 0, &mut regs);
                    if BANNED_SYSCALLS.contains(&regs.orig_rax) {
                        libc::ptrace(libc::PTRACE_KILL, pid, 0, 0);
                        libc::waitpid(pid, &mut status, libc::WUNTRACED);
                        case_status = Status::RuntimeError;
                        break;
                    }
                }
                *timer_stopped_clone.lock().unwrap() = true;
                if let Some(handle) = join_handle {
                    handle.join().unwrap();
                }
                if *timeout_clone.lock().unwrap() || (self.time_limit > 0 && timer.elapsed() > self.time_limit) {
                    (Err(Status::TimeLimitExceeded), Resources {
                        time: timer.elapsed(),
                        memory: memory_used
                    })
                }
                else if case_status == Status::Accepted {
                    (Ok(()), Resources {
                        time: timer.elapsed(),
                        memory: memory_used
                    })
                }
                else {
                    (Err(case_status), Resources {
                        time: timer.elapsed(),
                        memory: memory_used
                    })
                }
            }
            else {
                let input_fp = File::open(in_file).unwrap();
                let output_fp = File::create(out_file).unwrap();
                libc::dup2(input_fp.as_raw_fd(), libc::STDIN_FILENO);
                libc::dup2(output_fp.as_raw_fd(), libc::STDOUT_FILENO);
                if self.memory_limit > 0 {
                    let memory_limit = libc::rlimit {
                        rlim_cur: self.memory_limit,
                        rlim_max: self.memory_limit,
                    };
                    libc::setrlimit(libc::RLIMIT_DATA, &memory_limit);
                }
                libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0);
                libc::execl(exe_file.as_ptr() as _, exe_file.as_ptr() as _, 0);
                unreachable!();
            }
        }
    }
}