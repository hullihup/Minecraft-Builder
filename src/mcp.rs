//! MCP server (Model Context Protocol, streamable HTTP) so an AI assistant can
//! work on the open project: read it, add and connect blocks, build.
//!
//! The server only listens on this computer (127.0.0.1). The address contains a
//! secret token: `http://127.0.0.1:<port>/mcp/<token>`. Requests are handed to
//! the UI thread, which runs them against the app state.

use crate::app::App;
use crate::blocks::{self, Kind, PinType};
use crate::model::{Node, NodeId, PinRef};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};

/// A tool call waiting for the UI thread.
pub struct Call {
    pub tool: String,
    pub args: Value,
    pub reply: Sender<Result<String, String>>,
}

pub struct Server {
    pub port: u16,
    pub rx: Receiver<Call>,
    stop: Arc<AtomicBool>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Random-looking token for the address.
pub fn new_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::new();
    for i in 0..2u64 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(i);
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

pub fn url(port: u16, token: &str) -> String {
    format!("http://127.0.0.1:{port}/mcp/{token}")
}

/// Starts listening. Fails if the port is in use.
pub fn start(port: u16, token: String, ctx: egui::Context) -> Result<Server, String> {
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let (tx, rx) = channel();
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    std::thread::spawn(move || {
        while !stop2.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((sock, _)) => {
                    let (tx, token, ctx) = (tx.clone(), token.clone(), ctx.clone());
                    std::thread::spawn(move || {
                        let _ = sock.set_nonblocking(false);
                        let _ = sock.set_read_timeout(Some(std::time::Duration::from_secs(15)));
                        let _ = serve(sock, &token, &tx, &ctx);
                    });
                }
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(40)),
            }
        }
    });
    Ok(Server { port, rx, stop })
}

fn respond(sock: &mut TcpStream, status: &str, body: &str) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    sock.write_all(head.as_bytes())?;
    sock.write_all(body.as_bytes())
}

fn serve(mut sock: TcpStream, token: &str, tx: &Sender<Call>, ctx: &egui::Context) -> std::io::Result<()> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];
    let (head_end, len) = loop {
        let n = sock.read(&mut tmp)?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buf[..i]).to_lowercase();
            let len = head
                .lines()
                .find_map(|l| l.strip_prefix("content-length:"))
                .and_then(|v| v.trim().parse::<usize>().ok())
                .unwrap_or(0);
            break (i + 4, len);
        }
        if buf.len() > 1 << 20 {
            return Ok(());
        }
    };
    while buf.len() < head_end + len {
        let n = sock.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut first = head.lines().next().unwrap_or("").split(' ');
    let method = first.next().unwrap_or("");
    let path = first.next().unwrap_or("");
    if path.trim_end_matches('/') != format!("/mcp/{token}") {
        return respond(&mut sock, "404 Not Found", r#"{"error":"unknown address"}"#);
    }
    if method != "POST" {
        return respond(&mut sock, "405 Method Not Allowed", r#"{"error":"use POST"}"#);
    }
    let Ok(msg) = serde_json::from_slice::<Value>(&buf[head_end..]) else {
        return respond(
            &mut sock,
            "400 Bad Request",
            r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"parse error"}}"#,
        );
    };
    // Notifications have no id and get no answer.
    let Some(id) = msg.get("id").cloned().filter(|i| !i.is_null()) else {
        return respond(&mut sock, "202 Accepted", "");
    };
    let result: Result<Value, (i64, String)> = match msg["method"].as_str().unwrap_or("") {
        "initialize" => Ok(json!({
            "protocolVersion": msg["params"]["protocolVersion"].as_str().unwrap_or("2025-03-26"),
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "minecraft-builder", "version": env!("CARGO_PKG_VERSION") },
            "instructions": "Minecraft Builder: a visual editor for Minecraft plugins (Paper) and mods (Fabric). Call get_project first, find blocks with list_blocks, then add_block / connect / set_value. Every flow starts at an event block; exec pins (in, next, …) define the order, the other pins carry values."
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => {
            let (reply, wait) = channel();
            let call = Call {
                tool: msg["params"]["name"].as_str().unwrap_or("").to_string(),
                args: msg["params"]["arguments"].clone(),
                reply,
            };
            let _ = tx.send(call);
            ctx.request_repaint();
            match wait.recv_timeout(std::time::Duration::from_secs(20)) {
                Ok(Ok(text)) => Ok(json!({ "content": [{ "type": "text", "text": text }] })),
                Ok(Err(e)) => Ok(json!({ "content": [{ "type": "text", "text": e }], "isError": true })),
                Err(_) => Ok(json!({
                    "content": [{ "type": "text", "text": "Minecraft Builder did not answer (is a project open?)" }],
                    "isError": true
                })),
            }
        }
        m => Err((-32601, format!("unknown method {m}"))),
    };
    let body = match result {
        Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
        Err((code, message)) => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }),
    };
    respond(&mut sock, "200 OK", &body.to_string())
}

/// Tool names with a short description (also shown in the Plugins panel).
pub const TOOL_INFO: &[(&str, &str, &str)] = &[
    ("get_project", "Projekt lesen: Blöcke, Werte, Verbindungen, Befehle", "Read the project: blocks, values, links, commands"),
    ("list_blocks", "Block-Bibliothek durchsuchen", "Search the block library"),
    ("add_block", "Block hinzufügen (und anhängen)", "Add a block (and attach it)"),
    ("connect", "Zwei Sockel verbinden", "Connect two sockets"),
    ("set_value", "Wert eines Blocks ändern", "Change a value of a block"),
    ("delete_block", "Block löschen", "Delete a block"),
    ("add_command", "Befehl mit Optionen anlegen", "Create a command with options"),
    ("get_problems", "Fehler und Warnungen lesen", "Read errors and warnings"),
    ("build", "Bauen starten", "Start a build"),
    ("get_console", "Konsole lesen", "Read the console"),
];

fn tools() -> Value {
    let obj = |props: Value, required: &[&str]| json!({ "type": "object", "properties": props, "required": required });
    json!([
        { "name": "get_project", "description": "Returns the open project as JSON: settings, workspaces with blocks (id, kind, values), links (from/to node and pin) and commands. Call this first.", "inputSchema": obj(json!({}), &[]) },
        { "name": "list_blocks", "description": "Lists blocks of the library with their pins and fields. Filter with a search text (name, id or category). Pins of type exec define the flow; other pins carry values.", "inputSchema": obj(json!({ "query": { "type": "string", "description": "Search text, e.g. 'teleport' or 'menu'. Empty lists all." } }), &[]) },
        { "name": "add_block", "description": "Adds a block to the current workspace. 'after' attaches its flow input to an exec output of an existing block (pin 'next' unless after_pin is given). 'values' sets fields and unconnected inputs by id. Returns the new block id.", "inputSchema": obj(json!({
            "kind": { "type": "string", "description": "Block id from list_blocks, e.g. send_message" },
            "after": { "type": "number", "description": "Id of the block this one runs after" },
            "after_pin": { "type": "string", "description": "Exec output of 'after', e.g. next, then, else, found" },
            "values": { "type": "object", "description": "Field / input values by id, all as strings" }
        }), &["kind"]) },
        { "name": "connect", "description": "Connects an output pin of one block to an input pin of another (exec to exec, or value to value of a compatible type).", "inputSchema": obj(json!({
            "from": { "type": "number" }, "from_pin": { "type": "string" },
            "to": { "type": "number" }, "to_pin": { "type": "string" }
        }), &["from", "from_pin", "to", "to_pin"]) },
        { "name": "set_value", "description": "Sets a field or an unconnected input of a block.", "inputSchema": obj(json!({
            "node": { "type": "number" }, "key": { "type": "string" }, "value": { "type": "string" }
        }), &["node", "key", "value"]) },
        { "name": "delete_block", "description": "Deletes a block and its links.", "inputSchema": obj(json!({ "node": { "type": "number" } }), &["node"]) },
        { "name": "add_command", "description": "Creates a chat command and its event block. Options become output pins opt0, opt1, … of the block. Returns the block id.", "inputSchema": obj(json!({
            "name": { "type": "string", "description": "Without slash, e.g. heal" },
            "description": { "type": "string" },
            "op_only": { "type": "boolean" },
            "options": { "type": "array", "items": { "type": "object", "properties": {
                "name": { "type": "string" },
                "type": { "type": "string", "enum": ["word", "text", "number", "integer", "player", "bool"] },
                "required": { "type": "boolean" }
            }, "required": ["name"] } }
        }), &["name"]) },
        { "name": "get_problems", "description": "Errors and warnings of the project (e.g. blocks that never run, unconnected inputs).", "inputSchema": obj(json!({}), &[]) },
        { "name": "build", "description": "Saves and starts building the .jar. Returns at once; read progress with get_console.", "inputSchema": obj(json!({}), &[]) },
        { "name": "get_console", "description": "The last lines of the builder's console (build output).", "inputSchema": obj(json!({ "lines": { "type": "number" } }), &[]) }
    ])
}

fn num(v: &Value) -> Option<u64> {
    v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)).or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

fn ty_name(t: PinType) -> &'static str {
    match t {
        PinType::Exec => "exec",
        PinType::Player => "player",
        PinType::Entity => "entity",
        PinType::Text => "text",
        PinType::Number => "number",
        PinType::Bool => "bool",
        PinType::Location => "location",
        PinType::Item => "item",
    }
}

/// Runs one tool against the app. Called on the UI thread.
pub fn handle(app: &mut App, ctx: &egui::Context, tool: &str, args: &Value) -> Result<String, String> {
    if app.screen != crate::app::Screen::Editor {
        return Err("No project is open in Minecraft Builder. Ask the user to open or create one.".into());
    }
    let platform = app.project.settings.platform;
    match tool {
        "get_project" => {
            let s = &app.project.settings;
            let ws: Vec<Value> = app
                .project
                .workspaces
                .iter()
                .enumerate()
                .map(|(i, w)| {
                    json!({
                        "name": w.name,
                        "current": i == app.active_ws,
                        "blocks": w.nodes.iter().map(|n| json!({
                            "id": n.id,
                            "kind": n.kind,
                            "name": blocks::get(&n.kind).map(|d| d.name.1).unwrap_or("?"),
                            "command": app.project.node_command(n).map(|c| c.usage()),
                            "values": n.values,
                        })).collect::<Vec<_>>(),
                        "links": w.links.iter().map(|l| json!({
                            "from": l.from.node, "from_pin": l.from.pin, "to": l.to.node, "to_pin": l.to.pin
                        })).collect::<Vec<_>>(),
                    })
                })
                .collect();
            Ok(serde_json::to_string_pretty(&json!({
                "name": s.name,
                "platform": s.platform.label(),
                "minecraft": s.versions.minecraft,
                "workspaces": ws,
                "note": "Blocks are added to the workspace marked current."
            }))
            .unwrap_or_default())
        }
        "list_blocks" => {
            let q = args["query"].as_str().unwrap_or("").to_lowercase();
            let mut out = vec![];
            for d in blocks::all().iter().filter(|d| d.supports(platform)) {
                let hay = format!("{} {} {} {} {}", d.id, d.name.0, d.name.1, d.cat.label().1, d.desc.1).to_lowercase();
                if !q.split_whitespace().all(|w| hay.contains(w)) {
                    continue;
                }
                let pins = |v: &[blocks::PinDef]| -> Vec<String> {
                    v.iter().map(|p| format!("{}:{}", p.id, ty_name(p.ty))).collect()
                };
                out.push(json!({
                    "kind": d.id,
                    "name": d.name.1,
                    "type": match d.kind { Kind::Event => "event", Kind::Action => "action", Kind::Flow => "flow", Kind::Value => "value" },
                    "category": d.cat.label().1,
                    "description": d.desc.1,
                    "inputs": pins(&d.inputs),
                    "outputs": pins(&d.outputs),
                    "fields": d.fields.iter().map(|f| format!("{} (default {:?})", f.id, f.default)).collect::<Vec<_>>(),
                }));
            }
            let total = out.len();
            out.truncate(40);
            Ok(serde_json::to_string_pretty(&json!({ "matches": total, "blocks": out })).unwrap_or_default())
        }
        "add_block" => {
            let kind = args["kind"].as_str().ok_or("kind is missing")?;
            let def = blocks::get(kind).ok_or(format!("Unknown block kind '{kind}'. Use list_blocks."))?;
            if !def.supports(platform) {
                return Err(format!("'{kind}' does not exist for {}", platform.short()));
            }
            if kind == "ev_command" {
                return Err("Use add_command to create a command block.".into());
            }
            let after = num(&args["after"]);
            let after_pin = args["after_pin"].as_str().unwrap_or("next").to_string();
            // Position: right of the block it follows, otherwise below everything.
            let ws = app.ws();
            let pos = match after.and_then(|a| ws.node(a)) {
                Some(n) => {
                    let branch = if after_pin == "next" { 0.0 } else { 240.0 };
                    [n.pos[0] + 320.0, n.pos[1] + branch]
                }
                None => {
                    let y = ws.nodes.iter().map(|n| n.pos[1]).fold(-288.0_f32, f32::max);
                    [32.0, y + 320.0]
                }
            };
            if let Some(a) = after {
                let n = ws.node(a).ok_or(format!("There is no block {a}"))?;
                let shape = blocks::shape(n, &app.project).ok_or("unknown block")?;
                if !shape.outputs.iter().any(|o| o.id == after_pin && o.ty == PinType::Exec) {
                    let execs: Vec<String> = shape.outputs.iter().filter(|o| o.ty == PinType::Exec).map(|o| o.id.to_string()).collect();
                    return Err(format!("Block {a} has no exec output '{after_pin}'. It has: {}", execs.join(", ")));
                }
                if !def.has_exec_in() {
                    return Err(format!("'{kind}' is a value/event block and cannot run after another block. Add it without 'after' and connect its value pins."));
                }
            }
            let id = app.add_node(kind, egui::pos2(pos[0], pos[1]));
            if let Some(obj) = args["values"].as_object() {
                if let Some(n) = app.ws_mut().node_mut(id) {
                    for (k, v) in obj {
                        let v = v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string());
                        n.values.insert(k.clone(), v);
                    }
                }
            }
            if let Some(a) = after {
                crate::ui::canvas::connect(
                    app.ws_mut(),
                    PinRef { node: a, pin: after_pin },
                    PinRef { node: id, pin: "in".into() },
                    true,
                );
            }
            Ok(format!("Added block {id} ({})", def.name.1))
        }
        "connect" => {
            let (from, to) = (num(&args["from"]).ok_or("from is missing")?, num(&args["to"]).ok_or("to is missing")?);
            let (fp, tp) = (
                args["from_pin"].as_str().ok_or("from_pin is missing")?.to_string(),
                args["to_pin"].as_str().ok_or("to_pin is missing")?.to_string(),
            );
            let ws = app.ws();
            let fnode = ws.node(from).ok_or(format!("There is no block {from}"))?;
            let tnode = ws.node(to).ok_or(format!("There is no block {to}"))?;
            let fs = blocks::shape(fnode, &app.project).ok_or("unknown block")?;
            let ts = blocks::shape(tnode, &app.project).ok_or("unknown block")?;
            let out = fs.output(&fp).ok_or_else(|| {
                format!("Block {from} has no output '{fp}'. Outputs: {}", fs.outputs.iter().map(|p| p.id.to_string()).collect::<Vec<_>>().join(", "))
            })?;
            let inp = ts.input(&tp).ok_or_else(|| {
                format!("Block {to} has no input '{tp}'. Inputs: {}", ts.inputs.iter().map(|p| p.id.to_string()).collect::<Vec<_>>().join(", "))
            })?;
            let exec = out.ty == PinType::Exec;
            if exec != (inp.ty == PinType::Exec) || (!exec && !blocks::compatible(out.ty, inp.ty)) {
                return Err(format!("Cannot connect {} to {}", ty_name(out.ty), ty_name(inp.ty)));
            }
            crate::ui::canvas::connect(app.ws_mut(), PinRef { node: from, pin: fp }, PinRef { node: to, pin: tp }, exec);
            Ok("Connected".into())
        }
        "set_value" => {
            let node = num(&args["node"]).ok_or("node is missing")?;
            let key = args["key"].as_str().ok_or("key is missing")?.to_string();
            let value = args["value"].as_str().map(str::to_string).unwrap_or_else(|| args["value"].to_string());
            let n = app.ws_mut().node_mut(node).ok_or(format!("There is no block {node}"))?;
            let def = blocks::get(&n.kind).ok_or("unknown block")?;
            if def.field(&key).is_none() && def.input(&key).is_none() {
                return Err(format!(
                    "Block {node} has no field or input '{key}'. Fields: {}. Inputs: {}",
                    def.fields.iter().map(|f| f.id).collect::<Vec<_>>().join(", "),
                    def.inputs.iter().map(|p| p.id.to_string()).collect::<Vec<_>>().join(", ")
                ));
            }
            n.values.insert(key, value);
            Ok("Value set".into())
        }
        "delete_block" => {
            let node: NodeId = num(&args["node"]).ok_or("node is missing")?;
            if app.ws().node(node).is_none() {
                return Err(format!("There is no block {node}"));
            }
            app.ws_mut().remove_node(node);
            app.selection.remove(&node);
            Ok("Deleted".into())
        }
        "add_command" => {
            use crate::model::{CmdAccess, CmdOption, CommandDef, OptType};
            let name = crate::codegen::clean_command(args["name"].as_str().ok_or("name is missing")?);
            if name.is_empty() {
                return Err("The command name is empty".into());
            }
            if app.project.commands.iter().any(|c| c.name == name) {
                return Err(format!("The command /{name} already exists"));
            }
            let cid = app.project.alloc_id();
            let mut c = CommandDef::new(cid, &name);
            c.description = args["description"].as_str().unwrap_or("").to_string();
            if args["op_only"].as_bool() == Some(true) {
                c.access = CmdAccess::Op;
            }
            for o in args["options"].as_array().cloned().unwrap_or_default() {
                let ty = match o["type"].as_str().unwrap_or("word") {
                    "text" => OptType::Text,
                    "number" => OptType::Number,
                    "integer" => OptType::Integer,
                    "player" => OptType::Player,
                    "bool" => OptType::Bool,
                    _ => OptType::Word,
                };
                c.options.push(CmdOption {
                    name: crate::codegen::clean_command(o["name"].as_str().unwrap_or("arg")),
                    description: String::new(),
                    ty,
                    required: o["required"].as_bool().unwrap_or(true),
                    choices: vec![],
                });
            }
            let usage = c.usage();
            let n_opts = c.options.len();
            app.project.commands.push(c);
            let y = app.ws().nodes.iter().map(|n| n.pos[1]).fold(-288.0_f32, f32::max) + 320.0;
            let id = app.project.alloc_id();
            app.ws_mut().nodes.push(Node {
                id,
                kind: "ev_command".into(),
                pos: [32.0, y],
                values: [("cmd".to_string(), cid.to_string())].into_iter().collect(),
            });
            Ok(format!(
                "Created {usage} as block {id}. Its outputs: next:exec, player:player{}",
                (0..n_opts).map(|i| format!(", opt{i}")).collect::<String>()
            ))
        }
        "get_problems" => {
            let out = crate::codegen::generate(&app.project);
            if out.problems.is_empty() {
                return Ok("No problems.".into());
            }
            Ok(out
                .problems
                .iter()
                .map(|p| {
                    format!(
                        "{} {}{}",
                        if p.error { "ERROR" } else { "warning" },
                        p.node.map(|n| format!("block {n}: ")).unwrap_or_default(),
                        p.msg
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "build" => {
            if app.job.is_some() {
                return Err("A build is already running.".into());
            }
            if app.path.is_none() {
                return Err("The project has not been saved yet. Ask the user to save it once.".into());
            }
            app.start_build(ctx, false);
            if app.job.is_some() {
                Ok("Build started. Read progress with get_console.".into())
            } else {
                Err("The build did not start – see get_problems.".into())
            }
        }
        "get_console" => {
            let n = num(&args["lines"]).unwrap_or(40).clamp(1, 300) as usize;
            let lines: Vec<String> = app
                .console
                .iter()
                .rev()
                .take(n)
                .map(|l| format!("{} {:?} {}", l.time, l.level, l.text))
                .collect();
            Ok(lines.into_iter().rev().collect::<Vec<_>>().join("\n"))
        }
        other => Err(format!("Unknown tool '{other}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rpc(port: u16, token: &str, body: &str) -> (String, String) {
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let req = format!(
            "POST /mcp/{token} HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        s.write_all(req.as_bytes()).unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        let (head, body) = out.split_once("\r\n\r\n").unwrap();
        (head.lines().next().unwrap().to_string(), body.to_string())
    }

    #[test]
    fn speaks_mcp_over_http() {
        let ctx = egui::Context::default();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let server = start(port, "secret".into(), ctx).unwrap();
        let (status, body) = rpc(port, "secret", r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#);
        assert!(status.contains("200"));
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(v["result"]["serverInfo"]["name"], "minecraft-builder");
        let (status, _) = rpc(port, "secret", r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
        assert!(status.contains("202"));
        let (_, body) = rpc(port, "secret", r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["result"]["tools"].as_array().unwrap().len(), TOOL_INFO.len());
        // Wrong token: nothing is revealed.
        let (status, _) = rpc(port, "wrong", r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#);
        assert!(status.contains("404"));
        // A tool call is handed to the UI thread and its answer returned.
        let t = std::thread::spawn(move || rpc(port, "secret", r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"get_problems","arguments":{}}}"#));
        let call = server.rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        assert_eq!(call.tool, "get_problems");
        call.reply.send(Ok("No problems.".into())).unwrap();
        let (_, body) = t.join().unwrap();
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["result"]["content"][0]["text"], "No problems.");
    }
}
