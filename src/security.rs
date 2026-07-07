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

// Interrupt ssh on cancel so it doesn't keep re-prompting.
pub fn sigint_parent() {
    unsafe {
        let fg = tty_foreground_pgrp();
        let via_parent = {
            let p = libc::getpgid(libc::getppid());
            (p > 0).then_some(p)
        };

        if std::env::var_os("SSH_ASKPASS_DEBUG").is_some() {
            eprintln!(
                "ssh-askpass-rs: cancel: pid={} ppid={} tty_fg_pgrp={fg:?} getpgid(ppid)={via_parent:?}",
                libc::getpid(),
                libc::getppid(),
            );
        }

        if let Some(pgid) = fg.or(via_parent) {
            libc::kill(-pgid, libc::SIGINT);
        }
    }
}

// Foreground process group of the controlling terminal, if we have one.
unsafe fn tty_foreground_pgrp() -> Option<i32> {
    let fd = libc::open(c"/dev/tty".as_ptr(), libc::O_RDONLY);
    if fd < 0 {
        return None;
    }
    let pg = libc::tcgetpgrp(fd);
    libc::close(fd);
    (pg > 0).then_some(pg)
}
