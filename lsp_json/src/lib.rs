#[cfg(test)]
mod tests {
}


use std::collections::HashMap;

use serde::{Serialize, Deserialize, de::Visitor};

/// Defines an integer number in the range of -2^31 to 2^31 - 1.
type Integer = i64;

/// Defines an unsigned integer number in the range of 0 to 2^31 - 1.
type Uinteger = u64;

/// Defines a decimal number. Since decimal numbers are very
/// rare in the language server specification we denote the
/// exact range with every decimal using the mathematics
/// interval notation (e.g. \[0, 1] denotes all decimals d with
/// 0 <= d <= 1.
type Decimal = f64;

/// The LSP any type
#[derive(Debug)]
pub enum LSPAny {
    Object(Box<LSPObject>),
    Array(Box<LSPArray>),
    String(String),
    Integer(Integer),
    UInteger(Uinteger),
    Decimal(Decimal),
    Boolean(bool),
    Null
}

impl Serialize for LSPAny {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        match self {
            Self::Object(obj) => obj.serialize(serializer),
            Self::Array(arr) => arr.serialize(serializer),
            Self::Boolean(b) => serializer.serialize_bool(*b),
            Self::Decimal(d) => serializer.serialize_f64(*d),
            Self::Integer(i) => serializer.serialize_i64(*i),
            Self::String(s) => s.serialize(serializer),
            Self::UInteger(ui) => serializer.serialize_u64(*ui),
            Self::Null => serializer.serialize_unit()
        }
    }
}
struct LSPAnyVisitor;

impl<'de> Visitor<'de> for LSPAnyVisitor {
    type Value = LSPAny;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("object, array, boolean, decimal, integer, string or null.")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where
            E: serde::de::Error, {
        Ok(LSPAny::Boolean(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error, {
        Ok(LSPAny::Decimal(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error, {
        Ok(LSPAny::Integer(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error, {
        Ok(LSPAny::UInteger(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error, {
        Ok(LSPAny::String(String::from(v)))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error, {
        Ok(LSPAny::Null)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>, {
        let mut v = Box::new(vec![]);

        while let Some(element) = seq.next_element()? {
            v.push(element);
        }

        Ok(LSPAny::Array(v))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>, {
        let mut m = Box::new(HashMap::new());

        while let Some((key, value)) = map.next_entry()? {
            m.insert(key, value);
        }

        Ok(LSPAny::Object(m))
    }
}

impl<'de> Deserialize<'de> for LSPAny {

    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        deserializer.deserialize_any(LSPAnyVisitor)
    }
}

/// LSP object definition.
type LSPObject = HashMap<String, LSPAny>;

/// LSP arrays.
type LSPArray = Vec<LSPAny>;

/// Message id type
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum IntegerOrString {
    Integer(Integer),
    String(String)
}

impl Serialize for IntegerOrString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        match self {
            Self::Integer(i) => serializer.serialize_i64(*i),
            Self::String(s) => s.serialize(serializer),
        }
    }
}
struct IntegerOrStringVisitor;

impl<'de> Visitor<'de> for IntegerOrStringVisitor {
    type Value = IntegerOrString;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("integer or string.")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error, {
        Ok(IntegerOrString::Integer(v))
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
        where
            E: serde::de::Error, {
        Ok(IntegerOrString::Integer(v as i64))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error, {
        Ok(IntegerOrString::String(String::from(v)))
    }
}

impl<'de> Deserialize<'de> for IntegerOrString {

    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        deserializer.deserialize_any(IntegerOrStringVisitor)
    }
}

/// A request message to describe a request between the client and the server.
/// Every processed request must send a response back to the sender of the request.
#[derive(Debug, Deserialize, Serialize)]
pub struct RequestMessage<T> {
    /// jsonrpc version. LSP uses 2.0
    pub jsonrpc: String,
    /// The request id.
    pub id: IntegerOrString,
    /// The method to be invoked.
    pub method: String,
    /// The method's params
    pub params: Option<T>
}

/// A Response Message sent as a result of a request. If a request doesn’t provide a result value
/// the receiver of a request still needs to return a response message to conform to the JSON-RPC specification.
/// The result property of the ResponseMessage should be set to null in this case to signal a successful request.
#[derive(Debug, Deserialize, Serialize)]
pub struct ResponseMessage<T> {
    /// jsonrpc version. LSP uses 2.0
    pub jsonrpc: String,
    /// The request id.
    pub id: Option<IntegerOrString>,
    /// The result of a request. This member is REQUIRED on success.
    /// This member MUST NOT exist if there was an error invoking the method.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub result: Option<T>,
    /// The error object in case a request fails.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub error: Option<ResponseError>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseError {
    /// A number indicating the error type that occured.
    pub code: Integer,
    /// A string providing a short description of the error.
    pub message: String,
    /// A primitive or structured value that contains additional
    /// information about the error. Can be omitted.
    /// 
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub data: Option<LSPAny>
}

#[allow(dead_code)]
pub mod error_codes {
    use super::Integer;
    // Defined by JSON-RPC
    pub const PARSE_ERROR: Integer = -32700;
    pub const INVALID_REQUEST: Integer = -32600;
    pub const METHOD_NOT_FOUND: Integer = -32701;
    pub const INVALID_PARAMS: Integer = -32702;
    pub const INTERNAL_ERROR: Integer = -32703;

	/// This is the start range of JSON-RPC reserved error codes.
	/// It doesn't denote a real error code. No LSP error codes should
	/// be defined between the start and end range. For backwards
	/// compatibility the `ServerNotInitialized` and the `UnknownErrorCode`
	/// are left in the range.
    pub const JSONRPC_RESERVED_ERROR_RANGE_START: Integer = -32099;
    /// Deprecated: use `JSONRPC_RESERVED_ERROR_RANGE_START` instead.
    #[deprecated]
    pub const SERVER_ERROR_START: Integer = JSONRPC_RESERVED_ERROR_RANGE_START;

    /// Error code indicating that a server received a notification or
    /// request before the server has received the `initialize` request.
    pub const SERVER_NOT_INITIALIZED: Integer = -32002;
    pub const UNKNOWN_ERROR_CODE: Integer = -32001;

	/// This is the end range of JSON-RPC reserved error codes.
	/// It doesn't denote a real error code.
	pub const JSONRPC_RESERVED_ERROR_RANGE_END: Integer = -32000;
    /// Deprecated: use 
	#[deprecated]
	pub const SERVER_ERROR_END: Integer = JSONRPC_RESERVED_ERROR_RANGE_END;

	/// This is the start range of LSP reserved error codes.
	/// It doesn't denote a real error code.
	pub const LSP_RESERVED_ERROR_RANGE_START: Integer = -32899;

	/// A request failed but it was syntactically correct, e.g the
	/// method name was known and the parameters were valid. The error
	/// message should contain human readable information about why
	/// the request failed.
	pub const REQUEST_FAILED: Integer = -32803;

	/// The server cancelled the request. This error code should
	/// only be used for requests that explicitly support being
	/// server cancellable.
	pub const SERVER_CANCELLED: Integer = -32802;

	/// The server detected that the content of a document got
	/// modified outside normal conditions. A server should
	/// NOT send this error code if it detects a content change
	/// in it unprocessed messages. The result even computed
	/// on an older state might still be useful for the client.
	///
	/// If a client decides that a result is not of any use anymore
	/// the client should cancel the request.
	pub const CONTENT_MODIFIED: Integer = -32801;

	/// The client has canceled a request and a server as detected
	/// the cancel.
	pub const REQUEST_CANCELLED: Integer = -32800;

	/// This is the end range of LSP reserved error codes.
	/// It doesn't denote a real error code.
	pub const LSP_RESERVED_ERROR_RANGE_END: Integer = -32800;
}

/// A notification message. A processed notification message must not send a response back. They work like events.
#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationMessage<T> {
    /// The method to be invoked.
    method: String,
    /// The notification's params
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    params: Option<T>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CancelParams {
    /// The request id to cancel.
    id: IntegerOrString
}

/// The base protocol offers support for request cancellation.
/// A request A request that got canceled still needs to return from the server and send a response back.
/// It can not be left open / hanging. This is in line with the JSON-RPC protocol
/// that requires that every request sends a response back. In addition it allows for
/// returning partial results on cancel. If the request returns an error response on cancellation it is
/// advised to set the error code to `ErrorCodes.RequestCancelled`.
/// To cancel a request, a notification message with the following properties is sent:
/// # Notification
/// * `method`: '$/cancelRequest'
/// * `params`: CancelParams
#[allow(dead_code)]
pub type CancelNotification = NotificationMessage<CancelParams>;

pub type ProgressToken = IntegerOrString;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProgressParams<T> {
    /// The progress token provided by the client or server.
    token: ProgressToken,
    /// The progress data.
    value: T
}

/// The base protocol offers also support to report progress in a generic fashion.
/// This mechanism can be used to report any kind of progress including work done progress
/// (usually used to report progress in the user interface using a progress bar) and partial
/// result progress to support streaming of results.
///A progress notification has the following properties:
/// # Notification:
/// * `method`: '$/progress'
/// * `params`: ProgressParams
#[allow(dead_code)]
pub type ProgressNotification<T> = NotificationMessage<ProgressParams<T>>;

#[allow(dead_code)]
pub type DocumentUri = String;
#[allow(dead_code)]
pub type URI = String;

/// Client capabilities specific to regular expressions
#[derive(Debug, Serialize, Deserialize)]
pub struct RegularExpressionsClientCapabilities {
    /// The engine's name
    engine: String,
    /// The engine's version
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>
}

#[allow(dead_code)]
pub const EOL: [&str; 3] = ["\n", "\r\n", "\r"];

/// Position in a text document expressed as zero-based line and zero-based character offset.
/// A position is between two characters like an 'insert' cursor in an editor. Special values like for example -1 to
/// denote the end of a line are not supported.
#[derive(Debug, Serialize, Deserialize)]
pub struct Position {
    /// Line position in a document (zero-based).
    line: Uinteger,
    /// Character offset on a line in a document (zero-based). The meaning of this
	/// offset is determined by the negotiated `PositionEncodingKind`.
	/// If the character value is greater than the line length it defaults back
	/// to the line length.
    character: Uinteger
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
/// A type indicating how positions are encoded, specifically what column offsets mean.
pub enum PositionEncodingKind {
    /// Character offsets count UTF-8 code units.
    #[serde(rename = "utf-8")]
    UTF8,
    /// Character offsets count UTF-16 code units.
    /// 
    /// This is the default and must always be supported by servers.
    #[serde(rename = "utf-16")]
    UTF16,
    /// Character offsets count UTF-32 code units.
    /// 
    /// Implementation note: these are the same as Unicode code points,
    /// so this `PositionEncodingKind` may also be used for an
    /// encoding-agnostic representation of character offsets.
    #[serde(rename = "utf-32")]
    UTF32
}

/// A range in a text document expressed as (zero-based) start and end positions. A range is comparable to a selection in an editor.
/// Therefore the end position is exclusive. If you want to specify a range that contains a line including the
/// line ending character(s) then use an end position denoting the start of the next line. For example:
/// 
/// ```json
/// {
///     start: { line: 5, character: 23 },
///     end : { line: 6, character: 0 }
/// }
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct Range {
    /// The range's start position.
    pub start: Position,
    /// The range's end position.
    pub end: Position
}

/// Information about the client
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientInfo {
    /// The name of the client as defined by the client.
    name: String,
    /// The client's version as defined by the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    version: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    /// The process Id of the parent process that started the server. Is null if the process has not been started by another process.
    /// If the parent process is not alive then the server should exit (see exit notification) its process.
    pub process_id: Option<Integer>,
    /// Information about the client
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub client_info: Option<ClientInfo>,
    /// The locale the client is currently showing the user interface
	/// in. This must not necessarily be the locale of the operating
	/// system.
	///
	/// Uses IETF language tags as the value's syntax
	/// (See https://en.wikipedia.org/wiki/IETF_language_tag)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub locale: Option<String>,
    /// The root path of the workspace. Is null if no folder is open.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[deprecated = "in favour of root_uri"]
    pub root_path: Option<Option<String>>,
    /// The rootUri of the workspace. Is null if no folder is open. If both `root_path` and `root_uri` are set, `root_uri` wins.
    #[deprecated = "in favour of workspace_folders"]
    pub root_uri: Option<DocumentUri>,
    /// User provided initialization options.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub initialization_options: Option<LSPAny>,
    /// The capabilities provided by the client.
    pub capabilities: ClientCapabilities,
    /// The initial trace setting. If omitted trace is disabled (`'off'`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub trace_value: Option<TraceValue>,
    /// The workspace folders configured in the client when the server starts.
	/// This property is only available if the client supports workspace folders.
	/// It can be `null` if the client supports workspace folders but none are
	/// configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub workspace_folders: Option<Option<Vec<WorkspaceFolder>>>,
}

/// Information about the server.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInfo {
    /// The name of the server as defined by the server.
    pub name: String,
    /// The server's version as defined by the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub version: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// Information about the server
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub server_info: Option<ServerInfo>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitializedParams {}

/// The initialize request is sent as the first request from the client to the server. If the server receives a request or notification
/// before the initialize request it should act as follows:
///
/// * For a request the response should be an error with code: -32002. The message can be picked by the server.
/// * Notifications should be dropped, except for the exit notification. This will allow the exit of a server without an initialize request.
///
/// Until the server has responded to the initialize request with an InitializeResult, the client must not send any additional requests or
/// notifications to the server. In addition the server is not allowed to send any requests or notifications to the client until it has
/// responded with an InitializeResult, with the exception that during the initialize request the server is allowed to send the notifications
/// window/showMessage, window/logMessage and telemetry/event as well as the window/showMessageRequest request to the client.
/// In case the client sets up a progress token in the initialize params (e.g. property workDoneToken) the server is also allowed
/// to use that token (and only that token) using the $/progress notification sent from the server to the client.
///
/// The initialize request may only be sent once.
///
/// # Request
///
/// * `method`: 'initialize'
/// * `params`: `InitializeParams`
#[allow(dead_code)]
pub type InitializeRequest = RequestMessage<InitializeParams>;

/// # Response
/// * `result`: `InitializeResult`
#[allow(dead_code)]
pub type InitializeResponse = ResponseMessage<InitializeResult>;

/// The initialized notification is sent from the client to the server after the client received the result of the initialize request but before the client is sending any other request or notification to the server. The server can use the initialized notification for example to dynamically register capabilities. The initialized notification may only be sent once.
///
/// # Notification:
/// * `method`: 'initialized'
///* `params`: `InitializedParams`
#[allow(dead_code)]
pub type InitializedNotification = NotificationMessage<InitializedParams>;

/// `ClientCapabilities` define capabilities for dynamic registration, workspace and text document features the client supports.
/// The experimental can be used to pass experimental capabilities under development.
/// For future compatibility a ClientCapabilities object literal can have more properties set than currently defined.
/// Servers receiving a ClientCapabilities object literal with unknown properties should ignore these properties.
/// A missing property should be interpreted as an absence of the capability. If a missing property normally defines sub properties,
/// all missing sub properties should be interpreted as an absence of the corresponding capability.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub workspace: Option<WorkspaceClientCapabilities>,
}

/// Workspace specific capabilities.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceClientCapabilities {

}

/// A `TraceValue` represents the level of verbosity with which the server systematically reports its execution
/// trace using `$/logTrace` notifications. The initial trace value is set by the client at initialization
/// and can be modified later using the `$/setTrace` notification.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TraceValue {
    Off,
    Messages,
    Verbose
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceFolder {
    /// The associated URI for this workspace folder.
    uri: DocumentUri,
    /// The name of the workspace folder. Used to refer to this workspace folder in the user interface.
    name: String
}