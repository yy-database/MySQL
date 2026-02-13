use odbc_sys::*;
use we_trust::{WeTrustClient, ConnectionOptions};
use yyds_types::YYValue;
use tokio::runtime::Runtime;
use std::ptr;
use std::slice;
use std::str::FromStr;

const SQL_NTS: Integer = -3;

// Handles
pub struct Env {
    pub version: i32,
}

pub struct Conn {
    pub runtime: Runtime,
    pub client: Option<WeTrustClient>,
}

pub struct Stmt {
    pub conn: *mut Conn,
    pub results: Vec<Vec<YYValue>>,
    pub current_row: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn SQLAllocHandle(
    handle_type: HandleType,
    input_handle: Handle,
    output_handle: *mut Handle,
) -> SqlReturn {
    match handle_type {
        HandleType::Env => {
            let env = Box::into_raw(Box::new(Env { version: 0 })) as *mut std::ffi::c_void;
            unsafe { *output_handle = Handle(env) };
            SqlReturn::SUCCESS
        }
        HandleType::Dbc => {
            let env_ptr = input_handle.0 as *mut Env;
            if env_ptr.is_null() {
                return SqlReturn::INVALID_HANDLE;
            }
            let runtime = Runtime::new().unwrap();
            let conn = Box::into_raw(Box::new(Conn {
                runtime,
                client: None,
            })) as *mut std::ffi::c_void;
            unsafe { *output_handle = Handle(conn) };
            SqlReturn::SUCCESS
        }
        HandleType::Stmt => {
            let conn_ptr = input_handle.0 as *mut Conn;
            if conn_ptr.is_null() {
                return SqlReturn::INVALID_HANDLE;
            }
            let stmt = Box::into_raw(Box::new(Stmt {
                conn: conn_ptr,
                results: Vec::new(),
                current_row: 0,
            })) as *mut std::ffi::c_void;
            unsafe { *output_handle = Handle(stmt) };
            SqlReturn::SUCCESS
        }
        _ => SqlReturn::ERROR,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SQLFreeHandle(handle_type: HandleType, handle: Handle) -> SqlReturn {
    if handle.0.is_null() {
        return SqlReturn::INVALID_HANDLE;
    }
    match handle_type {
        HandleType::Env => {
            unsafe { drop(Box::from_raw(handle.0 as *mut Env)) };
            SqlReturn::SUCCESS
        }
        HandleType::Dbc => {
            unsafe { drop(Box::from_raw(handle.0 as *mut Conn)) };
            SqlReturn::SUCCESS
        }
        HandleType::Stmt => {
            unsafe { drop(Box::from_raw(handle.0 as *mut Stmt)) };
            SqlReturn::SUCCESS
        }
        _ => SqlReturn::ERROR,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SQLSetEnvAttr(
    _environment_handle: HEnv,
    _attribute: Integer,
    _value_ptr: Pointer,
    _string_length: Integer,
) -> SqlReturn {
    SqlReturn::SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLDriverConnect(
    connection_handle: HDbc,
    _window_handle: Pointer,
    in_connection_string: *const Char,
    string_length1: SmallInt,
    _out_connection_string: *mut Char,
    _buffer_length: SmallInt,
    _string_length2_ptr: *mut SmallInt,
    _driver_completion: USmallInt,
) -> SqlReturn {
    let conn = connection_handle.0 as *mut Conn;
    if conn.is_null() {
        return SqlReturn::INVALID_HANDLE;
    }

    let conn_str = if string_length1 as Integer == SQL_NTS {
        unsafe { std::ffi::CStr::from_ptr(in_connection_string as *const i8).to_string_lossy() }
    } else {
        let s = unsafe { slice::from_raw_parts(in_connection_string as *const u8, string_length1 as usize) };
        String::from_utf8_lossy(s)
    };

    let conn_obj = unsafe { &mut *conn };
    let options = match ConnectionOptions::from_str(&conn_str) {
        Ok(opts) => opts,
        Err(_) => return SqlReturn::ERROR,
    };

    let result = conn_obj.runtime.block_on(async {
        WeTrustClient::connect(options.addr, options.tenant_id, options.secret_key).await
    });

    match result {
        Ok(client) => {
            conn_obj.client = Some(client);
            SqlReturn::SUCCESS
        }
        Err(_) => SqlReturn::ERROR,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLExecDirect(
    statement_handle: HStmt,
    statement_text: *const Char,
    text_length: Integer,
) -> SqlReturn {
    let stmt = statement_handle.0 as *mut Stmt;
    if stmt.is_null() {
        return SqlReturn::INVALID_HANDLE;
    }
    let stmt_obj = unsafe { &mut *stmt };
    let conn_obj = unsafe { &mut *stmt_obj.conn };

    let query = if text_length == SQL_NTS {
        unsafe { std::ffi::CStr::from_ptr(statement_text as *const i8).to_string_lossy() }
    } else {
        let s = unsafe { slice::from_raw_parts(statement_text as *const u8, text_length as usize) };
        String::from_utf8_lossy(s)
    };

    if let Some(ref mut client) = conn_obj.client {
        let result: Result<Vec<Vec<YYValue>>, yyds_types::DsError> = conn_obj.runtime.block_on(async {
            client.send_query(query.as_ref()).await
        });

        match result {
            Ok(rows) => {
                stmt_obj.results = rows;
                stmt_obj.current_row = 0;
                SqlReturn::SUCCESS
            }
            Err(_) => SqlReturn::ERROR,
        }
    } else {
        SqlReturn::ERROR
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLFetch(statement_handle: HStmt) -> SqlReturn {
    let stmt = statement_handle.0 as *mut Stmt;
    if stmt.is_null() {
        return SqlReturn::INVALID_HANDLE;
    }
    let stmt_obj = unsafe { &mut *stmt };

    if stmt_obj.current_row < stmt_obj.results.len() {
        stmt_obj.current_row += 1;
        SqlReturn::SUCCESS
    } else {
        SqlReturn::NO_DATA
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLGetData(
    statement_handle: HStmt,
    column_number: USmallInt,
    _target_type: SmallInt,
    target_value_ptr: Pointer,
    _buffer_length: Integer,
    str_len_or_ind_ptr: *mut Integer,
) -> SqlReturn {
    let stmt = statement_handle.0 as *mut Stmt;
    if stmt.is_null() {
        return SqlReturn::INVALID_HANDLE;
    }
    let stmt_obj = unsafe { &mut *stmt };

    if stmt_obj.current_row == 0 || stmt_obj.current_row > stmt_obj.results.len() {
        return SqlReturn::ERROR;
    }

    let row = &stmt_obj.results[stmt_obj.current_row - 1];
    let col_idx = (column_number - 1) as usize;

    if col_idx >= row.len() {
        return SqlReturn::ERROR;
    }

    let val = &row[col_idx];
    match val {
        YYValue::Text(s) => {
            let bytes = s.as_bytes();
            let len = bytes.len();
            if !target_value_ptr.is_null() {
                // Buffer length in ODBC includes the null terminator for strings
                let max_copy = if _buffer_length > 0 { _buffer_length as usize } else { 0 };
                if max_copy > 0 {
                    let copy_len = std::cmp::min(len, max_copy - 1);
                    unsafe {
                        ptr::copy_nonoverlapping(bytes.as_ptr(), target_value_ptr as *mut u8, copy_len);
                        *(target_value_ptr as *mut u8).add(copy_len) = 0;
                    }
                }
            }
            if !str_len_or_ind_ptr.is_null() {
                unsafe { *str_len_or_ind_ptr = len as Integer };
            }
            SqlReturn::SUCCESS
        }
        YYValue::Int(i) => {
            if !target_value_ptr.is_null() {
                unsafe { *(target_value_ptr as *mut i64) = *i };
            }
            if !str_len_or_ind_ptr.is_null() {
                unsafe { *str_len_or_ind_ptr = 8 };
            }
            SqlReturn::SUCCESS
        }
        YYValue::Float(f) => {
            if !target_value_ptr.is_null() {
                unsafe { *(target_value_ptr as *mut f64) = *f };
            }
            if !str_len_or_ind_ptr.is_null() {
                unsafe { *str_len_or_ind_ptr = 8 };
            }
            SqlReturn::SUCCESS
        }
        YYValue::Bool(b) => {
            if !target_value_ptr.is_null() {
                unsafe { *(target_value_ptr as *mut u8) = if *b { 1 } else { 0 } };
            }
            if !str_len_or_ind_ptr.is_null() {
                unsafe { *str_len_or_ind_ptr = 1 };
            }
            SqlReturn::SUCCESS
        }
        YYValue::Null => {
            if !str_len_or_ind_ptr.is_null() {
                unsafe { *str_len_or_ind_ptr = -1 }; // SQL_NULL_DATA
            }
            SqlReturn::SUCCESS
        }
        _ => SqlReturn::ERROR,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLNumResultCols(
    statement_handle: HStmt,
    column_count_ptr: *mut SmallInt,
) -> SqlReturn {
    let stmt = statement_handle.0 as *mut Stmt;
    if stmt.is_null() {
        return SqlReturn::INVALID_HANDLE;
    }
    let stmt_obj = unsafe { &mut *stmt };

    if let Some(first_row) = stmt_obj.results.first() {
        unsafe { *column_count_ptr = first_row.len() as SmallInt };
        SqlReturn::SUCCESS
    } else {
        unsafe { *column_count_ptr = 0 };
        SqlReturn::SUCCESS
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLDescribeCol(
    statement_handle: HStmt,
    column_number: USmallInt,
    column_name: *mut Char,
    buffer_length: SmallInt,
    name_length_ptr: *mut SmallInt,
    data_type_ptr: *mut SmallInt,
    column_size_ptr: *mut UInteger,
    decimal_digits_ptr: *mut SmallInt,
    nullable_ptr: *mut SmallInt,
) -> SqlReturn {
    let stmt = statement_handle.0 as *mut Stmt;
    if stmt.is_null() {
        return SqlReturn::INVALID_HANDLE;
    }
    let stmt_obj = unsafe { &mut *stmt };

    if let Some(first_row) = stmt_obj.results.first() {
        let col_idx = (column_number - 1) as usize;
        if col_idx < first_row.len() {
            let name = format!("col{}", column_number);
            let name_bytes = name.as_bytes();
            if !column_name.is_null() {
                let copy_len = std::cmp::min(name_bytes.len(), buffer_length as usize - 1);
                unsafe {
                    ptr::copy_nonoverlapping(name_bytes.as_ptr(), column_name as *mut u8, copy_len);
                    *(column_name as *mut u8).add(copy_len) = 0;
                }
            }
            if !name_length_ptr.is_null() {
                unsafe { *name_length_ptr = name_bytes.len() as SmallInt };
            }
            // Simplified type mapping
            unsafe {
                *data_type_ptr = 12; // SQL_VARCHAR
                *column_size_ptr = 255;
                *decimal_digits_ptr = 0;
                *nullable_ptr = 1; // SQL_NULLABLE
            }
            return SqlReturn::SUCCESS;
        }
    }
    SqlReturn::ERROR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLDisconnect(connection_handle: HDbc) -> SqlReturn {
    let conn = connection_handle.0 as *mut Conn;
    if conn.is_null() {
        return SqlReturn::INVALID_HANDLE;
    }
    let conn_obj = unsafe { &mut *conn };
    conn_obj.client = None;
    SqlReturn::SUCCESS
}
