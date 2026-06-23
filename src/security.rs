// disable coredumps because it could contain passwords
pub fn disable_core_dumps() {
    unsafe {
        let rlim = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        libc::setrlimit(libc::RLIMIT_CORE, &rlim);
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
