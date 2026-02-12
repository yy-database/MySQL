/// SQLite 表结构信息
#[derive(Debug, Clone)]
pub struct TableSchema {
    pub table_type: String,
    pub name: String,
    pub tbl_name: String,
    pub rootpage: u32,
    pub sql: String,
}

impl TableSchema {
    /// 从 SQL 中解析列名 (极简实现)
    pub fn get_columns(&self) -> Vec<String> {
        if self.sql.is_empty() {
            return vec!["column".to_string()];
        }

        // 寻找括号内的内容: CREATE TABLE x (col1 type, col2 type)
        if let Some(start) = self.sql.find('(')
            && let Some(end) = self.sql.rfind(')')
        {
            let content = &self.sql[start + 1..end];
            return content
                .split(',')
                .map(|s| {
                    s.split_whitespace()
                        .next()
                        .unwrap_or("unknown")
                        .trim_matches(|c| c == '"' || c == '`' || c == '[')
                        .to_string()
                })
                .collect();
        }
        vec!["column".to_string()]
    }
}
