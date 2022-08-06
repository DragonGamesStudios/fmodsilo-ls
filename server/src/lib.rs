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

    use lsp_json::{ResponseError, IntegerOrString, ResponseMessage, error_codes, RequestMessage};

    use crate::{thread::ThreadPool, fmodsilo::Workspace};

    mod protocol {
        use std::sync::{Arc, Mutex};

        use lsp_json::{InitializeParams, InitializeResult, ResponseError, ServerInfo};

        use crate::fmodsilo::Workspace;

        use super::MessageManager;

        type WS = Arc<Mutex<Workspace>>;
        type MM = Arc<Mutex<MessageManager>>;

        pub fn initialize(_workspace: WS, _params: InitializeParams, _message_manager: &MM) -> Result<InitializeResult, ResponseError> {
            eprintln!("Hello from initialize!.");

            Ok(InitializeResult {
                server_info: Some(ServerInfo {
                    name: String::from("fmodsilo"),
                    version: Some(String::from(env!("CARGO_PKG_VERSION")))
                })
            })
        }

        pub fn on_initialized(_workspace: WS, _params: InitializeParams, _message_manager: &MM) {

        }
    }

    pub trait Sender: Send {
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
                            if self.current_message == "Content-Length:" {
                                self.stage = ReadingStage::ContentLength;
                            } else if self.current_message == "Content-Type:" {
                                self.stage = ReadingStage::ContentType;
                            } else {
                                return Err("Invalid header field");
                            }

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
        held_requests: HashMap<IntegerOrString, Value>
    }

    impl MessageManager {
        pub fn new(sender: Box<dyn Sender>) -> MessageManager {
            MessageManager {
                sender,
                held_requests: HashMap::new()
            }
        }

        pub fn send_response<R: Serialize>(&self, response: ResponseMessage<R>) -> Result<bool, &'static str> {
            let res = match serde_json::to_string(&response) {
                Ok(res) => res,
                Err(_) => return Err("Failed to serialize.")
            };

            self.sender.send(&res)
        }

        pub fn get_request(&self, key: &IntegerOrString) -> Option<Value> {
            match self.held_requests.get(key) {
                Some(req) => Some(req.clone()),
                None => None
            }
        }

        pub fn free_request(&mut self, key: &IntegerOrString) -> bool {
            self.held_requests.remove(key).is_some()
        }

        pub fn send_request<P: Serialize>(&mut self, request: RequestMessage<P>) -> Result<bool, &'static str> {
            let cloned_id = match &request.id {
                IntegerOrString::Integer(i) => IntegerOrString::Integer(*i),
                IntegerOrString::String(s) => IntegerOrString::String(s.clone())
            };

            let req_json = match to_value(request) {
                Ok(v) => v,
                Err(_) => return Err("Faild to serialize.")
            };

            let res = req_json.to_string();

            self.held_requests.insert(cloned_id, req_json);

            self.sender.send(&res)
        }
    }

    pub struct Server {
        listener: Box<dyn Listener>,
        message_manager: Arc<Mutex<MessageManager>>,
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
                message_manager: Arc::new(Mutex::new(MessageManager::new(sender))),
                workspace: Arc::new(Mutex::new(Workspace::new())),
                pool: ThreadPool::new(10),
                initialized: false,
                shutdown: false
            }
        }

        fn handle_message(&mut self, msg: &Value) -> Result<(), &'static str> {
            if let Some(params) = msg.get("params") {
                let method = match msg.get("method") {
                    Some(m) => match m.as_str() {
                        Some(s) => s,
                        None => return Err("Method should be a string.")
                    },
                    None => return Err("Missing field method.")
                };
                if let Some(id) = msg.get("id") {
                    // It's a request
                    let id = match id {
                        Value::Number(v) =>  match v.as_i64() {
                            Some(i) => IntegerOrString::Integer(i),
                            None => return Err("Id should be a string or an integer.")
                        },
                        Value::String(s) => IntegerOrString::String(s.clone()),
                        &_ => return Err("Id should be a string or an integer.")
                    };

                    if !self.shutdown {
                        if !self.initialized {
                            match method {
                                "initialize" => self.handle_request(protocol::initialize, params, id)?,
                                &_ => {
                                    self.message_manager.lock().unwrap().send_response(ResponseMessage::<()> {
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
                                &_ => ()
                            }
                        }
                    } else {
                        self.message_manager.lock().unwrap().send_response(ResponseMessage::<()> {
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
                        }
                        &_ => ()
                    };
                }
            } else {
                // It's a response
                if let Some(id) = msg.get("id") {
                    // It's a request
                    let id: IntegerOrString = match from_value(id.clone()) {
                        Ok(v) => v,
                        Err(_) => return Err("Id should be a string or an integer.")
                    };

                    let req_opt = self.message_manager.lock().unwrap().get_request(&id);

                    if let Some(req) = req_opt {
                        match req.get("method").unwrap().clone().as_str().unwrap() {
                            &_ => ()
                        };
                    } else {
                        return Err("Request with the specified id was not found.");
                    }
                }
            }

            Ok(())
        }

        pub fn push_message(&mut self, message: &String) -> Result<(), Vec<&'static str>> {
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
                F: FnOnce(Arc<Mutex<Workspace>>, P, &Arc<Mutex<MessageManager>>) -> LSPResult<R> + Send + 'static {
            let ws = Arc::clone(&self.workspace);
            let ps = match from_value(params.to_owned()) {
                Ok(p) => p,
                Err(_) => return Err("Could not parse params. Invalid JSON value.")
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
                    let manager = mm.lock().unwrap();
                    manager.send_response(response).unwrap();
                }
            });

            Ok(())
        }

        fn handle_notification<P, F>(&self, func: F, params: &Value) -> Result<(), &'static str>
            where
                P: for<'de> Deserialize<'de> + Send + 'static,
                F: FnOnce(Arc<Mutex<Workspace>>, P, &Arc<Mutex<MessageManager>>) + Send + 'static {
            let ws = Arc::clone(&self.workspace);
            let ps = match from_value(params.to_owned()) {
                Ok(p) => p,
                Err(_) => return Err("Could not parse params. Invalid JSON value.")
            };
            let mm = Arc::clone(&self.message_manager);

            self.pool.execute(move || {
                func(ws, ps, &mm);
            });

            Ok(())
        }

        fn handle_response<P, R, F>(&self, func: F, response: &Value, request: &Value, id: &IntegerOrString) -> Result<(), &'static str>
            where
                P: for<'de> Deserialize<'de> + Send + 'static,
                R: for<'de> Deserialize<'de> + Send + 'static,
                F: FnOnce(Arc<Mutex<Workspace>>, Option<P>, R, &Arc<Mutex<MessageManager>>) -> bool + Send + 'static {
            let ws = Arc::clone(&self.workspace);

            let response = match from_value(response.clone()) {
                Ok(resp) => resp,
                Err(_) => return Err("Failed to parse response.")
            };

            let mm = Arc::clone(&self.message_manager);

            let req = from_value(request.clone()).unwrap();

            let moved_id = match id {
                IntegerOrString::String(s) => IntegerOrString::String(s.clone()),
                IntegerOrString::Integer(i) => IntegerOrString::Integer(*i)
            };

            self.pool.execute(move || {
                if func(ws, response, req, &mm) {
                    mm.lock().unwrap().free_request(&moved_id);
                }
            });

            Ok(())
        }
    }
}
