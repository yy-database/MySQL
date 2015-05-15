//! Schema 反射模块
//! 
//! 从数据库读取现有表结构信息。

use std::collections::HashMap;
use crate::{DatabaseBackend, DatabaseConnection, DatabaseResult};

/// 列类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    /// 整数类型
    Integer,
    /// 浮点类型
    Real,
    /// 文本类型
    Text,
    /// 二进制类型
    Blob,
}

impl ColumnType {
    /// 从 SQL 类型字符串解析
    pub fn from_sql(sql: &str) -> Self {
        match sql.to_uppercase().as_str() {
            "INTEGER" | "INT" | "BIGINT" | "SMALLINT" | "TINYINT" | "SERIAL" | "BIGSERIAL" => ColumnType::Integer,
            "REAL" | "FLOAT" | "DOUBLE" | "NUMERIC" | "DECIMAL" | "DOUBLE PRECISION" => ColumnType::Real,
            "TEXT" | "VARCHAR" | "CHAR" | "STRING" | "VARCHAR(255)" | "TEXT[]" => ColumnType::Text,
            "BLOB" | "BINARY" | "BYTEA" | "LONGBLOB" => ColumnType::Blob,
            _ => ColumnType::Text,
        }
    }
}

/// 列定义
#[derive(Debug, Clone)]
pub struct ColumnDef {
    /// 列名
    pub name: String,
    /// 列类型
    pub col_type: ColumnType,
    /// 是否可空
    pub nullable: bool,
    /// 是否主键
    pub primary_key: bool,
    /// 是否自增
    pub auto_increment: bool,
    /// 默认值
    pub default_value: Option<String>,
    /// 是否唯一
    pub unique: bool,
}

/// 索引定义
#[derive(Debug, Clone)]
pub struct IndexDef {
    /// 索引名称
    pub name: String,
    /// 表名
    pub table_name: String,
    /// 列名列表
    pub columns: Vec<String>,
    /// 是否唯一索引
    pub unique: bool,
}

/// 外键引用行为
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferentialAction {
    /// 无操作
    NoAction,
    /// 限制
    Restrict,
    /// 级联
    Cascade,
    /// 设为空
    SetNull,
    /// 设为默认值
    SetDefault,
}

impl ReferentialAction {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "CASCADE" => ReferentialAction::Cascade,
            "RESTRICT" => ReferentialAction::Restrict,
            "SET NULL" => ReferentialAction::SetNull,
            "SET DEFAULT" => ReferentialAction::SetDefault,
            _ => ReferentialAction::NoAction,
        }
    }
}

/// 外键定义
#[derive(Debug, Clone)]
pub struct ForeignKeyDef {
    /// 外键名称
    pub name: String,
    /// 本表列名
    pub column: String,
    /// 引用表名
    pub ref_table: String,
    /// 引用列名
    pub ref_column: String,
    /// 更新行为
    pub on_update: ReferentialAction,
    /// 删除行为
    pub on_delete: ReferentialAction,
}

/// 表结构定义
#[derive(Debug, Clone)]
pub struct TableSchema {
    /// 表名
    pub name: String,
    /// 列定义列表
    pub columns: Vec<ColumnDef>,
    /// 索引定义列表
    pub indexes: Vec<IndexDef>,
    /// 外键约束列表
    pub foreign_keys: Vec<ForeignKeyDef>,
}

/// Schema 反射器
pub struct SchemaReflector<'a> {
    conn: &'a dyn DatabaseConnection,
}

impl<'a> SchemaReflector<'a> {
    /// 创建新的反射器
    pub fn new(conn: &'a dyn DatabaseConnection) -> Self {
        Self { conn }
    }

    /// 获取所有表名
    pub async fn get_table_names(&self) -> DatabaseResult<Vec<String>> {
        match self.conn.backend() {
            DatabaseBackend::Limbo => self.get_table_names_limbo().await,
            DatabaseBackend::Postgres => self.get_table_names_postgres().await,
            DatabaseBackend::MySql => self.get_table_names_mysql().await,
        }
    }

    /// 获取表的完整结构
    pub async fn get_table_schema(&self, table_name: &str) -> DatabaseResult<TableSchema> {
        let columns = self.get_columns(table_name).await?;
        let indexes = self.get_indexes(table_name).await?;
        let foreign_keys = self.get_foreign_keys(table_name).await.unwrap_or_default();

        Ok(TableSchema {
            name: table_name.to_string(),
            columns,
            indexes,
            foreign_keys,
        })
    }

    /// 获取表的所有外键约束
    pub async fn get_foreign_keys(&self, table_name: &str) -> DatabaseResult<Vec<ForeignKeyDef>> {
        match self.conn.backend() {
            DatabaseBackend::Limbo => self.get_foreign_keys_limbo(table_name).await,
            DatabaseBackend::Postgres => self.get_foreign_keys_postgres(table_name).await,
            DatabaseBackend::MySql => self.get_foreign_keys_mysql(table_name).await,
        }
    }

    /// 获取表的所有列
    pub async fn get_columns(&self, table_name: &str) -> DatabaseResult<Vec<ColumnDef>> {
        match self.conn.backend() {
            DatabaseBackend::Limbo => self.get_columns_limbo(table_name).await,
            DatabaseBackend::Postgres => self.get_columns_postgres(table_name).await,
            DatabaseBackend::MySql => self.get_columns_mysql(table_name).await,
        }
    }

    /// 获取表的所有索引
    pub async fn get_indexes(&self, table_name: &str) -> DatabaseResult<Vec<IndexDef>> {
        match self.conn.backend() {
            DatabaseBackend::Limbo => self.get_indexes_limbo(table_name).await,
            DatabaseBackend::Postgres => self.get_indexes_postgres(table_name).await,
            DatabaseBackend::MySql => self.get_indexes_mysql(table_name).await,
        }
    }

    /// 获取所有表的 Schema
    pub async fn get_all_schemas(&self) -> DatabaseResult<HashMap<String, TableSchema>> {
        let table_names = self.get_table_names().await?;
        let mut schemas = HashMap::new();

        for name in table_names {
            let schema = self.get_table_schema(&name).await?;
            schemas.insert(name, schema);
        }

        Ok(schemas)
    }

    /// 检查表是否存在
    pub async fn table_exists(&self, table_name: &str) -> DatabaseResult<bool> {
        match self.conn.backend() {
            DatabaseBackend::Limbo => self.table_exists_limbo(table_name).await,
            DatabaseBackend::Postgres => self.table_exists_postgres(table_name).await,
            DatabaseBackend::MySql => self.table_exists_mysql(table_name).await,
        }
    }

    async fn get_table_names_limbo(&self) -> DatabaseResult<Vec<String>> {
        let sql = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != '_migrations'";
        let mut rows = self.conn.query(sql).await?;

        let mut tables = Vec::new();
        while let Some(row) = rows.next().await? {
            tables.push(row.get_string(0)?);
        }

        Ok(tables)
    }

    async fn get_table_names_postgres(&self) -> DatabaseResult<Vec<String>> {
        let sql = "SELECT tablename FROM pg_tables WHERE schemaname = 'public' AND tablename != '_migrations'";
        let mut rows = self.conn.query(sql).await?;

        let mut tables = Vec::new();
        while let Some(row) = rows.next().await? {
            tables.push(row.get_string(0)?);
        }

        Ok(tables)
    }

    async fn get_table_names_mysql(&self) -> DatabaseResult<Vec<String>> {
        let sql = "SELECT table_name FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name != '_migrations'";
        let mut rows = self.conn.query(sql).await?;

        let mut tables = Vec::new();
        while let Some(row) = rows.next().await? {
            tables.push(row.get_string(0)?);
        }

        Ok(tables)
    }

    async fn get_columns_limbo(&self, table_name: &str) -> DatabaseResult<Vec<ColumnDef>> {
        let sql = format!("PRAGMA table_info({})", table_name);
        let mut rows = self.conn.query(&sql).await?;

        let mut columns = Vec::new();
        while let Some(row) = rows.next().await? {
            let name = row.get_string(1)?;
            let type_str = row.get_string(2)?;
            let not_null = row.get_i64(3)? != 0;
            let default_value = row.get_option_string(4)?;
            let is_pk = row.get_i64(5)? != 0;

            let col_type = ColumnType::from_sql(&type_str);

            columns.push(ColumnDef {
                name,
                col_type,
                nullable: !not_null,
                primary_key: is_pk,
                auto_increment: false,
                default_value,
                unique: false,
            });
        }

        Ok(columns)
    }

    async fn get_columns_postgres(&self, table_name: &str) -> DatabaseResult<Vec<ColumnDef>> {
        let sql = format!(
            "SELECT column_name, data_type, is_nullable, column_default, \
             EXISTS (SELECT 1 FROM information_schema.key_column_usage k \
             JOIN information_schema.table_constraints t ON k.constraint_name = t.constraint_name \
             WHERE k.table_name = '{}' AND k.column_name = c.column_name AND t.constraint_type = 'PRIMARY KEY') AS is_pk, \
             EXISTS (SELECT 1 FROM information_schema.key_column_usage k \
             JOIN information_schema.table_constraints t ON k.constraint_name = t.constraint_name \
             WHERE k.table_name = '{}' AND k.column_name = c.column_name AND t.constraint_type = 'UNIQUE') AS is_unique \
             FROM information_schema.columns c \
             WHERE table_name = '{}'",
            table_name, table_name, table_name
        );
        let mut rows = self.conn.query(&sql).await?;

        let mut columns = Vec::new();
        while let Some(row) = rows.next().await? {
            let name = row.get_string(0)?;
            let type_str = row.get_string(1)?;
            let is_nullable = row.get_string(2)? == "YES";
            let default_value = row.get_option_string(3)?;
            let is_pk = row.get_bool(4)?;
            let is_unique = row.get_bool(5)?;

            let col_type = ColumnType::from_sql(&type_str);

            columns.push(ColumnDef {
                name,
                col_type,
                nullable: is_nullable,
                primary_key: is_pk,
                auto_increment: false,
                default_value,
                unique: is_unique,
            });
        }

        Ok(columns)
    }

    async fn get_columns_mysql(&self, table_name: &str) -> DatabaseResult<Vec<ColumnDef>> {
        let sql = format!(
            "SELECT column_name, data_type, is_nullable, column_default, column_key = 'PRI' AS is_pk, \
             column_key = 'UNI' AS is_unique, extra LIKE '%auto_increment%' AS is_auto_inc \
             FROM information_schema.columns WHERE table_name = '{}' AND table_schema = DATABASE()",
            table_name
        );
        let mut rows = self.conn.query(&sql).await?;

        let mut columns = Vec::new();
        while let Some(row) = rows.next().await? {
            let name = row.get_string(0)?;
            let type_str = row.get_string(1)?;
            let is_nullable = row.get_string(2)? == "YES";
            let default_value = row.get_option_string(3)?;
            let is_pk = row.get_bool(4)?;
            let is_unique = row.get_bool(5)?;
            let is_auto_inc = row.get_bool(6)?;

            let col_type = ColumnType::from_sql(&type_str);

            columns.push(ColumnDef {
                name,
                col_type,
                nullable: is_nullable,
                primary_key: is_pk,
                auto_increment: is_auto_inc,
                default_value,
                unique: is_unique,
            });
        }

        Ok(columns)
    }

    async fn get_indexes_limbo(&self, table_name: &str) -> DatabaseResult<Vec<IndexDef>> {
        let sql = format!("PRAGMA index_list({})", table_name);
        let mut rows = self.conn.query(&sql).await?;

        let mut indexes = Vec::new();
        while let Some(row) = rows.next().await? {
            let index_name = row.get_string(1)?;
            let unique = row.get_i64(2)? != 0;

            let columns = self.get_index_columns_limbo(&index_name).await?;

            indexes.push(IndexDef {
                name: index_name,
                table_name: table_name.to_string(),
                columns,
                unique,
            });
        }

        Ok(indexes)
    }

    async fn get_indexes_postgres(&self, table_name: &str) -> DatabaseResult<Vec<IndexDef>> {
        let sql = format!(
            "SELECT i.relname AS index_name, ix.indisunique AS is_unique, \
             array_agg(a.attname ORDER BY array_position(ix.indkey, a.attnum)) AS columns \
             FROM pg_index ix \
             JOIN pg_class i ON i.oid = ix.indexrelid \
             JOIN pg_class t ON t.oid = ix.indrelid \
             JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey) \
             WHERE t.relname = '{}' \
             GROUP BY i.relname, ix.indisunique",
            table_name
        );
        let mut rows = self.conn.query(&sql).await?;

        let mut indexes = Vec::new();
        while let Some(row) = rows.next().await? {
            let index_name = row.get_string(0)?;
            let unique = row.get_bool(1)?;
            let columns_str = row.get_string(2)?;
            let columns: Vec<String> = columns_str
                .trim_matches(|c| c == '{' || c == '}')
                .split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .collect();

            indexes.push(IndexDef {
                name: index_name,
                table_name: table_name.to_string(),
                columns,
                unique,
            });
        }

        Ok(indexes)
    }

    async fn get_indexes_mysql(&self, table_name: &str) -> DatabaseResult<Vec<IndexDef>> {
        let sql = format!(
            "SELECT index_name, non_unique = 0 AS is_unique, \
             GROUP_CONCAT(column_name ORDER BY seq_in_index SEPARATOR ',') AS columns \
             FROM information_schema.statistics \
             WHERE table_name = '{}' AND table_schema = DATABASE() \
             GROUP BY index_name, non_unique",
            table_name
        );
        let mut rows = self.conn.query(&sql).await?;

        let mut indexes = Vec::new();
        while let Some(row) = rows.next().await? {
            let index_name = row.get_string(0)?;
            let unique = row.get_bool(1)?;
            let columns_str = row.get_string(2)?;
            let columns: Vec<String> = columns_str.split(',').map(|s| s.trim().to_string()).collect();

            indexes.push(IndexDef {
                name: index_name,
                table_name: table_name.to_string(),
                columns,
                unique,
            });
        }

        Ok(indexes)
    }

    async fn get_foreign_keys_limbo(&self, table_name: &str) -> DatabaseResult<Vec<ForeignKeyDef>> {
        let sql = format!("PRAGMA foreign_key_list({})", table_name);
        let mut rows = self.conn.query(&sql).await?;

        let mut foreign_keys = Vec::new();
        while let Some(row) = rows.next().await? {
            let id = row.get_i64(0)?;
            let seq = row.get_i64(1)?;
            if seq != 0 {
                continue;
            }
            let ref_table = row.get_string(2)?;
            let column = row.get_string(3)?;
            let ref_column = row.get_string(4)?;
            let on_update_str = row.get_string(5)?;
            let on_delete_str = row.get_string(6)?;

            let on_update = ReferentialAction::from_str(&on_update_str);
            let on_delete = ReferentialAction::from_str(&on_delete_str);

            let fk_name = format!("fk_{}_{}", table_name, column);

            foreign_keys.push(ForeignKeyDef {
                name: fk_name,
                column,
                ref_table,
                ref_column,
                on_update,
                on_delete,
            });
        }

        Ok(foreign_keys)
    }

    async fn get_foreign_keys_postgres(&self, table_name: &str) -> DatabaseResult<Vec<ForeignKeyDef>> {
        let sql = format!(
            "SELECT tc.constraint_name, kcu.column_name, ccu.table_name AS foreign_table_name, \
             ccu.column_name AS foreign_column_name, rc.update_rule, rc.delete_rule \
             FROM information_schema.table_constraints tc \
             JOIN information_schema.key_column_usage kcu \
             ON tc.constraint_name = kcu.constraint_name \
             JOIN information_schema.constraint_column_usage ccu \
             ON ccu.constraint_name = tc.constraint_name \
             JOIN information_schema.referential_constraints rc \
             ON tc.constraint_name = rc.constraint_name \
             WHERE tc.table_name = '{}' AND tc.constraint_type = 'FOREIGN KEY'",
            table_name
        );
        let mut rows = self.conn.query(&sql).await?;

        let mut foreign_keys = Vec::new();
        while let Some(row) = rows.next().await? {
            let name = row.get_string(0)?;
            let column = row.get_string(1)?;
            let ref_table = row.get_string(2)?;
            let ref_column = row.get_string(3)?;
            let on_update_str = row.get_string(4)?;
            let on_delete_str = row.get_string(5)?;

            let on_update = ReferentialAction::from_str(&on_update_str);
            let on_delete = ReferentialAction::from_str(&on_delete_str);

            foreign_keys.push(ForeignKeyDef {
                name,
                column,
                ref_table,
                ref_column,
                on_update,
                on_delete,
            });
        }

        Ok(foreign_keys)
    }

    async fn get_foreign_keys_mysql(&self, table_name: &str) -> DatabaseResult<Vec<ForeignKeyDef>> {
        let sql = format!(
            "SELECT kcu.constraint_name, kcu.column_name, kcu.referenced_table_name, kcu.referenced_column_name, \
             rc.update_rule, rc.delete_rule \
             FROM information_schema.key_column_usage kcu \
             JOIN information_schema.referential_constraints rc \
             ON kcu.constraint_name = rc.constraint_name \
             WHERE kcu.table_name = '{}' AND kcu.table_schema = DATABASE() AND kcu.referenced_table_name IS NOT NULL",
            table_name
        );
        let mut rows = self.conn.query(&sql).await?;

        let mut foreign_keys = Vec::new();
        while let Some(row) = rows.next().await? {
            let name = row.get_string(0)?;
            let column = row.get_string(1)?;
            let ref_table = row.get_string(2)?;
            let ref_column = row.get_string(3)?;
            let on_update_str = row.get_string(4)?;
            let on_delete_str = row.get_string(5)?;

            let on_update = ReferentialAction::from_str(&on_update_str);
            let on_delete = ReferentialAction::from_str(&on_delete_str);

            foreign_keys.push(ForeignKeyDef {
                name,
                column,
                ref_table,
                ref_column,
                on_update,
                on_delete,
            });
        }

        Ok(foreign_keys)
    }

    async fn table_exists_limbo(&self, table_name: &str) -> DatabaseResult<bool> {
        let sql = format!("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'", table_name);
        let mut rows = self.conn.query(&sql).await?;

        if let Some(row) = rows.next().await? {
            let count = row.get_i64(0)?;
            Ok(count > 0)
        }
        else {
            Ok(false)
        }
    }

    async fn table_exists_postgres(&self, table_name: &str) -> DatabaseResult<bool> {
        let sql = format!("SELECT COUNT(*) FROM pg_tables WHERE schemaname = 'public' AND tablename = '{}'", table_name);
        let mut rows = self.conn.query(&sql).await?;

        if let Some(row) = rows.next().await? {
            let count = row.get_i64(0)?;
            Ok(count > 0)
        }
        else {
            Ok(false)
        }
    }

    async fn table_exists_mysql(&self, table_name: &str) -> DatabaseResult<bool> {
        let sql = format!(
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = '{}'",
            table_name
        );
        let mut rows = self.conn.query(&sql).await?;

        if let Some(row) = rows.next().await? {
            let count = row.get_i64(0)?;
            Ok(count > 0)
        }
        else {
            Ok(false)
        }
    }

    async fn get_index_columns_limbo(&self, index_name: &str) -> DatabaseResult<Vec<String>> {
        let sql = format!("PRAGMA index_info({})", index_name);
        let mut rows = self.conn.query(&sql).await?;

        let mut columns = Vec::new();
        while let Some(row) = rows.next().await? {
            columns.push(row.get_string(2)?);
        }

        Ok(columns)
    }
}