pub fn disable_core_dumps() {
    unsafe {
        let rlim = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if libc::setrlimit(libc::RLIMIT_CORE, &rlim) != 0 {
            eprintln!("ssh-askpass-rs: warning: failed to disable core dumps");
        }
    }
}

// killing ssh when cancel to avoid recurring password prompts
pub fn sigint_parent() {
    unsafe {
        let ppid = libc::getppid();
        let pgid = libc::getpgid(ppid);
        libc::kill(-pgid, libc::SIGINT);
    }
}
