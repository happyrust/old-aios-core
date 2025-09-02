use crate::{
    aios_db_mgr::{aios_mgr::AiosDBMgr, PdmsDataInterface},
    geometry::EleGeosInfo,
    get_pe, init_test_surreal,
    pe::SPdmsElement,
    query_deep_children_refnos, NamedAttrMap, NamedAttrValue, RefU64, RefnoEnum, SUL_DB,
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::collections::{btree_map::BTreeMap, HashSet};
use std::io::Write;
use surrealdb::sql::{Datetime, Thing};

pub async fn export_surreal_data(
    refno: RefU64,
    aios_mgr: &AiosDBMgr,
) -> anyhow::Result<(Vec<String>, HashSet<String>)> {
    let children = query_deep_children_refnos(refno.into()).await?;
    let mut sqls = Vec::new();
    let mut meshes = HashSet::new();
    // 递归收集属性中的引用，并导出其树节点和属性（排除 id/refno/owner 的引用）
    let mut visited: HashSet<RefU64> = HashSet::new();
    visited.insert(refno);
    // 递归收集属性中的引用，并导出其树节点和属性（排除 id/refno/owner 的引用）
    // 使用显式栈避免 async 递归
    let mut queue: Vec<RefU64> = Vec::new();
    // 收集属性中的外键引用，并递归导出对应节点
    for refno in children {
        // 导出
        if let Some(pe) = get_pe(refno).await? {
            let refno: RefU64 = refno.into();
            // 树节点
            let tree_sql = export_tree_node(&pe);
            sqls.push(tree_sql);
            // relate
            let relate = OwnerRelate::query_owner_relations_by_refno(refno.into()).await?;
            if !relate.is_empty() {
                let relate_sql = export_owner_relate(relate[0].clone());
                sqls.push(relate_sql);
            }
            // 属性
            match get_all_attributes(&pe, aios_mgr).await {
                Ok(attr) => {
                    let insert_sql = generate_attr_insert_sql(&pe, &attr);
                    sqls.push(insert_sql);
                    // for r in extract_refnos_from_attributes(&attr) {
                    //     if r.0 != 0 && r.is_valid() && !visited.contains(&r) {
                    //         visited.insert(r);
                    //         queue.push(r);
                    //     }
                    // }
                }
                Err(e) => {
                    dbg!(&e.to_string());
                }
            }
            // 模型
            match get_inst_data(&pe, &mut sqls).await {
                Ok(_) => {}
                Err(e) => {
                    dbg!(&e.to_string());
                }
            }

            // 导出当前元素的 inst_relate 和 geo_relate（如果有的话）
            let inst_relate = InstRelate::query_inst_relations_by_refno(refno.into()).await?;
            for relate in inst_relate {
                // 导出 inst_relate 本体
                let relate_sql = export_inst_relate(relate.clone());
                sqls.push(relate_sql);

                // 导出相关的 aabb 表记录
                if let Some(aabb) = &relate.aabb {
                    if let Some(aabb_row) = AabbRecord::query_by_id(aabb).await? {
                        let aabb_sql = export_aabb_record(&aabb_row);
                        sqls.push(aabb_sql);
                    }
                }

                // 导出相关的 trans 表记录
                if let Some(trans_row) = TransRecord::query_by_id(&relate.world_trans).await? {
                    let trans_sql = export_trans_record(&trans_row);
                    sqls.push(trans_sql);
                }

                // 导出相关的 inst_info 表记录（out）
                if let Some(info_row) = InstInfoRecord::query_by_id(&relate.out).await? {
                    let info_sql = export_inst_info_record(&info_row);
                    sqls.push(info_sql);

                    // 导出geo_relate
                    let geo_relates = GeoRelate::query_by_inst_info_id(&relate.out).await?;
                    for geo_rel in geo_relates {
                        meshes.insert(format!("{}.mesh",geo_rel.out.id.to_string().replace("⟨", "").replace("⟩", "")));
                        // 导出 geo_relate 本体
                        let geo_sql = export_geo_relate(&geo_rel);
                        sqls.push(geo_sql);
                        // 导出关联的 inst_geo 记录
                        if let Some(geo_row) = InstGeoRecord::query_by_id(&geo_rel.out).await? {
                            let geo_sql = export_inst_geo_record(&geo_row);
                            sqls.push(geo_sql);

                            // 导出 inst_geo 中的 aabb 字段（如果存在）
                            if let Some(thing) = geo_row.aabb {
                                if let Some(aabb_row) = AabbRecord::query_by_id(&thing).await? {
                                    let aabb_sql = export_aabb_record(&aabb_row);
                                    sqls.push(aabb_sql);
                                }
                            }

                            // 导出 inst_geo 中的 pts 数组（vec3:⟨...⟩）
                            for thing in geo_row.pts {
                                if let Some(vrow) = Vec3Record::query_by_id(&thing).await? {
                                    let vsql = export_vec3_record(&vrow);
                                    sqls.push(vsql);
                                }
                            }
                        }

                        // 导出关联的 trans 记录
                        if let Some(trans_row) = TransRecord::query_by_id(&geo_rel.trans).await? {
                            let trans_sql = export_trans_record(&trans_row);
                            sqls.push(trans_sql);
                        }

                        // 导出 pts 数组中的 vec3 记录
                        for pt_thing in &geo_rel.pts {
                            if let Some(vrow) = Vec3Record::query_by_id(pt_thing).await? {
                                let vsql = export_vec3_record(&vrow);
                                sqls.push(vsql);
                            }
                        }
                    }
                }
            }
            // tubi: 仅 BRAN 才有
            if pe.noun == "BRAN" {
                if let Ok(tubis) = TubiRelate::query_by_in_refno(refno.into()).await {
                    for t in tubis {
                        // 导出 tubi_relate 行
                        let tubi_sql = export_tubi_relate(&t);
                        // dbg!(&tubi_sql);
                        sqls.push(tubi_sql);
                        // 导出 aabb
                        if let Some(aabb_row) = AabbRecord::query_by_id(&t.aabb).await? {
                            let aabb_sql = export_aabb_record(&aabb_row);
                            // dbg!(&aabb_sql);
                            sqls.push(aabb_sql);
                        }
                        // 导出 trans
                        if let Some(trans_row) = TransRecord::query_by_id(&t.world_trans).await? {
                            let trans_sql = export_trans_record(&trans_row);
                            // dbg!(&trans_sql);
                            sqls.push(trans_sql);
                        }
                        // 导出 inst_geo 记录（t.out 指向 inst_geo:⟨...⟩），并导出其中 aabb/pts 引用
                        if let Some(geo_row) = InstGeoRecord::query_by_id(&t.out).await? {
                            let geo_sql = export_inst_geo_record(&geo_row);
                            // dbg!(&geo_sql);
                            sqls.push(geo_sql);
                            // aabb 字段（如果存在）
                            if let Some(thing) = geo_row.aabb {
                                if let Some(aabb_row) = AabbRecord::query_by_id(&thing).await? {
                                    let aabb_sql = export_aabb_record(&aabb_row);
                                    sqls.push(aabb_sql);
                                }
                            }
                            // pts 数组（vec3:⟨...⟩）
                            for thing in geo_row.pts {
                                if let Some(vrow) = Vec3Record::query_by_id(&thing).await? {
                                    let vsql = export_vec3_record(&vrow);
                                    sqls.push(vsql);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // 外部引用
    while let Some(child_ref) = queue.pop() {
        if let Some(child_pe) = get_pe(child_ref.into()).await? {
            let child_tree_sql = export_tree_node(&child_pe);
            // dbg!(&child_tree_sql);
            sqls.push(child_tree_sql);
            // 导出 child 的 owner 关系
            if let Ok(child_rel) =
                OwnerRelate::query_owner_relations_by_refno(child_ref.into()).await
            {
                if !child_rel.is_empty() {
                    let relate_sql = export_owner_relate(child_rel[0].clone());
                    // dbg!(&relate_sql);
                    sqls.push(relate_sql);
                }
            }
            // 导出 child 的属性，并将其中的外部引用加入队列
            if let Ok(child_attr) = get_all_attributes(&child_pe, aios_mgr).await {
                let child_attr_sql = generate_attr_insert_sql(&child_pe, &child_attr);
                // dbg!(&child_attr_sql);
                sqls.push(child_attr_sql);
                for r in extract_refnos_from_attributes(&child_attr) {
                    if r.0 != 0 && r.is_valid() && !visited.contains(&r) {
                        visited.insert(r);
                        queue.push(r);
                    }
                }
            }
        }
    }
    Ok((sqls, meshes))
}

fn export_tree_node(pe: &SPdmsElement) -> String {
    // 生成 SurrealDB INSERT 语句
    let insert_sql = format!(
        r#"INSERT IGNORE INTO pe {{ id: {}, refno: {},owner: {},name: "{}",noun: "{}",dbnum: {},sesno: {},status_code: {},cata_hash: "{}",lock: {},deleted: {} }};"#,
        pe.refno.to_pe_key(),            //  id
        pe.refno.to_table_key(&pe.noun), // refno
        pe.owner.to_pe_key(),            // owner
        pe.name,                         // name
        pe.noun,                         // noun
        pe.dbnum,                        // dbnum
        pe.sesno,                        // sesno
        match &pe.status_code {
            // status_code 处理 Option
            Some(code) => format!(r#""{}""#, code),
            None => "NONE".to_string(),
        },
        pe.cata_hash, // 转义 cata_hash 中的引号
        pe.lock,      // lock 布尔值
        pe.deleted,
    );
    insert_sql
}

fn export_owner_relate(relate: OwnerRelate) -> String {
    format!(
        "INSERT RELATION INTO pe_owner {{ id: pe_owner:[{1}, {2}], in: {0}, out: {1} }}",
        relate.r#in, relate.id.0, relate.id.1
    )
}

/// 查询所有的属性，不包含uda
async fn get_all_attributes(
    refno: &SPdmsElement,
    aios_mgr: &AiosDBMgr,
) -> anyhow::Result<NamedAttrMap> {
    let attr = aios_mgr.get_attr(refno.refno.refno()).await?;
    let attr = attr
        .map
        .into_iter()
        .filter(|(k, v)| !k.starts_with(":"))
        .collect::<BTreeMap<String, NamedAttrValue>>();
    Ok(NamedAttrMap { map: attr })
}

fn export_attributes(refno: &SPdmsElement, attmap: &NamedAttrMap) -> String {
    let mut fields = Vec::new();
    for (k, v) in &attmap.map {
        let field_value = match v {
            NamedAttrValue::IntegerType(val) => val.to_string(),
            NamedAttrValue::StringType(val) => format!(r#""{}""#, val),
            NamedAttrValue::F32Type(val) => val.to_string(),
            NamedAttrValue::BoolType(val) => val.to_string(),
            NamedAttrValue::LongType(val) => val.to_string(),
            NamedAttrValue::RefU64Type(val) => format!("{}", val.to_pe_key()),
            NamedAttrValue::RefnoEnumType(val) => format!("{}", val.to_pe_key()),
            NamedAttrValue::ElementType(val) => format!(r#""{}""#, val),
            NamedAttrValue::WordType(val) => format!(r#""{}""#, val),
            NamedAttrValue::F32VecType(val) => {
                let vec_str = val
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", vec_str)
            }
            NamedAttrValue::Vec3Type(val) => {
                format!("[{}, {}, {}]", val.x, val.y, val.z)
            }
            NamedAttrValue::StringArrayType(val) => {
                let vec_str = val
                    .iter()
                    .map(|x| format!(r#""{}""#, x))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", vec_str)
            }
            NamedAttrValue::BoolArrayType(val) => {
                let vec_str = val
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", vec_str)
            }
            NamedAttrValue::IntArrayType(val) => {
                let vec_str = val
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", vec_str)
            }
            NamedAttrValue::RefU64Array(val) => {
                let vec_str = val
                    .iter()
                    .map(|x| format!("{}", x.to_pe_key()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", vec_str)
            }
            NamedAttrValue::InvalidType => "NONE".to_string(),
        };

        fields.push(format!("{}: {}", k, field_value));
    }
    fields.push(format!(
        "{}: {}",
        "id",
        refno.refno().to_table_key(&refno.noun)
    ));

    if fields.is_empty() {
        return "{}".to_string();
    }

    format!("{{ {} }}", fields.join(", "))
}

/// 生成属性表的 SurrealDB INSERT 语句
fn generate_attr_insert_sql(refno: &SPdmsElement, attmap: &NamedAttrMap) -> String {
    let attr_data = export_attributes(refno, attmap);
    format!("INSERT IGNORE INTO {} {};", refno.noun, attr_data)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct OwnerRelate {
    pub id: (Thing, u32),
    pub r#in: Thing,
    pub out: Thing,
}

impl OwnerRelate {
    /// 查询指定 RefnoEnum 的所有者关系
    pub async fn query_owner_relations_by_refno(
        refno: crate::RefnoEnum,
    ) -> anyhow::Result<Vec<OwnerRelate>> {
        use crate::SUL_DB;

        let sql = format!(
            "select record::id(id) as id, in, out from {}->pe_owner;",
            refno.to_pe_key()
        );

        let mut response = SUL_DB.query(sql).await?;
        let relations: Vec<OwnerRelate> = response.take(0)?;

        Ok(relations)
    }
}

/// 查询模型相关的所有数据
async fn get_inst_data(refno: &SPdmsElement, mut sqls: &mut Vec<String>) -> anyhow::Result<()> {
    // inst_relate
    let inst_relate = InstRelate::query_inst_relations_by_refno(refno.refno.into()).await?;
    if !inst_relate.is_empty() {
        for relate in inst_relate {
            // 导出 inst_relate 本体
            let relate_sql = export_inst_relate(relate.clone());
            // dbg!(&relate_sql);
            sqls.push(relate_sql);
            // 进一步导出 aabb 表记录
            if let Some(aabb) = &relate.aabb {
                if let Some(aabb_row) = AabbRecord::query_by_id(aabb).await? {
                    let aabb_sql = export_aabb_record(&aabb_row);
                    // dbg!(&aabb_sql);
                    sqls.push(aabb_sql);
                }
            }
            // 进一步导出 trans 表记录
            if let Some(trans_row) = TransRecord::query_by_id(&relate.world_trans).await? {
                let trans_sql = export_trans_record(&trans_row);
                // dbg!(&trans_sql);
                sqls.push(trans_sql);
            }
            // 进一步导出 inst_info 表记录（out）
            if let Some(info_row) = InstInfoRecord::query_by_id(&relate.out).await? {
                let info_sql = export_inst_info_record(&info_row);
                // dbg!(&info_sql);
                sqls.push(info_sql);

                // 导出geo_relate
                let geo_relates = GeoRelate::query_by_inst_info_id(&relate.out).await?;
                for geo_rel in geo_relates {
                    // 导出 geo_relate 本体
                    let geo_sql = export_geo_relate(&geo_rel);
                    sqls.push(geo_sql);

                    // 导出关联的 inst_geo 记录
                    if let Some(geo_row) = InstGeoRecord::query_by_id(&geo_rel.out).await? {
                        let geo_sql = export_inst_geo_record(&geo_row);
                        sqls.push(geo_sql);

                        // 导出 inst_geo 中的 aabb 字段（如果存在）
                        if let Some(thing) = geo_row.aabb {
                            if let Some(aabb_row) = AabbRecord::query_by_id(&thing).await? {
                                let aabb_sql = export_aabb_record(&aabb_row);
                                sqls.push(aabb_sql);
                            }
                        }

                        // 导出 inst_geo 中的 pts 数组（vec3:⟨...⟩）
                        for thing in geo_row.pts {
                            if let Some(vrow) = Vec3Record::query_by_id(&thing).await? {
                                let vsql = export_vec3_record(&vrow);
                                sqls.push(vsql);
                            }
                        }
                    }

                    // 导出关联的 trans 记录
                    if let Some(trans_row) = TransRecord::query_by_id(&geo_rel.trans).await? {
                        let trans_sql = export_trans_record(&trans_row);
                        sqls.push(trans_sql);
                    }

                    // 导出 pts 数组中的 vec3 记录
                    for pt_thing in &geo_rel.pts {
                        if let Some(vrow) = Vec3Record::query_by_id(pt_thing).await? {
                            let vsql = export_vec3_record(&vrow);
                            sqls.push(vsql);
                        }
                    }
                }
            }
        }

        #[derive(Serialize, Deserialize, Clone, Debug)]
        struct TubiRelate {
            pub id: Thing,
            pub r#in: Thing,
            pub out: Thing, // inst_geo
            pub aabb: Thing,
            pub world_trans: Thing,
            pub arrive: Thing,
            pub leave: Thing,
            #[serde(default)]
            pub bore_size: Vec<i32>,
        }

        impl TubiRelate {
            pub async fn query_by_in_refno(
                refno: crate::RefnoEnum,
            ) -> anyhow::Result<Vec<TubiRelate>> {
                use crate::SUL_DB;
                let sql = format!("select * from {}->tubi_relate;", refno.to_pe_key());
                let mut response = SUL_DB.query(sql).await?;
                let rows: Vec<TubiRelate> = response.take(0)?;
                Ok(rows)
            }
        }

        fn export_tubi_relate(rel: &TubiRelate) -> String {
            let bore = if rel.bore_size.is_empty() {
                "[]".to_string()
            } else {
                let s = rel
                    .bore_size
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", s)
            };
            format!(
        "INSERT IGNORE INTO tubi_relate {{ id: {}, in: {}, out: {}, aabb: {}, world_trans: {}, arrive: {}, leave: {}, bore_size: {} }};",
        rel.id, rel.r#in, rel.out, rel.aabb, rel.world_trans, rel.arrive, rel.leave, bore
    )
        }
    }
    Ok(())
}

fn extract_refnos_from_attributes(attmap: &NamedAttrMap) -> Vec<RefU64> {
    let mut refs = Vec::new();
    for (k, v) in &attmap.map {
        let key = k.to_lowercase();
        if key == "id" || key == "refno" || key == "owner" {
            continue;
        }
        match v {
            NamedAttrValue::RefU64Type(r) => {
                if r.is_valid() {
                    refs.push(*r);
                }
            }
            NamedAttrValue::RefnoEnumType(r) => {
                let rr = r.refno();
                if rr.is_valid() {
                    refs.push(rr);
                }
            }
            _ => {}
        }
    }
    refs
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct InstRelate {
    pub id: Thing,
    pub r#in: Thing,
    pub out: Thing,
    pub aabb: Option<Thing>,
    pub world_trans: Thing,
    pub generic: String,
    pub has_cata_neg: bool,
    pub solid: bool,
    pub zone_refno: Option<Thing>,
    pub dt: Option<surrealdb::sql::Datetime>,
}

impl InstRelate {
    /// 查询指定 RefnoEnum 的实例关系
    pub async fn query_inst_relations_by_refno(
        refno: crate::RefnoEnum,
    ) -> anyhow::Result<Vec<InstRelate>> {
        use crate::SUL_DB;

        let sql = format!("select * from inst_relate:{};", refno.to_string());

        let mut response = SUL_DB.query(sql).await?;
        let relations: Vec<InstRelate> = response.take(0)?;

        Ok(relations)
    }
}

fn export_inst_relate(relate: InstRelate) -> String {
    format!(
            "INSERT RELATION INTO inst_relate {{ id: {}, in: {}, out: {}, aabb: {}, world_trans: {}, generic: \"{}\", has_cata_neg: {}, solid: {}, dt: {} }};",
            relate.id.clone(),
            relate.r#in,
            relate.out,
            relate.aabb.unwrap_or(relate.id),
            relate.world_trans,
            relate.generic,
            relate.has_cata_neg,
            relate.solid,
            relate.dt.unwrap_or(Datetime::default())
        )
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct TubiRelate {
    pub id: Thing,
    pub r#in: Thing,
    pub out: Thing, // inst_geo
    pub aabb: Thing,
    pub world_trans: Thing,
    pub arrive: Thing,
    pub leave: Thing,
    #[serde(default)]
    pub bore_size: Vec<f32>,
}

impl TubiRelate {
    /// 查询指定 RefnoEnum 的所有 tubi 关系（仅 BRAN 有）
    pub async fn query_by_in_refno(refno: crate::RefnoEnum) -> anyhow::Result<Vec<TubiRelate>> {
        use crate::SUL_DB;
        let sql = format!("select * from {}->tubi_relate;", refno.to_pe_key());
        let mut response = SUL_DB.query(sql).await?;
        let rows: Vec<TubiRelate> = response.take(0).unwrap();
        Ok(rows)
    }
}

fn export_tubi_relate(rel: &TubiRelate) -> String {
    let bore = if rel.bore_size.is_empty() {
        "[]".to_string()
    } else {
        let s = rel
            .bore_size
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{}]", s)
    };
    format!(
        "INSERT RELATION INTO tubi_relate {{ id: {}, in: {}, out: {}, aabb: {}, world_trans: {}, arrive: {}, leave: {}, bore_size: {} }};",
        rel.id, rel.r#in, rel.out, rel.aabb, rel.world_trans, rel.arrive, rel.leave, bore
    )
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Vec3Record {
    pub id: Thing,
    pub d: serde_json::Value,
}

impl Vec3Record {
    pub async fn query_by_id(id: &Thing) -> anyhow::Result<Option<Vec3Record>> {
        use crate::SUL_DB;
        let sql = format!("select * from {};", id);
        let mut response = SUL_DB.query(sql).await?;
        let mut rows: Vec<Vec3Record> = response.take(0)?;
        Ok(rows.pop())
    }
}

fn export_vec3_record(row: &Vec3Record) -> String {
    format!(
        "INSERT IGNORE INTO vec3 {{ id: {}, d: {} }};",
        row.id, row.d
    )
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct InstGeoRecord {
    pub id: Thing,
    #[serde(default)]
    pub aabb: Option<Thing>,
    #[serde(default)]
    pub meshed: Option<bool>,
    #[serde(default)]
    pub param: Option<serde_json::Value>,
    #[serde(default)]
    pub pts: Vec<Thing>,
}

impl InstGeoRecord {
    /// 根据 inst_geo 主键查询行
    pub async fn query_by_id(id: &Thing) -> anyhow::Result<Option<InstGeoRecord>> {
        use crate::SUL_DB;
        let sql = format!("select * from {};", id);
        let mut response = SUL_DB.query(sql).await?;
        let mut rows: Vec<InstGeoRecord> = response.take(0)?;
        Ok(rows.pop())
    }
}

fn export_inst_geo_record(row: &InstGeoRecord) -> String {
    let aabb = row
        .aabb
        .clone()
        .map(|t| t.to_string())
        .unwrap_or("NONE".into());
    let pts = row
        .pts
        .iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let meshed = row.meshed.unwrap_or(false);
    let param = row
        .param
        .as_ref()
        .map(|p| p.to_string())
        .unwrap_or("{}".to_string());
    format!(
        "INSERT IGNORE INTO inst_geo {{ id: {}, aabb: {}, meshed: {}, param: {}, pts: [{}] }};",
        row.id, aabb, meshed, param, pts
    )
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AabbRecord {
    pub id: Thing,
    pub d: serde_json::Value,
}

impl AabbRecord {
    /// 根据 aabb 表的主键查询整条记录，例如 aabb:⟨...⟩
    pub async fn query_by_id(aabb_id: &Thing) -> anyhow::Result<Option<AabbRecord>> {
        use crate::SUL_DB;
        let sql = format!("select * from {};", aabb_id);
        let mut response = SUL_DB.query(sql).await?;
        let mut rows: Vec<AabbRecord> = response.take(0)?;
        Ok(rows.pop())
    }
}

fn export_aabb_record(row: &AabbRecord) -> String {
    format!(
        "INSERT IGNORE INTO aabb {{ id: {}, d: {} }};",
        row.id, row.d
    )
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct TransRecord {
    pub id: Thing,
    pub d: serde_json::Value,
}

impl TransRecord {
    /// 根据 trans 表的主键查询整条记录，例如 trans:⟨...⟩
    pub async fn query_by_id(trans_id: &Thing) -> anyhow::Result<Option<TransRecord>> {
        use crate::SUL_DB;

        let sql = format!("select * from {};", trans_id);
        let mut response = SUL_DB.query(sql).await?;
        let mut rows: Vec<TransRecord> = response.take(0)?;
        Ok(rows.pop())
    }
}

fn export_trans_record(row: &TransRecord) -> String {
    format!(
        "INSERT IGNORE INTO trans {{ id: {}, d: {} }};",
        row.id, row.d
    )
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct InstInfoRecord {
    pub id: Thing,
    #[serde(flatten)]
    pub fields: serde_json::Map<String, serde_json::Value>,
}

impl InstInfoRecord {
    pub async fn query_by_id(info_id: &Thing) -> anyhow::Result<Option<InstInfoRecord>> {
        use crate::SUL_DB;
        let sql = format!("select * from {};", info_id);
        let mut response = SUL_DB.query(sql).await?;
        let mut rows: Vec<InstInfoRecord> = response.take(0)?;
        Ok(rows.pop())
    }
}

fn export_inst_info_record(row: &InstInfoRecord) -> String {
    let mut parts = Vec::new();
    parts.push(format!("id: {}", row.id));
    for (k, v) in &row.fields {
        parts.push(format!("{}: {}", k, v));
    }
    format!("INSERT IGNORE INTO inst_info {{ {} }};", parts.join(", "))
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct GeoRelate {
    pub id: Thing,
    pub r#in: Thing,
    pub out: Thing,
    pub geo_type: String,
    pub geom_refno: Thing,
    #[serde(default)]
    pub pts: Vec<Thing>,
    pub trans: Thing,
    pub visible: bool,
}

impl GeoRelate {
    /// 根据 inst_info ID 查询所有相关的 geo_relate 记录
    pub async fn query_by_inst_info_id(inst_info_id: &Thing) -> anyhow::Result<Vec<GeoRelate>> {
        use crate::SUL_DB;
        let sql = format!("select * from {}->geo_relate;", inst_info_id);
        let mut response = SUL_DB.query(sql).await?;
        let rows: Vec<GeoRelate> = response.take(0)?;
        Ok(rows)
    }
}

fn export_geo_relate(rel: &GeoRelate) -> String {
    let pts = rel
        .pts
        .iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "INSERT RELATION INTO geo_relate {{ id: {}, in: {}, out: {}, geo_type: \"{}\", geom_refno: {}, pts: [{}], trans: {}, visible: {} }};",
        rel.id, rel.r#in, rel.out, rel.geo_type, rel.geom_refno, pts, rel.trans, rel.visible
    )
}

#[tokio::test]
async fn test_export_surreal_data() {
    init_test_surreal().await.unwrap();
    // 创建测试数据
    let test_refno = RefU64::from("17414/24944");
    let aios_mgr = AiosDBMgr::init_from_db_option().await.unwrap();

    // 测试导出功能
    let (sqls,_) = export_surreal_data(test_refno, &aios_mgr).await.unwrap();
    let sqls = sqls.join(";").into_bytes();
    // 生成sql文件
    let file_name = format!("{}_{}.txt", test_refno.get_0(), test_refno.get_1());
    let mut file = std::fs::File::create(file_name.as_str()).unwrap();
    file.write_all(&sqls).unwrap();
}
