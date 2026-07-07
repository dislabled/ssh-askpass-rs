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

// Terminale ssh on cancel so it doesn't keep re-prompting.
pub fn terminate_ssh() {
    unsafe {
        let ppid = libc::getppid();

        if std::env::var_os("SSH_ASKPASS_DEBUG").is_some() {
            eprintln!(
                "ssh-askpass-rs: cancel: pid={} ppid={ppid}",
                libc::getpid(),
            );
        }

        // ppid <= 1 means we've been orphaned; don't signal init.
        if ppid > 1 {
            libc::kill(ppid, libc::SIGTERM);
        }
    }
}
