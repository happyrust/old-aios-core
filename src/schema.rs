use serde::Serialize;

pub fn generate_basic_versioned_schema<T: Serialize + Default>() -> String {
    let json_representation = serde_json::to_string(&T::default()).unwrap();

    // 这里, 我们简单地假设每个 JSON 键都对应一个 TerminusDB 类型
    // 在实际应用中, 你可能需要进行更复杂的映射或转换
    let schema: serde_json::Value = serde_json::from_str(&json_representation).unwrap();

    let mut terminusdb_schemas = Vec::new();

    for (key, value) in schema.as_object().unwrap() {
        let datatype = match value {
            serde_json::Value::String(_) => "xsd:string",
            serde_json::Value::Bool(_) => "xsd:boolean",
            serde_json::Value::Number(n) => {
                if n.is_f64() {
                    "xsd:decimal"
                } else {
                    "xsd:integer"
                }
            } // 假设所有数字都是整数
            // 你可以根据需要增加更多的类型映射
            _ => "xsd:any",
        };
        if datatype == "xsd:any" {
            continue;
        }

        terminusdb_schemas.push(format!(r#""{}": "{}""#, key, datatype));
    }

    terminusdb_schemas.join(",")
}
