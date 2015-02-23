use yykv_types::{DsError, DsSchema, DsValue};

pub struct SchemaEngine;

impl SchemaEngine {
    /// 验证数据是否符合 Schema 约束
    pub fn validate(schema: &DsSchema, value: &DsValue) -> Result<(), DsError> {
        schema.validate(value)
    }

    /// 尝试将数据转换为目标 Schema（读时转换基础）
    pub fn transform(value: DsValue, target_schema: &DsSchema) -> DsValue {
        match value {
            DsValue::Object {
                schema_id: _,
                mut fields,
            } => {
                let mut new_fields = std::collections::BTreeMap::new();

                // 遍历目标 Schema 的字段
                for (tag, constraint) in &target_schema.fields {
                    if let Some(val) = fields.remove(tag) {
                        // 字段存在，目前直接保留，后续可以加入类型转换逻辑
                        new_fields.insert(*tag, val);
                    } else if constraint.nullable {
                        // 字段缺失但允许为空，填充 Null
                        new_fields.insert(*tag, DsValue::Null);
                    } else if let Some(default) = &constraint.default_value {
                        // 字段缺失但有默认值
                        new_fields.insert(*tag, default.clone());
                    }
                }

                DsValue::Object {
                    schema_id: target_schema.id,
                    fields: new_fields,
                }
            }
            // 非对象类型暂不支持转换，直接返回原值
            _ => value,
        }
    }
}
