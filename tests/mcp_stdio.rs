use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn stdio_server_lists_tools_without_writing_logs_to_stdout()
-> Result<(), Box<dyn std::error::Error>> {
    let config_dir = tempdir()?;
    let mut child = Command::new(env!("CARGO_BIN_EXE_igara"))
        .args(["mcp", "serve"])
        .env("IGARA_CONFIG_DIR", config_dir.path())
        .env("RUST_LOG", "debug")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientInfo": {
                    "name": "igara-stdio-test",
                    "version": "0.1.0"
                },
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        }
    });

    child
        .stdin
        .as_mut()
        .ok_or("MCP server stdin was not piped")?
        .write_all(format!("{request}\n").as_bytes())?;

    let stdout = child
        .stdout
        .take()
        .ok_or("MCP server stdout was not piped")?;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
        let _ = sender.send(result);
    });

    let response = receiver.recv_timeout(Duration::from_secs(10));
    let _ = child.kill();
    let _ = child.wait();
    let response = response??;
    let response: Value = serde_json::from_str(&response)?;

    let tools = response["result"]["tools"]
        .as_array()
        .ok_or("tools/list response did not contain a tools array")?;
    let names = tools
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();

    assert!(names.contains(&"list_hosts"));
    assert!(names.contains(&"remote_build"));
    assert_eq!(tools.len(), 2);

    let remote_build = tools
        .iter()
        .find(|tool| tool["name"] == "remote_build")
        .ok_or("remote_build tool was not advertised")?;
    let required = remote_build["inputSchema"]["required"]
        .as_array()
        .ok_or("remote_build schema did not list required fields")?;
    assert!(required.iter().any(|field| field == "host"));
    assert!(required.iter().any(|field| field == "command"));
    assert!(
        remote_build["description"]
            .as_str()
            .unwrap_or_default()
            .contains("remote code execution")
    );

    Ok(())
}
