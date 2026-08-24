use crate::RefU64;

#[test]
fn test_des_refno() {
    //生成针对refno反序列化的测试，包含 string和u64两种情况
    let num = 8242042241025u64;
    let num_str = "pe:1919_1";

    let refno: RefU64 = num_str.into();
    dbg!(refno);
    dbg!(refno.0);
    let test_refno: RefU64 =
        serde_json::from_str(serde_json::to_string(&num).unwrap().as_str()).unwrap();
    dbg!(test_refno);

    let test_refno1: RefU64 =
        serde_json::from_str(serde_json::to_string(&num_str).unwrap().as_str()).unwrap();
    dbg!(test_refno1);

    assert_eq!(refno, test_refno);
    assert_eq!(refno, test_refno1);
}
