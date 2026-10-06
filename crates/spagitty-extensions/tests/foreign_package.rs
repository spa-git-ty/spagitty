// SPDX-License-Identifier: GPL-3.0-or-later

//! A package built by the TypeScript kit, installed and run by the host.
//!
//! Ignored by default because the package is a build product: make it with
//!
//! ```sh
//! bun run ext pack examples/extensions/hello --build
//! SPAGITTY_TEST_PACKAGE=examples/extensions/hello/dist/com.example.hello-0.1.0.spagitty-extension \
//!   cargo test -p spagitty-extensions --test foreign_package -- --ignored
//! ```
//!
//! It is the developer path's last step done by a different implementation
//! than the one that packed it: nothing in the package knows about the host's
//! Rust types, and nothing in the host knows about the example.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;
use spagitty_core::fixture::Fixture;
use spagitty_extensions::host::{Config, Events, ExtensionHost, HostEvent, Invocation, Services};
use spagitty_extensions::manifest::Context;
use spagitty_extensions::protocol::RpcError;
use spagitty_extensions::registry::Paths;

#[derive(Default)]
struct Collected(Mutex<Vec<HostEvent>>);

impl Events for Collected {
    fn emit(&self, event: HostEvent) {
        self.0.lock().unwrap().push(event);
    }
}

struct NoServices;

impl Services for NoServices {
    fn pull_request_snapshot(&self, _: &std::path::Path, _: u64) -> Result<Value, RpcError> {
        Err(RpcError::new(-32010, "none"))
    }
    fn post_pull_request_comment(
        &self,
        _: &std::path::Path,
        _: u64,
        _: &str,
        _: &str,
        _cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<Value, RpcError> {
        Err(RpcError::new(-32011, "none"))
    }
}

#[test]
#[ignore = "needs a package built by `bun run ext pack`"]
fn a_package_the_typescript_kit_built_installs_and_runs() {
    let package =
        PathBuf::from(std::env::var("SPAGITTY_TEST_PACKAGE").expect("SPAGITTY_TEST_PACKAGE"));
    let package = if package.is_relative() {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(package)
    } else {
        package
    };
    let data = tempfile::tempdir().unwrap();
    let events = Arc::new(Collected::default());
    let host = ExtensionHost::new(
        Config::new(
            "0.9.0",
            Paths {
                data: data.path().join("extensions"),
                bundled: None,
                exe_dir: None,
            },
        ),
        events.clone(),
        Arc::new(NoServices),
    );

    let preview = host.inspect_package(&package, None).unwrap();
    assert_eq!(preview.id, "com.example.hello");
    assert!(
        preview.compatibility.compatible,
        "{:?}",
        preview.compatibility
    );
    host.install(&preview.token).unwrap();

    let repo = Fixture::woven();
    host.enable("com.example.hello", repo.path(), &[], None)
        .unwrap();
    let started = host
        .run_command(
            "com.example.hello",
            "hello",
            &Invocation {
                kind: Context::WorkingCopy,
                workdir: Some(repo.path().to_path_buf()),
                task_id: None,
                pull_request: None,
            },
        )
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(30);
    let finished = loop {
        let found = events.0.lock().unwrap().iter().find_map(|e| match e {
            HostEvent::OperationFinished {
                operation,
                status,
                message,
                ..
            } if *operation == started.operation => Some((status.clone(), message.clone())),
            _ => None,
        });
        if let Some(found) = found {
            break found;
        }
        assert!(
            Instant::now() < deadline,
            "the command did not finish: {:?}",
            events.0.lock().unwrap()
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(finished.0, "completed");
    assert!(finished.1.starts_with("Hello, "), "{}", finished.1);
    assert!(events
        .0
        .lock()
        .unwrap()
        .iter()
        .any(|e| matches!(e, HostEvent::Notice { message, .. } if message == "Hello from main")));

    let panel = host
        .resolve_panel(
            "com.example.hello",
            "about",
            &Invocation {
                kind: Context::WorkingCopy,
                workdir: Some(repo.path().to_path_buf()),
                task_id: None,
                pull_request: None,
            },
        )
        .unwrap();
    assert_eq!(panel["rows"][1]["value"], "main");

    host.disable("com.example.hello", repo.path(), false)
        .unwrap();
    host.uninstall("com.example.hello", false, Some(repo.path()))
        .unwrap();
    assert!(
        host.list(Some(repo.path())).extensions.is_empty(),
        "nothing is left behind"
    );
    host.shutdown();
}
