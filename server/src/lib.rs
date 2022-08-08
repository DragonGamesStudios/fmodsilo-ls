#[cfg(test)]
mod tests {
    // use std::cell::RefCell;

    use crate::server::Listener;
    use crate::server::DefaultListener;
    // use crate::server::Sender;

    use serde_json::json;

    // struct MockSender {
    //     sent_messages: RefCell<Vec<String>>
    // }

    // impl MockSender {
    //     fn new() -> MockSender {
    //         MockSender { sent_messages: RefCell::new(vec![]) }
    //     }
    // }

    // impl Sender for MockSender {
    //     fn send(&self, message: &String) -> Result<bool, &'static str> {
    //         self.sent_messages.borrow_mut().push(message.clone());

    //         Ok(true)
    //     }
    // }

    #[test]
    fn process_message_integral() {
        let mut server = DefaultListener::new();
        let content = json!({
            "jsonrpc": "2.0"
        });
        let content_str = content.to_string();
        let msg = String::from(format!("Content-Length: {}\r\n\r\n{}", content_str.len(), content_str));

        let x = server.push_message(&msg).unwrap();
        assert_eq!(x[0], content);
    }

    #[test]
    fn process_message_sliced() {
        let mut server = DefaultListener::new();
        let content = json!({
            "jsonrpc": "2.0"
        });
        let content_str = content.to_string();
        let msg = String::from(format!("Content-Length: {}\r\n\r\n{}Content-", content_str.len(), content_str));

        let (first, second) = msg.split_at(12);
        server.push_message(&String::from(first)).unwrap();

        let x = server.push_message(&String::from(second)).unwrap();
        assert_eq!(x[0], content);
    }
}

mod thread;
mod fmodsilo;

pub mod server {
    use std::{sync::{Arc, Mutex}, vec, collections::HashMap, process};

    use serde::{Deserialize, Serialize};
    use serde_json::{from_value, Value, to_value};

    use lsp_json::{ResponseError, IntegerOrString, ResponseMessage, error_codes, RequestMessage, TraceValue, NotificationMessage, LogTraceNotification, LogTraceParams};
    use uuid::Uuid;

    use crate::{thread::ThreadPool, fmodsilo::Workspace};

    mod protocol {
        use std::sync::{Arc, Mutex};

        use lsp_json::{InitializeParams, InitializeResult, ResponseError, ServerInfo, TraceValue, SetTraceParams, ServerCapabilities, InitializedParams, ShutdownParams, ShutdownResult};

        use crate::fmodsilo::Workspace;

        use super::MessageManager;

        type WS = Arc<Mutex<Workspace>>;
        type MM = Arc<MessageManager>;

        pub fn initialize(_workspace: WS, params: InitializeParams, message_manager: &MM) -> Result<InitializeResult, ResponseError> {
            message_manager.set_trace(match params.trace {
                Some(x) => x,
                None => TraceValue::Off
            });

            if let TraceValue::Verbose = message_manager.get_trace() {
                message_manager.trace_log(String::from("Server initialized."), None);
            }

            Ok(InitializeResult {
                capabilities: ServerCapabilities {},
                server_info: Some(ServerInfo {
                    name: String::from("fmodsilo"),
                    version: Some(String::from(env!("CARGO_PKG_VERSION")))
                })
            })
        }
        pub fn shutdown(_workspace: WS, _params: Option<ShutdownParams>, message_manager: &MM) -> Result<ShutdownResult, ResponseError> {
            if let TraceValue::Verbose = message_manager.get_trace() {
                message_manager.trace_log(String::from("Server shut down."), None)
            }

            Ok(())
        }

        pub fn on_initialized(_workspace: WS, _params: InitializedParams, _message_manager: &MM) {

        }

        pub fn on_set_trace(_workspace: WS, params: SetTraceParams, message_manager: &MM) {
            message_manager.set_trace(params.value);
        }
    }

    pub trait Sender: Send + Sync {
        fn send(&self, message: &String) -> Result<bool, &'static str>;
    }

    pub trait Listener {
        /// Reads a message string and returns a vector of parsed JSON messages.
        fn push_message(&mut self, message: &String) -> Result<Vec<serde_json::Value>, &'static str>;
    }

    enum ReadingStage {
        HeaderField,
        ContentLength,
        ContentType,
        Content
    }

    pub struct DefaultListener {
        content_length: usize,
        content_type: String,
        current_message: String,
        stage: ReadingStage,
        ignore_next: bool
    }

    impl DefaultListener {
        pub fn new() -> DefaultListener {
            DefaultListener {
                content_length: 0,
                content_type: String::new(),
                current_message: String::new(),
                stage: ReadingStage::HeaderField,
                ignore_next: false
            }
        }
    }

    impl Listener for DefaultListener {
        /// Reads a message string and returns a vector of parsed JSON messages.
        fn push_message(&mut self, message: &String) -> Result<Vec<serde_json::Value>, &'static str> {
            let mut messages = vec![];

            for char in message.chars() {
                match self.stage {
                    ReadingStage::HeaderField => {
                        if char == ' ' {
                            self.stage = match self.current_message.as_str() {
                                "Content-Length:" => ReadingStage::ContentLength,
                                "Content-Type:" => ReadingStage::ContentType,
                                s => { eprintln!("{s}"); return Err("Invalid header field")}
                            };

                            self.current_message.clear();
                        } else if char == '\n' {
                            if self.current_message.ends_with('\r') && self.current_message.len() == 1 {
                                self.current_message.clear();
                                self.stage = ReadingStage::Content;
                            } else {
                                return Err("Invalid character when parsing headers");
                            }
                        } else {
                            self.current_message.push(char)
                        }
                    },
                    ReadingStage::ContentLength => {
                        if char == '\n' {
                            if self.current_message.ends_with('\r') {
                                self.content_length = match self.current_message.trim().parse() {
                                    Ok(v) => v,
                                    Err(_) => return Err("Error when parsing Content-Length.")
                                };
                                self.current_message.clear();
                                self.stage = ReadingStage::HeaderField;
                            } else {
                                return Err("Invalid line ending when parsing header field.");
                            }
                        } else {
                            self.current_message.push(char);
                        }
                    },
                    ReadingStage::ContentType => {
                        if char == '\n' && !self.ignore_next {
                            if self.current_message.ends_with('\r') {
                                self.content_type = String::from(self.current_message.trim());
                                self.current_message.clear();
                                self.stage = ReadingStage::HeaderField;
                            } else {
                                return Err("Invalid line ending when parsing header field.");
                            }
                        } else {
                            if char == '\\' && !self.ignore_next {
                                self.ignore_next = true;
                            } else {
                                self.current_message.push(char);
                            }
                        }
                    },
                    ReadingStage::Content => {
                        self.current_message.push(char);

                        if self.current_message.len() == self.content_length {
                            self.content_length = 0;
                            self.content_type = String::new();
                            self.stage = ReadingStage::HeaderField;

                            let parsed: serde_json::Value = match serde_json::from_str(self.current_message.as_str()) {
                                Ok(v) => v,
                                Err(_) => return Err("Error when parsing JSON content")
                            };

                            self.current_message.clear();

                            messages.push(parsed);
                        }
                    }
                }
            }

            Ok(messages)
        }
    }

    pub struct MessageManager {
        sender: Box<dyn Sender>,
        held_requests: Mutex<HashMap<String, Value>>,
        trace: Mutex<TraceValue>
    }

    impl MessageManager {
        pub fn new(sender: Box<dyn Sender>) -> MessageManager {
            MessageManager {
                sender,
                held_requests: Mutex::new(HashMap::new()),
                trace: Mutex::new(TraceValue::Off)
            }
        }

        pub fn send_response<R: Serialize>(&self, response: ResponseMessage<R>) -> Result<bool, &'static str> {
            let res = match serde_json::to_string(&response) {
                Ok(res) => res,
                Err(_) => return Err("Failed to serialize.")
            };

            if let Some(id) = response.id {
                match self.get_trace() {
                    TraceValue::Off => (),
                    TraceValue::Messages => self.trace_log(format!("Sending response {}", id), None),
                    TraceValue::Verbose =>
                        self.trace_log(format!("Sending response {}", id), Some(res.clone()))
                };
            }

            self.sender.send(&res)
        }

        pub fn send_notification<P: Serialize>(&self, notification: NotificationMessage<P>) -> Result<bool, &'static str> {
            let not = match serde_json::to_string(&notification) {
                Ok(res) => res,
                Err(_) => return Err("Failed to serialize.")
            };

            if notification.method != "$/logTrace" {
                match self.get_trace() {
                    TraceValue::Off => (),
                    TraceValue::Messages =>
                    self.trace_log(format!("Sending notification {}", &notification.method), None),
                    TraceValue::Verbose =>
                        self.trace_log(format!("Sending notification {}", &notification.method), Some(not.clone()))
                };
            }

            self.sender.send(&not)
        }

        pub fn get_request(&self, key: &String) -> Option<Value> {
            match self.held_requests.lock().unwrap().get(key) {
                Some(req) => Some(req.clone()),
                None => None
            }
        }

        pub fn free_request(&self, key: &String) -> bool {
            self.held_requests.lock().unwrap().remove(key).is_some()
        }

        pub fn send_request<P: Serialize>(&self, mut request: RequestMessage<P>) -> Result<bool, &'static str> {
            let uuid = Uuid::new_v4().to_string();
            request.id = IntegerOrString::String(uuid.clone());

            let req_json = match to_value(&request) {
                Ok(v) => v,
                Err(_) => return Err("Failed to serialize.")
            };

            match self.get_trace() {
                TraceValue::Off => (),
                TraceValue::Messages => self.trace_log(format!("Sending request {} {}", &request.method, &request.id), None),
                TraceValue::Verbose =>
                    self.trace_log(format!("Sending request {} {}", &request.method, &request.id), Some(format!("{req_json}")))
            };

            let res = req_json.to_string();

            self.held_requests.lock().unwrap().insert(uuid, req_json);

            self.sender.send(&res)
        }

        pub fn set_trace(&self, trace: TraceValue) {
            if let TraceValue::Verbose = self.get_trace() {
                self.trace_log(format!("Setting trace value to {}", serde_json::to_string(&trace).unwrap()), None)
            }

            *self.trace.lock().unwrap() = trace;
        }

        pub fn get_trace(&self) -> TraceValue {
            self.trace.lock().unwrap().clone()
        }

        /// Note: this does not check if the trace value is set to the right values.
        pub fn trace_log(&self, message: String, verbose: Option<String>) {
            self.send_notification(LogTraceNotification {
                jsonrpc: String::from("2.0"),
                method: String::from("$/logTrace"),
                params: Some(LogTraceParams {
                    message,
                    verbose
                })
            }).unwrap();
        }
    }

    pub struct Server {
        listener: Box<dyn Listener>,
        message_manager: Arc<MessageManager>,
        workspace: Arc<Mutex<Workspace>>,
        pool: ThreadPool,
        initialized: bool,
        shutdown: bool
    }

    type LSPResult<T> = Result<T, ResponseError>;

    impl Server {
        pub fn new(listener: Box<dyn Listener>, sender: Box<dyn Sender>) -> Server {
            Server{
                listener,
                message_manager: Arc::new(MessageManager::new(sender)),
                workspace: Arc::new(Mutex::new(Workspace::new())),
                pool: ThreadPool::new(10),
                initialized: false,
                shutdown: false
            }
        }

        fn handle_message(&mut self, msg: &Value) -> Result<(), String> {
            if let Some(params) = msg.get("params") {
                let method = match msg.get("method") {
                    Some(m) => match m.as_str() {
                        Some(s) => s,
                        None => return Err(String::from("Method should be a string."))
                    },
                    None => return Err(String::from("Missing field method."))
                };
                if let Some(id) = msg.get("id") {
                    // It's a request
                    let id = match id {
                        Value::Number(v) =>  match v.as_i64() {
                            Some(i) => IntegerOrString::Integer(i),
                            None => return Err(String::from("Id should be a string or an integer."))
                        },
                        Value::String(s) => IntegerOrString::String(s.clone()),
                        &_ => return Err(String::from("Id should be a string or an integer."))
                    };

                    match self.message_manager.get_trace() {
                        TraceValue::Messages =>
                            self.message_manager.trace_log(format!("Received request {id} {method}."), None),
                        TraceValue::Verbose =>
                            self.message_manager.trace_log(format!("Received request {id} {method}.",), Some(msg.to_string())),
                        TraceValue::Off => ()
                    }

                    if !self.shutdown {
                        if !self.initialized {
                            match method {
                                "initialize" => self.handle_request(protocol::initialize, params, id)?,
                                &_ => {
                                    self.message_manager.send_response(ResponseMessage::<()> {
                                        jsonrpc: String::from("2.0"),
                                        id: Some(id),
                                        result: None,
                                        error: Some(ResponseError{
                                            code: error_codes::SERVER_NOT_INITIALIZED,
                                            message: String::from("Server was not initialized."),
                                            data: None
                                        })
                                    }).unwrap();
                                },
                            };
                        } else {
                            match method {
                                "shutdown" => {
                                    self.handle_request(protocol::shutdown, params, id)?;
                                    self.shutdown = true;
                                },
                                met => {
                                    self.message_manager.send_response(ResponseMessage::<()> {
                                        jsonrpc: String::from("2.0"),
                                        id: Some(id),
                                        result: None,
                                        error: Some(ResponseError {
                                            code: error_codes::METHOD_NOT_FOUND,
                                            message: format!("Couldn'd find function bound for method {met}."),
                                            data: None
                                        })
                                    }).unwrap();
                                }
                            }
                        }
                    } else {
                        self.message_manager.send_response(ResponseMessage::<()> {
                            jsonrpc: String::from("2.0"),
                            id: Some(id),
                            result: None,
                            error: Some(ResponseError{
                                code: error_codes::INVALID_REQUEST,
                                message: String::from("Server was shut down."),
                                data: None
                            })
                        }).unwrap();
                    }
                } else {
                    // It's a notification
                    match self.message_manager.get_trace() {
                        TraceValue::Messages =>
                            self.message_manager.trace_log(format!("Received notification {method}."), None),
                        TraceValue::Verbose =>
                            self.message_manager.trace_log(format!("Received notification {method}."), Some(msg.to_string())),
                        TraceValue::Off => ()
                    }

                    if !self.initialized && method == "initialized" {
                        self.initialized = true;
                    }

                    match method {
                        "initialized" => self.handle_notification(protocol::on_initialized, params)?,
                        "exit" => {
                            if self.shutdown {
                                process::exit(0);
                            } else {
                                process::exit(1);
                            }
                        },
                        "$/setTrace" => self.handle_notification(protocol::on_set_trace, params)?,
                        &_ => ()
                    };
                }
            } else {
                // It's a response
                if let Some(id) = msg.get("id") {
                    let id: IntegerOrString = match from_value(id.clone()) {
                        Ok(v) => v,
                        Err(_) => return Err(String::from("Id should be a string or an integer."))
                    };
                    
                    match self.message_manager.get_trace() {
                        TraceValue::Messages =>
                            self.message_manager.trace_log(format!("Received response {id}."), None),
                        TraceValue::Verbose =>
                            self.message_manager.trace_log(format!("Received response {id}."), Some(msg.to_string())),
                        TraceValue::Off => ()
                    }

                    match id {
                        IntegerOrString::String(s) => {
                            let req_opt = self.message_manager.get_request(&s);
        
                            if let Some(req) = req_opt {
                                match req.get("method").unwrap().clone().as_str().unwrap() {
                                    &_ => ()
                                };
                            } else {
                                return Err(format!("Request with the specified id {} was not found.", &s));
                            }
                        },
                        // Since FModSilo doesn't use integer ids, we can be sure we will only receive string-identified responses.
                        IntegerOrString::Integer(_) => return Err(format!("Request with the specified id {} was not found.", &id))
                    }
                }
            }

            Ok(())
        }

        pub fn push_message(&mut self, message: &String) -> Result<(), Vec<String>> {
            let messages = self.listener.push_message(message).unwrap();

            let mut errors = vec![];

            for msg in messages.iter() {
                match self.handle_message(msg) {
                    Ok(_) => (),
                    Err(s) => errors.push(s)
                }
            }

            if errors.len() > 0 {
                return Err(errors);
            }

            Ok(())
        }

        fn handle_request<P, R, F>(&self, func: F, params: &Value, id: IntegerOrString) -> Result<(), &'static str>
            where
                P: for<'de> Deserialize<'de> + Send + 'static,
                R: Serialize,
                F: FnOnce(Arc<Mutex<Workspace>>, P, &Arc<MessageManager>) -> LSPResult<R> + Send + 'static {
            let ws = Arc::clone(&self.workspace);
            let ps = match from_value(params.to_owned()) {
                Ok(p) => p,
                Err(_) => {
                    self.message_manager.send_response(ResponseMessage::<()> {
                        jsonrpc: String::from("2.0"),
                        id: Some(id),
                        result: None,
                        error: Some(ResponseError{
                            code: error_codes::PARSE_ERROR,
                            message: String::from("Could not parse params."),
                            data: None
                        })
                    }).unwrap();

                    return Err("Could not parse params. Invalid JSON value.")
                }
            };
            let mm = Arc::clone(&self.message_manager);

            self.pool.execute(move || {
                let (result, error) = match func(ws, ps, &mm) {
                    Ok(res) => (Some(res), None),
                    Err(err) => (None, Some(err))
                };

                let response = ResponseMessage::<R> {
                    jsonrpc: String::from("2.0"),
                    id: Some(id),
                    result,
                    error
                };

                {
                    mm.send_response(response).unwrap();
                }
            });

            Ok(())
        }

        fn handle_notification<P, F>(&self, func: F, params: &Value) -> Result<(), String>
            where
                P: for<'de> Deserialize<'de> + Send + 'static,
                F: FnOnce(Arc<Mutex<Workspace>>, P, &Arc<MessageManager>) + Send + 'static {
            let ws = Arc::clone(&self.workspace);
            let ps = match from_value(params.to_owned()) {
                Ok(p) => p,
                Err(err) =>
                    return Err(format!("Could not parse params. Invalid JSON value. At {}:{}: {}.", err.line(), err.column(), err.to_string()))
            };
            let mm = Arc::clone(&self.message_manager);

            self.pool.execute(move || {
                func(ws, ps, &mm);
            });

            Ok(())
        }

        fn handle_response<P, R, F>(&self, func: F, response: &Value, request: &Value, id: &String) -> Result<(), &'static str>
            where
                P: for<'de> Deserialize<'de> + Send + 'static,
                R: for<'de> Deserialize<'de> + Send + 'static,
                F: FnOnce(Arc<Mutex<Workspace>>, Option<P>, R, &Arc<MessageManager>) -> bool + Send + 'static {
            let ws = Arc::clone(&self.workspace);

            let response = match from_value(response.clone()) {
                Ok(resp) => resp,
                Err(_) => return Err("Failed to parse response.")
            };

            let mm = Arc::clone(&self.message_manager);

            let req = from_value(request.clone()).unwrap();

            let moved_id = id.clone();

            self.pool.execute(move || {
                if func(ws, response, req, &mm) {
                    mm.free_request(&moved_id);
                }
            });

            Ok(())
        }
    }
}
