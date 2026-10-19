// SPDX-License-Identifier: GPL-3.0-or-later

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `spagitty --history-mcp <repository>`: the history tools the farm's
    // agents ask (FEAT-145), served on stdio, with no window at all.
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() == Some(std::ffi::OsStr::new(spagitty_farm::history::FLAG)) {
        let repo = args
            .next()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let _ =
            spagitty_farm::history::serve(&repo, std::io::stdin().lock(), std::io::stdout().lock());
        return;
    }
    spagitty_lib::run()
}
