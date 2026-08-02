use super::types::{LspError, LspProtocolError, RustAnalyzerConfig};
use crate::processes::{
    configure_managed_process, terminate_managed_process,
    terminate_remaining_managed_process_group,
};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;
const SHUTDOWN_GRACE: Duration = Duration::from_millis(250);
const TERMINATION_GRACE: Duration = Duration::from_millis(250);

pub(super) struct LspConnection {
    child: Child,
    pub(super) system_pid: u32,
    stdin: ChildStdin,
    inbound: Receiver<Inbound>,
    queued_notifications: VecDeque<Value>,
    workspace_folders: Vec<Value>,
    next_request_id: u64,
    stdout_reader: Option<JoinHandle<()>>,
    stderr_reader: Option<JoinHandle<io::Result<Vec<u8>>>>,
    stopped: bool,
}

impl LspConnection {
    pub(super) fn spawn(config: &RustAnalyzerConfig) -> Result<Self, LspError> {
        let mut command = Command::new(config.executable());
        command
            .args(config.arguments())
            .current_dir(config.workspace_root())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure_managed_process(&mut command);
        let mut child = command
            .spawn()
            .map_err(|error| LspError::Spawn(error.to_string()))?;
        let system_pid = child.id();
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| LspError::Spawn("language server stdin was not piped".to_owned()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| LspError::Spawn("language server stdout was not piped".to_owned()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| LspError::Spawn("language server stderr was not piped".to_owned()))?;
        let (sender, inbound) = mpsc::channel();
        let stdout_reader = thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_message(&mut reader) {
                    Ok(Some(message)) => {
                        if sender.send(Inbound::Message(message)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        let _ = sender.send(Inbound::End);
                        break;
                    }
                    Err(error) => {
                        let _ = sender.send(Inbound::Protocol(error));
                        break;
                    }
                }
            }
        });
        let stderr_reader = thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes)?;
            Ok(bytes)
        });
        Ok(Self {
            child,
            system_pid,
            stdin,
            inbound,
            queued_notifications: VecDeque::new(),
            workspace_folders: Vec::new(),
            next_request_id: 1,
            stdout_reader: Some(stdout_reader),
            stderr_reader: Some(stderr_reader),
            stopped: false,
        })
    }

    pub(super) fn set_workspace_folder(&mut self, uri: String, name: &str) {
        self.workspace_folders = vec![json!({"uri": uri, "name": name})];
    }

    pub(super) fn request(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, LspError> {
        let request_id = self.next_request_id;
        self.next_request_id = self
            .next_request_id
            .checked_add(1)
            .ok_or(LspError::RequestIdExhausted)?;
        self.write(json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params,
        }))?;
        let deadline = Instant::now().checked_add(timeout).ok_or(LspError::Timeout)?;
        loop {
            let message = self.receive_until(deadline)?;
            if let Some(response) = response_for_id(&message, request_id)? {
                return response;
            }
            self.handle_server_message(message)?;
        }
    }

    pub(super) fn notify(&mut self, method: &str, params: Value) -> Result<(), LspError> {
        self.write(json!({"jsonrpc": "2.0", "method": method, "params": params}))
    }

    pub(super) fn wait_for_notification(
        &mut self,
        method: &str,
        timeout: Duration,
    ) -> Result<Value, LspError> {
        if let Some(index) = self.queued_notifications.iter().position(|message| {
            message.get("method").and_then(Value::as_str) == Some(method)
        }) {
            let message = self
                .queued_notifications
                .remove(index)
                .expect("queued notification index was found");
            return notification_params(message, method);
        }
        let deadline = Instant::now().checked_add(timeout).ok_or(LspError::Timeout)?;
        loop {
            let message = self.receive_until(deadline)?;
            if message.get("method").and_then(Value::as_str) == Some(method)
                && message.get("id").is_none()
            {
                return notification_params(message, method);
            }
            self.handle_server_message(message)?;
        }
    }

    fn handle_server_message(&mut self, message: Value) -> Result<(), LspError> {
        let object = message
            .as_object()
            .ok_or_else(|| LspError::Protocol(LspProtocolError::InvalidMessageShape))?;
        let Some(method) = object.get("method").and_then(Value::as_str) else {
            return Err(LspError::Protocol(LspProtocolError::UnexpectedResponse));
        };
        if let Some(id) = object.get("id") {
            let result = match method {
                "workspace/configuration" => {
                    let count = object
                        .get("params")
                        .and_then(|params| params.get("items"))
                        .and_then(Value::as_array)
                        .map_or(0, Vec::len);
                    Value::Array((0..count).map(|_| Value::Null).collect())
                }
                "client/registerCapability"
                | "client/unregisterCapability"
                | "window/workDoneProgress/create" => Value::Null,
                "workspace/workspaceFolders" => Value::Array(self.workspace_folders.clone()),
                _ => {
                    self.write(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32601,
                            "message": format!("unsupported server request: {method}"),
                        }
                    }))?;
                    return Ok(());
                }
            };
            self.write(json!({"jsonrpc": "2.0", "id": id, "result": result}))?;
        } else {
            self.queued_notifications.push_back(message);
        }
        Ok(())
    }

    fn receive_until(&mut self, deadline: Instant) -> Result<Value, LspError> {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or(LspError::Timeout)?;
        match self.inbound.recv_timeout(remaining) {
            Ok(Inbound::Message(message)) => Ok(message),
            Ok(Inbound::Protocol(error)) => Err(LspError::Protocol(error)),
            Ok(Inbound::End) => Err(LspError::ServerExited(self.stderr_if_finished())),
            Err(RecvTimeoutError::Timeout) => Err(LspError::Timeout),
            Err(RecvTimeoutError::Disconnected) => {
                Err(LspError::ServerExited(self.stderr_if_finished()))
            }
        }
    }

    fn write(&mut self, message: Value) -> Result<(), LspError> {
        write_message(&mut self.stdin, &message).map_err(LspError::Protocol)
    }

    pub(super) fn shutdown(&mut self) -> Result<(), LspError> {
        if self.stopped {
            return Ok(());
        }
        let request_result = self.request("shutdown", Value::Null, SHUTDOWN_GRACE);
        let exit_result = self.notify("exit", Value::Null);
        let stop_result = self.stop_child();
        self.stopped = stop_result.is_ok();
        request_result?;
        exit_result?;
        stop_result
    }

    fn stop_child(&mut self) -> Result<(), LspError> {
        let started = Instant::now();
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => {
                    terminate_remaining_managed_process_group(self.system_pid);
                    break;
                }
                Ok(None) if started.elapsed() < SHUTDOWN_GRACE => {
                    thread::sleep(Duration::from_millis(5));
                }
                Ok(None) => {
                    terminate_managed_process(
                        &mut self.child,
                        self.system_pid,
                        Duration::from_millis(5),
                        TERMINATION_GRACE,
                    )
                    .map_err(|error| LspError::Termination(error.to_string()))?;
                    break;
                }
                Err(error) => return Err(LspError::Termination(error.to_string())),
            }
        }
        if let Some(handle) = self.stdout_reader.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.stderr_reader.take() {
            let _ = handle.join();
        }
        Ok(())
    }

    fn stderr_if_finished(&mut self) -> String {
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            return String::new();
        }
        terminate_remaining_managed_process_group(self.system_pid);
        let Some(handle) = self.stderr_reader.take() else {
            return String::new();
        };
        match handle.join() {
            Ok(Ok(bytes)) => String::from_utf8_lossy(&bytes).into_owned(),
            Ok(Err(error)) => format!("stderr read failed: {error}"),
            Err(_) => "stderr reader thread panicked".to_owned(),
        }
    }
}

impl Drop for LspConnection {
    fn drop(&mut self) {
        let readers_can_finish = if self.stopped {
            true
        } else {
            let terminated = terminate_managed_process(
                &mut self.child,
                self.system_pid,
                Duration::from_millis(5),
                TERMINATION_GRACE,
            )
            .is_ok();
            self.stopped = true;
            terminated || matches!(self.child.try_wait(), Ok(Some(_)))
        };
        if readers_can_finish {
            terminate_remaining_managed_process_group(self.system_pid);
            if let Some(handle) = self.stdout_reader.take() {
                let _ = handle.join();
            }
            if let Some(handle) = self.stderr_reader.take() {
                let _ = handle.join();
            }
        }
    }
}

#[derive(Debug)]
enum Inbound {
    Message(Value),
    Protocol(LspProtocolError),
    End,
}

fn response_for_id(message: &Value, request_id: u64) -> Result<Option<Value>, LspError> {
    let Some(object) = message.as_object() else {
        return Err(LspError::Protocol(LspProtocolError::InvalidMessageShape));
    };
    let Some(id) = object.get("id") else {
        return Ok(None);
    };
    if object.get("method").is_some() {
        return Ok(None);
    }
    if id.as_u64() != Some(request_id) {
        return Err(LspError::Protocol(LspProtocolError::UnexpectedResponseId));
    }
    if let Some(error) = object.get("error").and_then(Value::as_object) {
        let code = error.get("code").and_then(Value::as_i64).unwrap_or(-32603);
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("remote JSON-RPC error")
            .to_owned();
        return Err(LspError::Remote { code, message });
    }
    Ok(Some(object.get("result").cloned().unwrap_or(Value::Null)))
}

fn notification_params(message: Value, expected_method: &str) -> Result<Value, LspError> {
    let object = message
        .as_object()
        .ok_or_else(|| LspError::Protocol(LspProtocolError::InvalidMessageShape))?;
    if object.get("method").and_then(Value::as_str) != Some(expected_method)
        || object.get("id").is_some()
    {
        return Err(LspError::Protocol(LspProtocolError::InvalidMessageShape));
    }
    Ok(object.get("params").cloned().unwrap_or(Value::Null))
}

fn write_message(writer: &mut impl Write, message: &Value) -> Result<(), LspProtocolError> {
    let body = serde_json::to_vec(message)
        .map_err(|error| LspProtocolError::InvalidJson(error.to_string()))?;
    if body.len() > MAX_MESSAGE_BYTES {
        return Err(LspProtocolError::MessageTooLarge { length: body.len() });
    }
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())
        .map_err(|error| LspProtocolError::Io(error.to_string()))?;
    writer
        .write_all(&body)
        .map_err(|error| LspProtocolError::Io(error.to_string()))?;
    writer
        .flush()
        .map_err(|error| LspProtocolError::Io(error.to_string()))
}

fn read_message(reader: &mut impl BufRead) -> Result<Option<Value>, LspProtocolError> {
    let mut content_length = None;
    let mut read_any = false;
    loop {
        let mut line = String::new();
        let count = reader
            .read_line(&mut line)
            .map_err(|error| LspProtocolError::Io(error.to_string()))?;
        if count == 0 {
            if read_any {
                return Err(LspProtocolError::MissingContentLength);
            }
            return Ok(None);
        }
        read_any = true;
        if line == "\r\n" || line == "\n" {
            break;
        }
        let line = line.trim_end_matches(&['\r', '\n'][..]);
        let Some((name, value)) = line.split_once(':') else {
            return Err(LspProtocolError::InvalidContentLength);
        };
        if name.eq_ignore_ascii_case("Content-Length") {
            if content_length.is_some() {
                return Err(LspProtocolError::DuplicateContentLength);
            }
            let length = value
                .trim()
                .parse::<usize>()
                .map_err(|_| LspProtocolError::InvalidContentLength)?;
            if length > MAX_MESSAGE_BYTES {
                return Err(LspProtocolError::MessageTooLarge { length });
            }
            content_length = Some(length);
        }
    }
    let length = content_length.ok_or(LspProtocolError::MissingContentLength)?;
    let mut body = vec![0_u8; length];
    reader
        .read_exact(&mut body)
        .map_err(|error| LspProtocolError::Io(error.to_string()))?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|error| LspProtocolError::InvalidJson(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn framed_json_round_trips_exactly() {
        let message = json!({"jsonrpc":"2.0","id":7,"method":"test","params":{"x":1}});
        let mut bytes = Vec::new();
        write_message(&mut bytes, &message).expect("write frame");
        let decoded = read_message(&mut BufReader::new(Cursor::new(bytes)))
            .expect("read frame")
            .expect("one frame");
        assert_eq!(decoded, message);
    }

    #[test]
    fn framing_rejects_missing_duplicate_and_invalid_lengths() {
        for bytes in [
            &b"X-Test: 1\r\n\r\n{}"[..],
            &b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}"[..],
            &b"Content-Length: nope\r\n\r\n{}"[..],
        ] {
            assert!(read_message(&mut BufReader::new(Cursor::new(bytes))).is_err());
        }
    }
}
