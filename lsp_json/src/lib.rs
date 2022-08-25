#[cfg(test)]
mod tests {
}


use std::{collections::HashMap, fmt::Display};

use serde::{Serialize, Deserialize, de::Visitor};
use serde_json::{from_value, to_value};
use serde_repr::{Serialize_repr, Deserialize_repr};

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

// Not a very rusty way, I think, but it's simple and effective

pub trait FromSerialized<T: Serialize>
    where Self: for<'de> Deserialize<'de> {
    fn from_serialized(value: &T) -> Self {
        from_value(to_value(value).unwrap()).unwrap()
    }
}

impl<T: Serialize> FromSerialized<T> for LSPAny {}

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

impl Display for IntegerOrString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegerOrString::Integer(i) => write!(f, "{i}"),
            IntegerOrString::String(s) => write!(f, "\"{s}\"")
        }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default = "Option::default")]
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
    #[serde(default = "Option::default")]
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

pub type DocumentUri = String;
pub type URI = String;

/// A notification message. A processed notification message must not send a response back. They work like events.
#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationMessage<T> {
    /// jsonrpc version. LSP uses 2.0
    pub jsonrpc: String,
    /// The method to be invoked.
    pub method: String,
    /// The notification's params
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub params: Option<T>
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
pub type ProgressNotification<T> = NotificationMessage<ProgressParams<T>>;

/// Client capabilities specific to regular expressions
#[derive(Debug, Serialize, Deserialize)]
pub struct RegularExpressionsClientCapabilities {
    /// The engine's name
    engine: String,
    /// The engine's version
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>
}

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
    pub trace: Option<TraceValue>,
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
    /// The capabilities the language server provides.
    pub capabilities: ServerCapabilities,
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
pub type InitializeRequest = RequestMessage<InitializeParams>;

/// # Response
/// * `result`: `InitializeResult`
pub type InitializeResponse = ResponseMessage<InitializeResult>;

/// The initialized notification is sent from the client to the server after the client received the result of the initialize
/// request but before the client is sending any other request or notification to the server. The server can use the initialized
/// notification for example to dynamically register capabilities. The initialized notification may only be sent once.
///
/// # Notification:
/// * `method`: 'initialized'
///* `params`: `InitializedParams`
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
#[serde(rename_all = "camelCase")]
pub struct WorkspaceClientCapabilities {
    /// The client has support for file requests/notifications.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub file_operations: Option<FileOperationsCapabilities>,
    /// Capabilities specific to the `workspace/didChangeWatchedFiles1 notification.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub did_change_watched_files: Option<DidChangeWatchedFilesClientCapabilities>,
}

/// A `TraceValue` represents the level of verbosity with which the server systematically reports its execution
/// trace using `$/logTrace` notifications. The initial trace value is set by the client at initialization
/// and can be modified later using the `$/setTrace` notification.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum TraceValue {
    Off,
    Messages,
    Verbose
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceFolder {
    /// The associated URI for this workspace folder.
    pub uri: DocumentUri,
    /// The name of the workspace folder. Used to refer to this workspace folder in the user interface.
    pub name: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetTraceParams {
    /// The new value that should be assigned to the trace setting.
    pub value: TraceValue
}

/// A notification that should be used by the client to modify the trace setting of the server.
///
/// # Notification
///
/// * `method`: `'$/setTrace'`
/// * `params`: `SetTraceParams`
pub type SetTraceNotification = NotificationMessage<SetTraceParams>;

#[derive(Debug, Serialize, Deserialize)]
pub struct LogTraceParams {
    /// The message to be logged.
    pub message: String,
    /// Additional information that can be computed if the `trace` configuration is set to `'verbose'`.
    #[serde(skip_serializing_if = "Option::is_none  ")]
    pub verbose: Option<String>
}

/// A notification to log the trace of the server’s execution. The amount and content of these notifications depends
/// on the current trace configuration. If trace is 'off', the server should not send any logTrace notification. If trace is 'messages',
/// the server should not add the 'verbose' field in the LogTraceParams.
///
/// `'$/logTrace'` should be used for systematic trace reporting. For single debugging messages, the server should send window/logMessage notifications.
///
/// # Notification
///
/// * `method`: `'$/logTrace'`
/// * `params`: `LogTraceParams`
pub type LogTraceNotification = NotificationMessage<LogTraceParams>;

/// The server can signal the following capabilities
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    /// Workspace specific server capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub workspace: Option<WorkspaceServerCapabilities>
}

pub type ShutdownParams = ();
pub type ShutdownResult = ();

/// The `shutdown` request is sent from the client to the server. It asks the server to shut down, but to not exit
/// (otherwise the response might not be delivered correctly to the client). There is a separate `exit` notification
/// that asks the server to exit. Clients must not send any notifications other than exit or requests to a server
/// to which they have sent a `shutdown` request. Clients should also wait with sending the `exit` notification until
/// they have received a response from the `shutdown` request.
///
/// If a server receives requests after a shutdown request those requests should error with `InvalidRequest`.
///
/// # Request
/// * `method`: `'shutdown'`
/// * `params`: `void`
pub type ShutdownRequest = RequestMessage<()>;

/// # Response
/// * `result`: `null`
/// * `error`: code and message set in case an exception happens during shutdown request.
pub type ShutdownResponse = ResponseMessage<()>;

/// The client has support for file requests/notifications.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileOperationsCapabilities {
    /// The client has support for sending didCreateFiles notifications.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub did_create: Option<bool>,
    /// The client has support for sending didRenameFiles notifications.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub did_rename: Option<bool>,
    /// The client has support for sending didDeleteFiles notifications.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub did_delete: Option<bool>,
}

/// The server is interested in file requests/notifications.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileOperationsOptions {
    /// The server is interested in sending didCreateFiles notifications.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub did_create: Option<FileOperationRegistrationOptions>,
    /// The server is interested in sending willCreateFiles requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub will_create: Option<FileOperationRegistrationOptions>,
    /// The server is interested in sending didRenameFiles notifications.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub did_rename: Option<FileOperationRegistrationOptions>,
    /// The server is interested in sending willRenameFiles requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub will_rename: Option<FileOperationRegistrationOptions>,
    /// The server is interested in sending didDeleteFiles notifications.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub did_delete: Option<FileOperationRegistrationOptions>,
    /// The server is interested in sending willDeleteFiles requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub will_delete: Option<FileOperationRegistrationOptions>
}

/// The options to register for file operations.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileOperationRegistrationOptions {
    /// The actual filters
    pub filters: Vec<FileOperationFilter>
}

/// A filter to describe in which file operation requests or notifications the server is interested in.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileOperationFilter {
    /// A Uri like `file` or `untitled`
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub scheme: Option<String>,
    /// The actual file operation pattern.
    pub pattern: FileOperationPattern
}

/// A pattern to describe in which file operation requests or notifications the server is interested in.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileOperationPattern {
    /// The glob pattern to match. Glob patterns can have the following syntax:
	/// - `*` to match one or more characters in a path segment
	/// - `?` to match on one character in a path segment
	/// - `**` to match any number of path segments, including none
	/// - `{}` to group sub patterns into an OR expression. (e.g. `**​/*.{ts,js}`
	///   matches all TypeScript and JavaScript files)
	/// - `[]` to declare a range of characters to match in a path segment
	///   (e.g., `example.[0-9]` to match on `example.0`, `example.1`, …)
	/// - `[!...]` to negate a range of characters to match in a path segment
	///   (e.g., `example.[!0-9]` to match on `example.a`, `example.b`, but
	///   not `example.0`)
    pub glob: String,
    /// Wether to match files or folders with this pattern.
    /// 
    /// Matches both if undefined.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub matches: Option<FileOperationPatternKind>,
    /// Additional options used during matching.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub options: Option<FileOperationPatternOptions>
}

/// Matching options for file operation pattern.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileOperationPatternOptions {
    /// The pattern should me matched ignoring casing.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub ignore_case: Option<bool>
}

/// A pattern kind describing if a glob pattern matches a file, a folder or both.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileOperationPatternKind {
    /// The pattern matches a file only.
    File,
    /// The pattern matches a folder only.
    Folder
}

/// Workspace specific server capabilities.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceServerCapabilities {
    /// The server supports workspace folders.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub workspace_folders: Option<WorkspaceFoldersServerCapabilities>,
    /// The server is interested in file notifications/requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub file_operations: Option<FileOperationsOptions>
}

/// String or boolean
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrBoolean {
    String(String),
    Boolean(bool)
}

/// Workspace folders server capabilities.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFoldersServerCapabilities {
    /// The server has support for workspace folders
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub supported: Option<bool>,
    /// Whether the server wants to receive workspace folder
	/// change notifications.
	///
	/// If a string is provided, the string is treated as an ID
    /// under which the notification is registered on the client
	/// side. The ID can be used to unregister for these events
	/// using the `client/unregisterCapability` request.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub change_notifications: Option<StringOrBoolean>
}

/// The parameters sent in notifications/request for user-initiated creation of files.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateFilesParams {
    /// An array of all files/folders created in this operation.
    pub files: Vec<FileCreate>
}

/// Represents information on a file/folder create.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileCreate {
    /// A file:// URI for the location of the file/folder being created.
    pub uri: String
}

/// The did create files notification is sent from the client to the server when files were created from within the client.
/// 
/// # Notification
/// * `method`: `'workspace/didCreateFiles'`
/// * `params`: `CreateFilesParams` 
pub type DidCreateFilesNotification = NotificationMessage<CreateFilesParams>;

/// The parameters sent in notifications/request for user-initiated renames of files.
#[derive(Debug, Serialize, Deserialize)]
pub struct RenameFilesParams {
    /// An array of all files/folders renamed in this operation. When a folder is renamed, only the folder
    /// will be included, and not its children,
    pub files: Vec<FileRename>
}

/// Represents information on a file/folder rename.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRename {
    /// A file:// URI for the original location of the file/folder being rename.
    pub old_uri: String,
    /// A file:// URI for the new location of the file/folder being rename.
    pub new_uri: String
}

/// The did rename files notification is sent from the client to the server when files were renamed from within the client.
/// 
/// # Notification
/// * `method`: `'workspace/didRenameFiles'`
/// * `params`: `RenameFilesParams` 
pub type DidRenameFilesNotification = NotificationMessage<RenameFilesParams>;

/// The parameters sent in notifications/request for user-initiated deletion of files.
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteFilesParams {
    /// An array of all files/folders deleted in this operation.
    pub files: Vec<FileDelete>
}

/// Represents information on a file/folder delete.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileDelete {
    /// A file:// URI for the location of the file/folder being deleted.
    pub uri: String
}

/// The did delete files notification is sent from the client to the server when files were deleted from within the client.
/// 
/// # Notification
/// * `method`: `'workspace/didDeleteFiles'`
/// * `params`: `DeleteFilesParams` 
pub type DidDeleteFilesNotification = NotificationMessage<DeleteFilesParams>;

/// The will rename files request is sent from the client to the server before files are actually renamed as
/// long as the rename is triggered from within the client either by a user action or by applying a workspace edit.
/// The request can return a WorkspaceEdit which will be applied to workspace before the files are renamed.
/// Please note that clients might drop results if computing the edit took too long or if a server constantly
/// fails on this request. This is done to keep renames fast and reliable.
/// 
/// # Request:
/// * `method`: `workspace/willRenameFiles`
/// * `params`: `RenameFilesParams`
pub type WillRenameFilesRequest = RequestMessage<RenameFilesParams>;
pub type WillRenameFilesResult = Option<WorkspaceEdit>;

/// A workspace edit represents changes to many resources managed in the workspace. The edit should either provide `changes` or `documentChanges`.
/// If the client can handle versioned document edits and if `documentChanges` are present, the latter are preferred over changes.
///
/// Since version 3.13.0 a workspace edit can contain resource operations (create, delete or rename files and folders) as well.
/// If resource operations are present clients need to execute the operations in the order in which they are provided.
/// So a workspace edit for example can consist of the following two changes: (1) create file `a.txt` and (2) a text
/// document edit which insert text into file `a.txt`. An invalid sequence (e.g. (1) delete file `a.txt` and (2) insert
/// text into file `a.txt`) will cause failure of the operation. How the client recovers from the failure is described by
/// the client capability: `workspace.workspaceEdit.failureHandling`.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceEdit {

}

/// General parameters to register for a capability.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Registration {
    /// The id used to register the request. The id can be used to deregister the request again.
    pub id: String,
    /// The method / capability to register for.
    pub method: String,
    /// Options necessary for registration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub register_options: Option<LSPAny>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistrationParams {
    pub registrations: Vec<Registration>
}

pub type RegistrationResult = ();

/// The client/registerCapability request is sent from the server to the client to register for
/// a new capability on the client side. Not all clients need to support dynamic capability registration.
/// A client opts in via the dynamicRegistration property on the specific client capabilities.
/// A client can even provide dynamic registration for capability A but not for capability B
/// (see TextDocumentClientCapabilities as an example).
///
/// Server must not register the same capability both statically through the initialize result and
/// \dynamically for the same document selector. If a server wants to support both static and dynamic
/// registration it needs to check the client capability in the initialize request and only register the
/// capability statically if the client doesn’t support dynamic registration for that capability.
///
/// # Request:
/// * `method`: `client/registerCapability`
/// * `params`: `RegistrationParams`
pub type RegisterCapabilityRequest = RequestMessage<RegistrationParams>;

/// # Response
/// * `result`: `void`
/// * `error`: code and message set in case an exception happens during the request.
pub type RegisterCapabilityResponse = ResponseMessage<RegistrationResult>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidChangeWatchedFilesClientCapabilities {
    /// Did change watched files notification supports dynamic registration.
    /// Please note that the current protocol doesn't support static
    /// configuration for file changes from the server side.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub dynamic_registration: Option<bool>,
    /// Whether the client has support for relative pattern or not.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub relative_pattern_support: Option<bool>
}

/// Describe options to be used when registering for file system change events.
#[derive(Debug, Serialize, Deserialize)]
pub struct DidChangeWatchedFilesRegistrationOptions {
    /// The watchers to register.
    pub watchers: Vec<FileSystemWatcher>
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSystemWatcher {
    /// The glob pattern to watch.
    pub glob_pattern: GlobPattern,
    /// The kind of events of interest. If omitted it defaults to watch_kind.CREATE | watch_kind.CHANGE | watch_kind.DELETE which is 7.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub kind: Option<WatchKind>
}

/// The glob pattern Either a string pattern or a relative pattern.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GlobPattern {
    /// The glob pattern to watch relative to the base path. Glob patterns can have
    /// the following syntax:
    /// - `*` to match one or more characters in a path segment
    /// - `?` to match on one character in a path segment
    /// - `**` to match any number of path segments, including none
    /// - `{}` to group conditions (e.g. `**​/*.{ts,js}` matches all TypeScript
    ///   and JavaScript files)
    /// - `[]` to declare a range of characters to match in a path segment
    ///   (e.g., `example.[0-9]` to match on `example.0`, `example.1`, …)
    /// - `[!...]` to negate a range of characters to match in a path segment
    ///   (e.g., `example.[!0-9]` to match on `example.a`, `example.b`,
    ///   but not `example.0`)
    Pattern(String),
    /// A relative pattern is a helper to construct glob patterns that are matched
    /// relatively to a base URI. The common value for a `baseUri` is a workspace
    /// folder root, but it can be another absolute URI as well.
    #[serde(rename_all = "camelCase")]
    RelativePattern {
        /// A workspace folder or a base URI to which this pattern will be matched against relatively.
        base_uri: WorkspaceFolderOrURI,
        /// The actual glob pattern
        pattern: String
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorkspaceFolderOrURI {
    URI(URI),
    WorkspaceFolder(WorkspaceFolder)
}

pub type WatchKind = u8;

pub mod watch_kind {
    /// Interested in create events.
    pub const CREATE: u8 = 1;
    /// Interested in change events.
    pub const CHANGE: u8 = 2;
    /// Interested in delete events.
    pub const DELETE: u8 = 4;
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DidChangeWatchedFilesParams {
    /// The actual file events.
    pub changes: Vec<FileEvent>
}

/// An event describing a file change.
#[derive(Debug, Deserialize, Serialize)]
pub struct FileEvent {
    /// The file's URI
    pub uri: DocumentUri,
    /// The change type
    #[serde(rename = "type")]
    pub change_type: FileChangeType
}

/// The file event type
#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum FileChangeType {
    /// The file got created.
    Created = 1,
    /// The file got changed.
    Changed = 2,
    /// The file got deleted.
    Deleted = 3
}

///The watched files notification is sent from the client to the server when the client detects changes to
/// files and folders watched by the language client (note although the name suggest that only file events
/// are sent it is about file system events which include folders as well). It is recommended that servers
/// register for these file system events using the registration mechanism. In former implementations clients
/// pushed file events without the server actively asking for it.
pub type DidChangeWatchedFilesNotification = NotificationMessage<DidChangeWatchedFilesParams>;

#[derive(Debug, Serialize, Deserialize)]
pub struct DidChangeWorkspaceFoldersParams {
    /// The actual workspace folder change event.
    pub event: WorkspaceFoldersChangeEvent
}

/// The workspace folder change event.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceFoldersChangeEvent {
    /// The array of added workspace folders.
    pub added: Vec<WorkspaceFolder>,
    /// The array of removed workspace folders.
    pub removed: Vec<WorkspaceFolder>
}

/// The `workspace/didChangeWorkspaceFolders` notification is sent from the client to the server to inform the server
/// about workspace folder configuration changes. The notification is sent by default if both client capability
/// `workspace.workspaceFolders` and the server capability `workspace.workspaceFolders.supported` are true;
/// or if the server has registered itself to receive this notification. To register for the `workspace/didChangeWorkspaceFolders`
/// send a `client/registerCapability` request from the server to the client. The registration parameter must have a registrations
/// item of the following form, where id is a unique id used to unregister the capability (the example uses a UUID):
///
/// ```json
/// {
///     "id": "28c6150c-bd7b-11e7-abc4-cec278b6b50a",
///	    "method": "workspace/didChangeWorkspaceFolders"
/// }
/// ```
///
/// # Notification:
/// * `method`: `workspace/didChangeWorkspaceFolders`
/// * `params`: [`DidChangeWorkspaceFoldersParams`]
pub type DidChangeWorkspaceFoldersNotification = NotificationMessage<DidChangeWorkspaceFoldersParams>;
