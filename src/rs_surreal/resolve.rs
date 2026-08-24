use std::collections::HashSet;
use std::{collections::HashMap, str::FromStr};

use crate::RefnoEnum;
use crate::pdms_types::PdmsGenericType;
use crate::{
    NamedAttrMap, NamedAttrValue, RefU64, math::polish_notation::Stack,
    tiny_expr::expr_eval::interp, tool::float_tool::f64_round_3,
};
use dashmap::DashMap;
use derive_more::{Deref, DerefMut};
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use tokio::sync::RwLock;

//生成模型的中间过程中产生的伪属性，需要保存下来
//使用once_cell, 初始化一个dashmap, 后面去修改用这个dashmap来保存NamedAttMap
//加上tokio的读写锁，保证线程安全
pub static HASH_PSEUDO_ATT_MAPS: Lazy<RwLock<HashMap<String, NamedAttrMap>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

static COMPATIBLE_UNIT_MAP: Lazy<HashMap<&'static str, HashSet<&'static str>>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("INT", ["DIST"].into());
    m.insert("DIST", ["INT"].into());
    m
});

//todo 收集所有的种类，不在这里面的为NONE
#[inline]
pub fn check_unit_compatible(unit_a: &str, unit_b: &str) -> bool {
    unit_a == unit_b
        || (unit_a == "REAL" || unit_b == "REAL")
        || (unit_a == "NUME" || unit_b == "NUME")
        || (unit_a == "DATA" || unit_b == "DATA")
        || COMPATIBLE_UNIT_MAP
            .get(unit_a)
            .map(|x| x.contains(unit_b))
            .unwrap_or(false)
}

pub const INTERNAL_PDMS_EXPRESS: [&'static str; 27] = [
    "MAX", "MIN", "COS", "SIN", "LOG", "ABS", "POW", "SQR", "NOT", "AND", "OR", "ATAN", "ACOS",
    "ATAN2", "ASIN", "INT", "OF", "MOD", "NEGATE", "SUM", "TANF", "TAN", "TIMES", "MULT", "DIV",
    "ADD", "MINUS",
];

/// 元件库表达式相关的参数
#[derive(Debug, Default, Clone, Deref, DerefMut)]
pub struct CataContext {
    #[deref]
    #[deref_mut]
    pub context: DashMap<String, String>,
    pub is_tubi: bool,
}

impl CataContext {
    pub fn insert(&self, key: impl Into<String>, value: impl Into<String>) {
        self.context.insert(key.into(), value.into());
    }
    pub fn get(&self, key: impl AsRef<str>) -> Option<String> {
        self.context.get(key.as_ref()).map(|x| x.value().clone())
    }
    pub fn contains_key(&self, key: impl AsRef<str>) -> bool {
        self.context.contains_key(key.as_ref())
    }

    #[inline]
    pub fn is_tubi(&self) -> bool {
        self.is_tubi
    }
}

pub const DDHEIGHT_STR: &'static str = "DDHEIGHT";
pub const DDRADIUS_STR: &'static str = "DDRADIUS";
pub const DDANGLE_STR: &'static str = "DDANGLE";

///创建desi参考号的元件库计算上下文
pub async fn get_or_create_cata_context(
    desi_refno: RefnoEnum,
    is_tubi: bool,
) -> anyhow::Result<CataContext> {
    let desi_att = crate::get_named_attmap(desi_refno).await?;
    let mut context = CataContext::default();
    context.is_tubi = is_tubi;
    if let Some(v) = desi_att.get_as_string("JUSL") {
        context.insert("JUSL".to_string(), v);
    }
    context.insert("DESI_REFNO".to_string(), desi_refno.to_string());
    let desp = desi_att.get_f32_vec("DESP").unwrap_or_default();
    for i in 0..desp.len() {
        context.insert(format!("DESI{}", i + 1), desp[i].to_string());
        context.insert(format!("DESP{}", i + 1), desp[i].to_string());
    }
    let ddesp = desi_att.get_ddesp().unwrap_or_default();
    // dbg!(&ddesp);
    for i in 0..ddesp.len() {
        context.insert(format!("DDES{}", i + 1), ddesp[i].to_string());
    }

    let height = desi_att.get_as_string("HEIG").unwrap_or("0.0".into());
    context.insert(DDHEIGHT_STR.to_string(), height.clone());
    let angle = desi_att.get_as_string("ANGL").unwrap_or("0.0".into());
    context.insert(DDANGLE_STR.to_string(), angle.clone());
    let radi = desi_att.get_as_string("RADI").unwrap_or("0.0".into());
    context.insert(DDRADIUS_STR.to_string(), radi.clone());

    for (str, v) in &desi_att.map {
        let is_uda = str.starts_with(":");
        let n = str.to_uppercase();
        match v {
            NamedAttrValue::F32Type(d) => {
                if is_uda {
                    dbg!((&n, d));
                }
                context.insert(n, d.to_string());
            }
            NamedAttrValue::F32VecType(ds) => {
                for (i, d) in ds.into_iter().enumerate() {
                    context.insert(format!("{}{}", &n, i + 1), d.to_string());
                }
            }
            _ => {}
        }
    }

    //todo 保温层厚度参数
    // let iparams = self.query_ipara_from_ele(desi_refno).unwrap_or_default();
    // for i in 0..iparams.len() {
    //     context.insert(format!("IPAR{}", i + 1), iparams[i].to_string());
    //     context.insert(format!("IPARM{}", i + 1), iparams[i].to_string());
    // }

    context.insert("RS_DES_REFNO".to_string(), desi_refno.to_string());
    // dbg!(&desi_refno);
    //添加cata的信息
    if let Ok(cata_attmap) = crate::get_cat_attmap(desi_refno).await {
        // dbg!(&cata_attmap);
        context.insert(
            "RS_CATR_REFNO".to_string(),
            cata_attmap.get_refno_or_default().to_string(),
        );
        // dbg!(&cata_attmap);
        let params = cata_attmap.get_f32_vec("PARA").unwrap_or_default();
        for i in 0..params.len() {
            context.insert(format!("CPAR{}", i + 1), params[i].to_string());
            context.insert(format!("PARA{}", i + 1), params[i].to_string());
            context.insert(format!("PARAM{}", i + 1), params[i].to_string());
            context.insert(format!("IPARA{}", i + 1), "0".to_string());
            context.insert(format!("IPAR{}", i + 1), "0".to_string());
        }
        let mut owner_ref = desi_att.get_owner();
        //todo 需要换掉
        let mut owner_att = crate::get_named_attmap(owner_ref).await?;
        //todo use a single query to get all the ancestors' attmap
        while !owner_att.contains_key("GTYP") {
            if owner_att.get_refno().is_none() || owner_att.get_type_str() == "ZONE" {
                break;
            }
            owner_ref = owner_att.get_owner();
            // owner_att = crate::get_named_attmap(owner_ref).await.unwrap_or_default();
            owner_att = crate::get_named_attmap(owner_ref).await?;
        }

        //dtse 的信息处理
        let dtre_refno = cata_attmap.get_foreign_refno("DTRE").unwrap_or_default();
        let children = crate::get_children_named_attmaps(dtre_refno).await?;
        //如果只查部分数据，可以改一下接口
        for child in children {
            if let Some(k) = child.get_as_string("DKEY") {
                let key = format!("RPRO_{}", &k);
                let exp = child.get_as_string("PPRO").unwrap_or_default();
                let default_key = format!("{}_default_expr", key);
                let default_expr = child.get_as_string("DPRO").unwrap_or_default();
                let type_key = format!("{}_default_type", key);
                let type_value = child.get_as_string("PTYP").unwrap_or_default();
                context.insert(key, exp);
                context.insert(default_key, default_expr);
                context.insert(type_key, type_value);
            }
        }

        let desp = owner_att.get_f32_vec("DESP").unwrap_or_default();
        for i in 0..desp.len() {
            context.insert(format!("ODES{}", i + 1), desp[i].to_string());
        }
        //找到owner 参考号，再找到它的元件库params
        if let Ok(parent_cat_am) = crate::get_cat_attmap(owner_ref).await {
            let params = parent_cat_am.get_f32_vec("PARA").unwrap_or_default();
            for i in 0..params.len() {
                context.insert(format!("OPAR{}", i + 1), params[i].to_string());
            }
        }
        let cref = desi_att.get_foreign_refno("CREF");
        if cref.is_some()
            && let Ok(c_att) = crate::get_named_attmap(cref.unwrap()).await
        {
            let desp = c_att.get_f32_vec("DESP").unwrap_or_default();
            for i in 0..desp.len() {
                context.insert(format!("ADES{}", i + 1), desp[i].to_string());
            }
            let c_refno = c_att.get_refno().unwrap_or_default();

            if let Ok(attach_cat_am) = crate::get_cat_attmap(c_refno).await {
                let params = attach_cat_am.get_f32_vec("PARA").unwrap_or_default();
                for i in 0..params.len() {
                    context.insert(format!("APAR{}", i + 1), params[i].to_string());
                }
            }
        }
    }
    // dbg!(&context);
    Ok(context)
}

fn replace_all_result<E>(
    re: &Regex,
    haystack: &str,
    replacement: impl Fn(&Captures) -> Result<String, E>,
) -> Result<String, E> {
    let mut new = String::with_capacity(haystack.len());
    let mut last_match = 0;
    for caps in re.captures_iter(haystack) {
        let m = caps.get(0).unwrap();
        new.push_str(&haystack[last_match..m.start()]);
        new.push_str(&replacement(&caps)?);
        last_match = m.end();
    }
    new.push_str(&haystack[last_match..]);
    Ok(new)
}

pub fn prepare_eval_str(input: &str) -> String {
    input
        .replace("IFTRUE", "if")
        .replace(" LT ", "<")
        .replace(" GT ", ">")
        .replace(" LE ", "<=")
        .replace(" GE ", ">=")
        .replace(" EQ ", "==")
        .replace("ATTRIB", "")
        .replace("DESIGN PARAM", "DESP")
        .replace("DESIGN PARA", "DESP")
}

///评估表达式的值
pub fn eval_str_to_f64(
    input_expr: &str,
    context: &CataContext,
    dtse_unit: &str,
) -> anyhow::Result<f64> {
    if input_expr.is_empty() || input_expr == "UNSET" {
        return Ok(0.0);
    }
    #[cfg(feature = "debug_expr")]
    dbg!(&input_expr);
    let refno = context
        .get("RS_DES_REFNO")
        .and_then(|x| Some(RefnoEnum::from(x.as_str())))
        .unwrap_or_default();
    //处理引用的情况 OF 的情况, 如果需要获取 att value，还是需要用数据库去获取值
    let mut new_exp = prepare_eval_str(input_expr);
    #[cfg(feature = "debug_expr")]
    dbg!(&new_exp);
    if new_exp.contains(" OF ") {
        let re = Regex::new(r"([A-Z\s]+) OF (PREV|NEXT|\d+/\d+)").unwrap();
        for caps in re.captures_iter(&new_exp.clone()) {
            let s = &caps[0];
            let c1 = caps.get(1).map_or("", |m| m.as_str().trim());
            let c2 = caps.get(2).map_or("", |m| m.as_str().trim());
            let is_tubi = context.is_tubi();
            #[cfg(not(target_arch = "wasm32"))]
            {
                let expr_val = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async move {
                        //如果是直段，直接取当前的参考号
                        let target_refno = match c2 {
                            "PREV" => {
                                if is_tubi {
                                    refno
                                } else {
                                    crate::get_next_prev(refno, false).await.unwrap_or_default()
                                }
                            }
                            "NEXT" => crate::get_next_prev(refno, true).await.unwrap_or_default(),
                            _ => c2.into(),
                        };
                        // dbg!(target_refno);
                        let pe = crate::get_pe(target_refno)
                            .await
                            .unwrap_or_default()
                            .unwrap_or_default();
                        // dbg!(&pe);
                        let pseudo_map = HASH_PSEUDO_ATT_MAPS.read().await;
                        // #[cfg(feature = "debug_expr")]
                        // dbg!(&pseudo_map);
                        //判断target_refno是否在pseudo_map，如果有，取出这里的值
                        if let Some(am) = pseudo_map.get(&pe.cata_hash) {
                            if let Some(v) = am.map.get(c1) {
                                return v.get_val_as_string();
                            }
                        }
                        "0".to_owned()
                    })
                });
                new_exp = new_exp.replace(s, expr_val.as_str());
            }
        }
    }

    #[cfg(feature = "debug_expr")]
    dbg!(&new_exp);

    //说明：匹配带小数的情况 PARA[1.1]
    let re = Regex::new(r"(:?[A-Z_]+[0-9]*)(\s*\[?\s*(([1-9]\d*\.?\d*)|(0\.\d*[1-9]\s*))\s*\]?)?")
        .unwrap();
    // 将NEXT PREV 的值统一换成参考号，然后 context_params 要存储 参考号对应的 attr，要是它这个值没有求解，
    // 相当于要递归去求值
    let rpro_re = Regex::new(r"(RPRO)\s+([a-zA-Z0-9]+)").unwrap();
    if new_exp.contains("RPRO") {
        new_exp = replace_all_result(&rpro_re, &new_exp, |caps: &Captures| {
            let key: String = format!("{}_{}", &caps[1], &caps[2]).into();
            let default_key: String = format!("{}_{}_default_expr", &caps[1], &caps[2]).into();
            let key_type: String = format!("{}_{}_default_type", &caps[1], &caps[2]).into();
            let unit_type = context.get(&key_type).unwrap_or_default();
            if (!unit_type.is_empty() && unit_type != dtse_unit)
                && !check_unit_compatible(dtse_unit, &unit_type)
            {
                #[cfg(feature = "debug_expr")]
                dbg!((&new_exp, &unit_type, dtse_unit));
                return Err(anyhow::anyhow!(
                    "DTSE 表达式 {new_exp} 有问题，可能单位不一致"
                ));
            } else {
                #[cfg(feature = "debug_expr")]
                dbg!((&new_exp, &unit_type, dtse_unit));
                let v = context
                    .get(&key)
                    .map(|x| x.to_string())
                    .unwrap_or("0".to_string());
                context.insert(format!("EXPR_HAS_DEFAULT"), "true");
                #[cfg(feature = "debug_expr")]
                dbg!(&v);
                if let Ok(t) = eval_str_to_f64(&v, &context, "DIST") {
                    #[cfg(feature = "debug_expr")]
                    dbg!(t);
                    Ok(t.to_string())
                } else {
                    context.context.remove("EXPR_HAS_DEFAULT");
                    Ok(context
                        .get(&default_key)
                        .map(|x| x.to_string())
                        .unwrap_or("0".to_string()))
                }
            }
        })?
        .trim()
        .to_string();
        #[cfg(feature = "debug_expr")]
        dbg!(&new_exp);
        if let Ok(s) = new_exp.parse::<f64>() {
            // dbg!(s);
            return Ok(s);
        }
    }
    let mut result_exp = new_exp.clone();
    //默认两次
    let mut found_replaced = false;
    let para_name_re =
        Regex::new(r"(DESI(GN)?\s+)?([I|C|O|A)]?PARA?M?)|DESP|(O|A|W|D)DESP?").unwrap();
    let mut uda_context_added = false;
    let mut uda_context = HashMap::new();
    for _ in 0..30 {
        for caps in re.captures_iter(&new_exp) {
            let s = caps[0].trim();
            if INTERNAL_PDMS_EXPRESS.contains(&s) {
                continue;
            }
            let mut para_name = caps.get(1).map_or("", |m| m.as_str());
            let c2 = caps.get(2).map_or("", |m| m.as_str());
            let c3 = caps.get(3).map_or("", |m| m.as_str());
            //处理掉PARA 和 PARAM的区别
            let is_some_param = para_name_re.is_match(para_name);
            if is_some_param {
                if para_name.ends_with("M") {
                    para_name = &para_name[0..para_name.len() - 1];
                }
            }
            // 小数向下取整
            let k: String = format!(
                "{}{}",
                para_name,
                c3.parse::<f32>()
                    .map(|x| x.floor().to_string())
                    .unwrap_or_default()
            )
            .into();
            let is_uda = k.starts_with(":");
            if is_uda && !uda_context_added {
                let refno_str = context.get("RS_DES_REFNO").unwrap();
                // dbg!(&refno_str);
                let refno: RefnoEnum = refno_str.as_str().into();
                // dbg!(&k);
                let uda_map = NamedAttrMap::default();
                #[cfg(not(target_arch = "wasm32"))]
                let uda_map = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async move {
                        crate::get_named_attmap_with_uda(refno)
                            .await
                            .unwrap_or_default()
                    })
                });
                for (kk, vv) in uda_map.map {
                    let kk = kk.to_uppercase();
                    if kk.starts_with({ ":" }) {
                        match vv {
                            NamedAttrValue::F32Type(d) => {
                                let short_name = if kk.len() >= 5 {
                                    kk[..5].to_uppercase()
                                } else {
                                    kk.to_uppercase()
                                };
                                uda_context.insert(short_name, d.to_string());
                                uda_context.insert(kk, d.to_string());
                            }
                            NamedAttrValue::F32VecType(ds) => {
                                let short_name = if kk.len() >= 5 {
                                    kk[..5].to_uppercase()
                                } else {
                                    kk.to_uppercase()
                                };
                                for (i, d) in ds.into_iter().enumerate() {
                                    // dbg!(format!("{} ->{}{}", kk, &short_name, i+1));
                                    uda_context
                                        .insert(format!("{}{}", &short_name, i + 1), d.to_string());
                                    uda_context.insert(format!("{}{}", &kk, i + 1), d.to_string());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                uda_context_added = true;
            }

            if context.contains_key(&k) {
                result_exp = result_exp.replace(s, &context.get(&k).unwrap());
                found_replaced = true;
            } else if is_uda && uda_context.contains_key(&k) {
                result_exp = result_exp.replace(s, &uda_context.get(&k).unwrap());
                found_replaced = true;
            } else if is_some_param {
                // 匹配到没有别的嵌套，比如 cos(DESP[1])，这种应该cos(DESP[1])整体结果为 0
                // dbg!(&result_exp);
                let hash_fallback_value =
                    context.get("EXPR_HAS_DEFAULT").unwrap_or_default() == "true";
                if dtse_unit == "DIST" && (!hash_fallback_value) {
                    result_exp = result_exp.replace(s, "NaN");
                    let re = Regex::new(r"\w+\(NaN\)").unwrap();
                    result_exp = re.replace_all(&result_exp, "0.0").to_string();
                    result_exp = result_exp.replace("NaN", " 0");
                    // println!("{input_expr}： {} not found, use {}.", &k, &result_exp);
                    //
                    found_replaced = true;
                } else {
                    return Err(anyhow::anyhow!(format!(
                        "{input_expr}:： {} not found.",
                        &k
                    )));
                }
            }
        }
        //如果有RPRO 需要执行两次处理
        result_exp = result_exp.replace("ATTRIB", "");
        if result_exp.contains("RPRO") {
            result_exp = rpro_re
                .replace_all(&result_exp, |caps: &Captures| {
                    let key: String = format!("{}_{}", &caps[1], &caps[2]).into();
                    let default_key: String =
                        format!("{}_{}_default_expr", &caps[1], &caps[2]).into();

                    context.get(&key).map(|x| x.to_string()).unwrap_or(
                        context
                            .get(&default_key)
                            .map(|x| x.to_string())
                            .unwrap_or("0".to_string()),
                    )
                })
                .trim()
                .to_string();
            found_replaced = true;
        }
        // dbg!(&result_exp);
        new_exp = result_exp.clone();
        if !found_replaced {
            break;
        }
        found_replaced = false;
    }
    let seg_strs: Vec<String> = result_exp
        .split_whitespace()
        .map(|x| x.trim().into())
        .collect::<Vec<_>>();
    if seg_strs.len() == 0 {
        return Ok(0.0);
    }
    let mut result_string = String::new();
    let mut p_vals = vec![];
    for s in seg_strs {
        let upper_s = s.to_uppercase();
        match upper_s.as_str() {
            "TIMES" | "MULT" => p_vals.push("*".to_string()),
            "DIV" => p_vals.push("/".to_string()),
            "ADD" => p_vals.push("+".to_string()),
            "SUBTRACT" => p_vals.push("-".to_string()),
            "DDHEIGHT" => p_vals.push(context.get("DDHEIGHT").unwrap().to_string()),
            "DDRADIUS" => p_vals.push(context.get("DDRADIUS").unwrap().to_string()),
            "DDANGLE" => p_vals.push(context.get("DDANGLE").unwrap().to_string()),
            _ => {
                if upper_s.ends_with("mm") {
                    p_vals.push(upper_s[..upper_s.len() - 2].to_string());
                } else {
                    p_vals.push(upper_s.to_string())
                }
            }
        }
    }
    let mut i = 0;
    let mut new_vals = vec![];
    while i < p_vals.len() {
        if p_vals[i] == "TWICE" {
            if i + 1 < p_vals.len() {
                if let Ok(val) = p_vals[i + 1].parse::<f64>() {
                    let v = val * 2.0f64;
                    new_vals.push(v.to_string());
                }
            }
            i += 2;
        } else if p_vals[i] == "TANF" {
            if i + 2 < p_vals.len() {
                if let Ok(val) = p_vals[i + 1].parse::<f64>() {
                    if let Ok(angle) = p_vals[i + 2].parse::<f64>() {
                        {
                            let v = val * ((angle / 2.0).to_radians() as f64).tan();
                            new_vals.push(v.to_string());
                        }
                    }
                }
            }
            i += 3;
        } else {
            new_vals.push(p_vals[i].clone());
            i += 1;
        }
    }
    let mut i = 0;
    while i < new_vals.len() {
        if (new_vals[i] == "SUM" || new_vals[i] == "DIFFERENCE") && i < new_vals.len() - 2 {
            if new_vals[i] == "SUM" {
                result_string.push_str(&format!(
                    "({} {} {})",
                    new_vals[i + 1],
                    "+",
                    new_vals[i + 2]
                ));
            } else {
                result_string.push_str(&format!(
                    "({} {} {})",
                    new_vals[i + 1],
                    "-",
                    new_vals[i + 2]
                ));
            }
            i += 3;
        } else {
            result_string.push_str(new_vals[i].as_str());
            i += 1;
        }
        result_string.push_str(" ");
    }
    //排除两个连续的负号的情况
    let final_expr = result_string.trim().to_lowercase().replace("--", "");
    #[cfg(feature = "debug_expr")]
    dbg!(&final_expr);
    match interp(&final_expr) {
        Ok(val) => Ok(f64_round_3(val).into()),
        Err(_) => {
            return if let Ok(mut val) = evalexpr::eval(&final_expr) {
                return Ok(f64_round_3(val.as_float()?).into());
            } else if let Ok(mut stack) = Stack::init(&final_expr) {
                stack.eval().ok_or(anyhow::anyhow!(format!(
                    "后缀表达式求解失败 {}",
                    &input_expr
                )))
            } else {
                #[cfg(debug_assertions)]
                println!("输入表达式有误 : {}", &input_expr);
                // dbg!(&context);
                // println!("计算后表达式 : {}", &result_string);
                // let refno_str = context.get("RS_CATR_REFNO").unwrap().as_str();
                // let refno = RefU64::from_str(refno_str)?;
                // dbg!(interface.unwrap().aios_core::get_named_attmap(refno).await.unwrap());
                Err(anyhow::anyhow!(format!("求解失败 {}", &input_expr)))
            };
        }
    }
}

pub async fn resolve_expression(
    expr: &str,
    desi_refno: RefnoEnum,
    is_tubi: bool,
) -> anyhow::Result<f64> {
    let context = get_or_create_cata_context(desi_refno, is_tubi).await?;
    eval_str_to_f64(expr, &context, "DIST")
}

/// 通用的解析表达式的方法, 解析desi参考号下的 表达式值
/// 如果 desi_refno 为空，代表design的数据不需要参与计算
pub async fn resolve_expression_to_f32(
    expr: &str,
    desi_refno: RefnoEnum,
    is_tubi: bool,
) -> anyhow::Result<f32> {
    let context = get_or_create_cata_context(desi_refno, is_tubi).await?;
    eval_str_to_f32(expr, &context, "DIST")
}

pub fn eval_str_to_f32(
    input_expr: impl AsRef<str>,
    context: &CataContext,
    dtse_unit: &str,
) -> anyhow::Result<f32> {
    let input_expr = input_expr.as_ref().trim().to_uppercase();
    eval_str_to_f64(&input_expr, context, dtse_unit).map(|x| x as f32)
}

pub fn eval_str_to_f32_or_default(
    input_expr: impl AsRef<str>,
    context: &CataContext,
    dtse_unit: &str,
) -> f32 {
    eval_str_to_f32(input_expr, context, dtse_unit).unwrap_or(0.0)
}
