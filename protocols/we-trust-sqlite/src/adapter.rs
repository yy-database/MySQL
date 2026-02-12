use crate::connection::SqliteConnection;
use crate::executor::SqliteExecutor;
use crate::reader::NativeReader;
use crate::writer::NativeWriter;
use std::sync::Arc;
use yykv_types::DsError;

type Result<T> = std::result::Result<T, DsError>;

pub struct SqliteAdapter;

impl SqliteAdapter {
    pub async fn connect(path: &str) -> Result<SqliteConnection> {
        let reader: Arc<NativeReader> = Arc::new(NativeReader::new(path));
        let writer: Arc<NativeWriter> = Arc::new(NativeWriter::new(path));
        let executor = Arc::new(SqliteExecutor::new(reader.clone(), writer.clone()));

        Ok(SqliteConnection::new(reader, writer, executor))
    }
}
