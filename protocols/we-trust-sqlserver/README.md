# SQL Server Compatibility Protocol

**we-trust-sqlserver** implements the basic parts of the Tabular Data Stream (TDS) protocol, providing support for traditional .NET applications and enterprise-grade BI tools.

## 1. Protocol Mapping

- **TDS Packet Parsing**: Identifies `Batch Request` and `RPC Request` sent by SQL Server clients.
- **Type Mapping**: Maps SQL Server types such as `money`, `datetime2`, and `uniqueidentifier` to `YYValue`.

## 2. Use Cases

- **Legacy System Migration**: Switch the backend database to high-concurrency YYKV without rewriting large amounts of C# code, simply by changing the connection string.

## Connection Example

```csharp
// Connection using C# SqlClient
string connectionString = "Server=127.0.0.1,1433;Database=yykv;User Id=sa;Password=your_password;";
using (SqlConnection connection = new SqlConnection(connectionString)) {
    connection.Open();
    // Execute query...
}
```
