use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

pub type DsResult<T> = Result<T, DsError>;

/// The primary error type for YYKV.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DsError {
    pub kind: Box<DsErrorKind>,
}

/// Detailed error categories for YYKV.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DsErrorKind {
    // --- I/O & Physical Storage ---
    InputOutput {
        path: Option<PathBuf>,
        kind: String, // From io::ErrorKind
        errno: Option<i32>,
    },
    Storage {
        op: String,
        details: String,
    },
    PageNotFound {
        id: u64,
    },
    PageFull,
    PageSizeMismatch {
        expected: usize,
        actual: usize,
    },
    ChecksumMismatch {
        id: u64,
    },
    InvalidPageType {
        page_type: u8,
    },
    Corrupted {
        reason: String,
    },

    // --- Data & Serialization ---
    Serialization {
        format: String,
        details: String,
    },
    Parse {
        target: String,
        details: String,
    },
    Type {
        expected: String,
        actual: String,
    },
    TypeConversion {
        from: String,
        to: String,
    },

    // --- Logic & Schema ---
    Query {
        sql: String,
        details: String,
    },
    Execution {
        op: String,
        details: String,
    },
    Schema {
        table: Option<String>,
        column: Option<String>,
        details: String,
    },
    TableNotFound {
        name: String,
    },

    // --- Infrastructure & System ---
    Consensus {
        op: String,
        details: String,
    },
    Network {
        addr: Option<String>,
        details: String,
    },
    Protocol {
        details: String,
    },
    Internal {
        details: String,
    },
    Config {
        key: String,
        details: String,
    },
    Http {
        code: u16,
        url: String,
        method: String,
    },
    Join {
        details: String,
    },
    Cache {
        op: String,
        details: String,
    },
    Sync {
        details: String,
    },

    // --- Auth & Resource Management ---
    Permission {
        action: String,
        resource: String,
    },
    Tenant {
        id: String,
        details: String,
    },
    Quota {
        resource: String,
        limit: u64,
        usage: u64,
    },
}

impl DsErrorKind {
    /// Gets the i18n key for this error kind.
    pub fn key(&self) -> &'static str {
        match self {
            DsErrorKind::InputOutput { .. } => "kv.error.io",
            DsErrorKind::Storage { .. } => "kv.error.storage",
            DsErrorKind::PageNotFound { .. } => "kv.error.page_not_found",
            DsErrorKind::PageFull => "kv.error.page_full",
            DsErrorKind::PageSizeMismatch { .. } => "kv.error.page_size_mismatch",
            DsErrorKind::ChecksumMismatch { .. } => "kv.error.checksum_mismatch",
            DsErrorKind::InvalidPageType { .. } => "kv.error.invalid_page_type",
            DsErrorKind::Corrupted { .. } => "kv.error.corrupted",
            DsErrorKind::Serialization { .. } => "kv.error.serialization",
            DsErrorKind::Parse { .. } => "kv.error.parse",
            DsErrorKind::Type { .. } => "kv.error.type",
            DsErrorKind::TypeConversion { .. } => "kv.error.type_conversion",
            DsErrorKind::Query { .. } => "kv.error.query",
            DsErrorKind::Execution { .. } => "kv.error.execution",
            DsErrorKind::Schema { .. } => "kv.error.schema",
            DsErrorKind::TableNotFound { .. } => "kv.error.table_not_found",
            DsErrorKind::Consensus { .. } => "kv.error.consensus",
            DsErrorKind::Network { .. } => "kv.error.network",
            DsErrorKind::Protocol { .. } => "kv.error.protocol",
            DsErrorKind::Internal { .. } => "kv.error.internal",
            DsErrorKind::Config { .. } => "kv.error.config",
            DsErrorKind::Http { .. } => "kv.error.http",
            DsErrorKind::Join { .. } => "kv.error.join",
            DsErrorKind::Cache { .. } => "kv.error.cache",
            DsErrorKind::Sync { .. } => "kv.error.sync",
            DsErrorKind::Permission { .. } => "kv.error.permission",
            DsErrorKind::Tenant { .. } => "kv.error.tenant",
            DsErrorKind::Quota { .. } => "kv.error.quota",
        }
    }
}

impl Display for DsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl Error for DsError {}

impl Display for DsErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DsErrorKind::InputOutput { path, kind, errno } => {
                let p = path
                    .as_ref()
                    .map(|p| format!(" at {:?}", p))
                    .unwrap_or_default();
                let e = errno.map(|n| format!(" (errno {})", n)).unwrap_or_default();
                write!(f, "I/O error{}: {}{}", p, kind, e)
            }
            DsErrorKind::Storage { op, details } => {
                write!(f, "Storage error ({}): {}", op, details)
            }
            DsErrorKind::PageNotFound { id } => write!(f, "Page not found: {}", id),
            DsErrorKind::PageFull => write!(f, "Page is full"),
            DsErrorKind::PageSizeMismatch { expected, actual } => {
                write!(
                    f,
                    "Page size mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            DsErrorKind::ChecksumMismatch { id } => write!(f, "Checksum mismatch for page {}", id),
            DsErrorKind::InvalidPageType { page_type } => {
                write!(f, "Invalid page type: {}", page_type)
            }
            DsErrorKind::Corrupted { reason } => write!(f, "Corrupted: {}", reason),
            DsErrorKind::Serialization { format, details } => {
                write!(f, "Serialization error ({}): {}", format, details)
            }
            DsErrorKind::Parse { target, details } => {
                write!(f, "Parse error for {}: {}", target, details)
            }
            DsErrorKind::Type { expected, actual } => {
                write!(f, "Type error: expected {}, got {}", expected, actual)
            }
            DsErrorKind::TypeConversion { from, to } => {
                write!(f, "Type conversion error: from {} to {}", from, to)
            }
            DsErrorKind::Query { sql, details } => write!(f, "Query error ({}): {}", sql, details),
            DsErrorKind::Execution { op, details } => {
                write!(f, "Execution error ({}): {}", op, details)
            }
            DsErrorKind::Schema {
                table,
                column,
                details,
            } => {
                let t = table
                    .as_ref()
                    .map(|s| format!("table {}", s))
                    .unwrap_or_default();
                let c = column
                    .as_ref()
                    .map(|s| format!("column {}", s))
                    .unwrap_or_default();
                let sep = if !t.is_empty() && !c.is_empty() {
                    ", "
                } else {
                    ""
                };
                write!(f, "Schema error ({}{}{}): {}", t, sep, c, details)
            }
            DsErrorKind::TableNotFound { name } => write!(f, "Table not found: {}", name),
            DsErrorKind::Consensus { op, details } => {
                write!(f, "Consensus error ({}): {}", op, details)
            }
            DsErrorKind::Network { addr, details } => {
                let a = addr
                    .as_ref()
                    .map(|s| format!(" ({})", s))
                    .unwrap_or_default();
                write!(f, "Network error{}: {}", a, details)
            }
            DsErrorKind::Protocol { details } => write!(f, "Protocol error: {}", details),
            DsErrorKind::Internal { details } => write!(f, "Internal error: {}", details),
            DsErrorKind::Config { key, details } => {
                write!(f, "Configuration error for {}: {}", key, details)
            }
            DsErrorKind::Http { code, url, method } => {
                write!(f, "HTTP {} {} {}", code, method, url)
            }
            DsErrorKind::Join { details } => write!(f, "Join error: {}", details),
            DsErrorKind::Cache { op, details } => write!(f, "Cache error ({}): {}", op, details),
            DsErrorKind::Sync { details } => write!(f, "Sync error: {}", details),
            DsErrorKind::Permission { action, resource } => {
                write!(f, "Permission denied for {} on {}", action, resource)
            }
            DsErrorKind::Tenant { id, details } => write!(f, "Tenant error ({}): {}", id, details),
            DsErrorKind::Quota {
                resource,
                limit,
                usage,
            } => {
                write!(
                    f,
                    "Quota exceeded for {}: limit {}, usage {}",
                    resource, limit, usage
                )
            }
        }
    }
}

impl From<DsErrorKind> for DsError {
    fn from(kind: DsErrorKind) -> Self {
        Self::new(kind)
    }
}

impl From<std::io::Error> for DsError {
    fn from(e: std::io::Error) -> Self {
        DsErrorKind::InputOutput {
            path: None,
            kind: format!("{:?}", e.kind()),
            errno: e.raw_os_error(),
        }
        .into()
    }
}


impl From<std::num::ParseIntError> for DsError {
    fn from(e: std::num::ParseIntError) -> Self {
        DsErrorKind::Parse {
            target: "int".into(),
            details: e.to_string(),
        }
        .into()
    }
}

impl From<std::num::ParseFloatError> for DsError {
    fn from(e: std::num::ParseFloatError) -> Self {
        DsErrorKind::Parse {
            target: "float".into(),
            details: e.to_string(),
        }
        .into()
    }
}

impl From<std::string::FromUtf8Error> for DsError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        DsErrorKind::Serialization {
            format: "utf8".into(),
            details: e.to_string(),
        }
        .into()
    }
}

impl From<std::str::Utf8Error> for DsError {
    fn from(e: std::str::Utf8Error) -> Self {
        DsErrorKind::Serialization {
            format: "utf8".into(),
            details: e.to_string(),
        }
        .into()
    }
}

impl From<std::time::SystemTimeError> for DsError {
    fn from(e: std::time::SystemTimeError) -> Self {
        DsErrorKind::Internal {
            details: e.to_string(),
        }
        .into()
    }
}

impl From<String> for DsError {
    fn from(s: String) -> Self {
        DsErrorKind::Internal { details: s }.into()
    }
}

impl From<&str> for DsError {
    fn from(s: &str) -> Self {
        DsErrorKind::Internal {
            details: s.to_string(),
        }
        .into()
    }
}

impl DsError {
    pub fn new(kind: DsErrorKind) -> Self {
        Self {
            kind: Box::new(kind),
        }
    }

    pub fn io_raw(err: std::io::Error, path: Option<PathBuf>) -> Self {
        Self::new(DsErrorKind::InputOutput {
            path,
            kind: format!("{:?}", err.kind()),
            errno: err.raw_os_error(),
        })
    }

    pub fn io(details: impl Into<String>) -> Self {
        Self::io_str(details)
    }

    pub fn io_str(details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::InputOutput {
            path: None,
            kind: details.into(),
            errno: None,
        })
    }

    pub fn storage(details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Storage {
            op: "".into(),
            details: details.into(),
        })
    }

    pub fn storage_with_op(op: impl Into<String>, details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Storage {
            op: op.into(),
            details: details.into(),
        })
    }

    pub fn corrupted(reason: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Corrupted {
            reason: reason.into(),
        })
    }

    pub fn page_full() -> Self {
        Self::new(DsErrorKind::PageFull)
    }

    pub fn page_not_found(id: u64) -> Self {
        Self::new(DsErrorKind::PageNotFound { id })
    }

    pub fn page_size_mismatch(expected: usize, actual: usize) -> Self {
        Self::new(DsErrorKind::PageSizeMismatch { expected, actual })
    }

    pub fn checksum_mismatch(id: u64) -> Self {
        Self::new(DsErrorKind::ChecksumMismatch { id })
    }

    pub fn invalid_page_type(page_type: u8) -> Self {
        Self::new(DsErrorKind::InvalidPageType { page_type })
    }

    pub fn serialization(details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Serialization {
            format: "".into(),
            details: details.into(),
        })
    }

    pub fn serialization_with_format(
        format: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self::new(DsErrorKind::Serialization {
            format: format.into(),
            details: details.into(),
        })
    }

    pub fn parse(details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Parse {
            target: "".into(),
            details: details.into(),
        })
    }

    pub fn parse_with_target(target: impl Into<String>, details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Parse {
            target: target.into(),
            details: details.into(),
        })
    }

    pub fn type_err(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Type {
            expected: expected.into(),
            actual: actual.into(),
        })
    }

    pub fn type_conversion(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self::new(DsErrorKind::TypeConversion {
            from: from.into(),
            to: to.into(),
        })
    }

    pub fn query(details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Query {
            sql: "".into(),
            details: details.into(),
        })
    }

    pub fn query_with_sql(sql: impl Into<String>, details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Query {
            sql: sql.into(),
            details: details.into(),
        })
    }

    pub fn execution(details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Execution {
            op: "".into(),
            details: details.into(),
        })
    }

    pub fn execution_with_op(op: impl Into<String>, details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Execution {
            op: op.into(),
            details: details.into(),
        })
    }

    pub fn schema(
        table: Option<String>,
        column: Option<String>,
        details: impl Into<String>,
    ) -> Self {
        Self::new(DsErrorKind::Schema {
            table,
            column,
            details: details.into(),
        })
    }

    pub fn table_not_found(table: impl Into<String>) -> Self {
        Self::new(DsErrorKind::TableNotFound { name: table.into() })
    }

    pub fn internal(details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Internal {
            details: details.into(),
        })
    }

    pub fn protocol(details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Protocol {
            details: details.into(),
        })
    }

    pub fn network(addr: Option<String>, details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Network {
            addr,
            details: details.into(),
        })
    }

    pub fn consensus(op: impl Into<String>, details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Consensus {
            op: op.into(),
            details: details.into(),
        })
    }

    pub fn config(key: impl Into<String>, details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Config {
            key: key.into(),
            details: details.into(),
        })
    }

    pub fn http(code: u16, url: impl Into<String>, method: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Http {
            code,
            url: url.into(),
            method: method.into(),
        })
    }

    pub fn join(details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Join {
            details: details.into(),
        })
    }

    pub fn cache(op: impl Into<String>, details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Cache {
            op: op.into(),
            details: details.into(),
        })
    }

    pub fn sync(details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Sync {
            details: details.into(),
        })
    }

    pub fn tenant(id: impl Into<String>, details: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Tenant {
            id: id.into(),
            details: details.into(),
        })
    }

    pub fn quota(resource: impl Into<String>, limit: u64, usage: u64) -> Self {
        Self::new(DsErrorKind::Quota {
            resource: resource.into(),
            limit,
            usage,
        })
    }

    pub fn permission(action: impl Into<String>, resource: impl Into<String>) -> Self {
        Self::new(DsErrorKind::Permission {
            action: action.into(),
            resource: resource.into(),
        })
    }
}
