use super::RefnoEnum;
use crate::RefU64;
use crate::pdms_types::{EleOperation, PdmsElement};
use bevy_ecs::resource::Resource;
use serde::{Deserialize, Serialize};
use serde_json::{json, to_string_pretty};
use serde_with::DisplayFromStr;
use std::fmt::format;
use surrealdb::sql::Thing;

#[derive(Serialize, Deserialize, Clone, Debug, Resource, Default, PartialEq)]
pub struct SPdmsElement {
    //指向具体的类型
    pub refno: RefnoEnum,
    pub owner: RefnoEnum,
    pub name: String,
    pub noun: String,
    pub dbnum: i32,
    ///小版本号
    pub sesno: i32,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<String>,
    pub cata_hash: String,
    ///锁定模型
    pub lock: bool,
    //todo 可以改为使用 op 来表达是否删除
    pub deleted: bool,
    #[serde(default)]
    pub op: EleOperation,
}

impl SPdmsElement {
    #[inline]
    pub fn refno(&self) -> RefU64 {
        self.refno.refno()
    }

    #[inline]
    pub fn history_id(&self) -> String {
        format!("pe:{}_{}", self.refno(), self.sesno)
    }

    pub fn gen_sur_json(&self, id: Option<String>) -> String {
        let mut json_string = to_string_pretty(&json!({
            "name": self.name,
            "noun": self.noun,
            "dbnum": self.dbnum,
            "sesno": self.sesno,
            "status_code": self.status_code,
            "cata_hash": self.cata_hash,
            "lock": self.lock,
            "deleted": self.deleted,
        }))
        .unwrap();
        json_string.remove(json_string.len() - 1);
        json_string.push_str(",");
        json_string.push_str(&format!(
            r#""refno": {},"#,
            self.refno().to_table_key(&self.noun)
        ));
        if let Some(id) = id {
            json_string.push_str(&format!(r#""id": {},"#, id));
        }
        json_string.push_str(&format!(r#""owner": {}"#, self.owner.to_pe_key()));
        json_string.push_str("}");
        json_string
    }

    //owner 的 sesno 也需要指定，不然不知道指向的是哪一个
    pub fn gen_sur_json_with_sesno(&self, sesno: i32, owner_sesno: i32) -> String {
        let mut json_string = to_string_pretty(&json!({
            "name": self.name,
            "noun": self.noun,
            "dbnum": self.dbnum,
            "sesno": self.sesno,
            "status_code": self.status_code,
            "cata_hash": self.cata_hash,
            "lock": self.lock,
            "deleted": self.deleted,
        }))
        .unwrap();
        json_string.remove(json_string.len() - 1);
        json_string.push_str(",");
        json_string.push_str(&format!(
            r#""refno": {}_H:['{}',{}], "#,
            &self.noun,
            self.refno(),
            sesno
        ));
        json_string.push_str(&format!(r#""id": ['{}',{}],"#, self.refno(), sesno));
        if owner_sesno != 0 {
            json_string.push_str(&format!(
                r#""owner": pe:['{}',{}]"#,
                self.owner.refno(),
                owner_sesno
            ));
        } else {
            json_string.push_str(&format!(r#""owner": {}"#, self.owner.to_pe_key()));
        }
        json_string.push_str("}");
        json_string
    }

    #[inline]
    pub fn get_type_str(&self) -> &str {
        return self.noun.as_str();
    }

    #[inline]
    pub fn get_owner(&self) -> RefnoEnum {
        return self.owner;
    }
}
