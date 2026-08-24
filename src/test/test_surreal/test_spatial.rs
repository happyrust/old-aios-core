use crate::tool::dir_tool::*;
use crate::tool::direction_parse::parse_expr_to_dir;
use crate::{RefU64, RefnoEnum, RefnoSesno};
use crate::{room::room::load_aabb_tree, rs_surreal, tool::math_tool};
use glam::{DMat3, DQuat, DVec3, Mat3, Quat, Vec3};
use std::sync::Arc;
use surrealdb::sql::Thing;

fn test_print_ori(ori: &str) {
    let rotation = parse_ori_str_to_quat(ori).unwrap_or(glam::Quat::IDENTITY);
    dbg!(Mat3::from_quat(rotation));
    dbg!(math_tool::quat_to_pdms_ori_xyz_str(&rotation, false));
}

// fn test_cal_ori(v: DVec3) {
//     let ref_dir = if v.dot(DVec3::NEG_Z).abs() > 0.999 {
//         DVec3::NEG_Y
//     }else{
//         DVec3::NEG_Z
//     };
//     let y_dir = v.cross(ref_dir).normalize();
//     let x_dir = y_dir.cross(v).normalize();
//     let rotation = DQuat::from_mat3(&DMat3::from_cols(x_dir.into(), y_dir.into(), v.into()));
//     dbg!((x_dir, y_dir, v));
//     dbg!(math_tool::quat_to_pdms_ori_xyz_str(&rotation.as_quat()));
// }

#[cfg(test)]
mod test_transform {
    use crate::test::test_surreal::test_spatial::test_transform;
    use crate::tool::dir_tool::{
        parse_ori_str_to_dquat, parse_ori_str_to_mat, parse_ori_str_to_quat,
    };
    use crate::tool::direction_parse::parse_expr_to_dir;
    use crate::tool::math_tool;
    use crate::tool::math_tool::{
        cal_quat_by_zdir_with_xref, dquat_to_pdms_ori_xyz_str, dvec3_to_xyz_str, to_pdms_dvec_str,
        vec3_to_xyz_str,
    };
    use crate::tool::parse_to_dir::{parse_coordinate, parse_str_to_vec3, parse_to_direction};
    use crate::{
        RefU64, RefnoEnum, RefnoSesno, cal_ori_by_extru_axis, cal_ori_by_z_axis_ref_x, rs_surreal,
    };
    use crate::{cal_ori_by_ydir, init_test_surreal};
    use bevy_reflect::Array;
    use glam::{DMat3, DQuat, DVec3, Mat3};

    //sctn 等等
    #[test]
    fn test_cal_ori_by_line_axis() {
        //Y方向始终在X Z 方向
        let tests = [
            "Y is X and Z is Z",  // X is -Y
            "Y is X and Z is -Z", // X is Y
            "Y is Z and Z is Y",
            "Y is Z and Z is -Y",
            "Y is Z and Z is X",
            "Y is Z and Z is -X",
            "Y is X 45 Z and Z is -Y 45 Z",
            "Y is X 45 Z and Z is Y 45 Z",
            "Y is X 45 Z and Z is X 45 -Z",
            "Y is -X 45 Z and Z is -X 45 -Z",
            "Y is Z and Z is -Y 31 -X",
        ];
        let oris = tests
            .into_iter()
            .map(|x| parse_ori_str_to_mat(x).unwrap())
            .collect::<Vec<_>>();
        // dbg!(&oris);

        for ori in oris {
            let extru_dir = ori.z_axis.as_dvec3();
            // dbg!(to_pdms_dvec_str(&extru_dir));
            let quat = cal_ori_by_extru_axis(extru_dir, false);
            // dbg!(dquat_to_pdms_ori_xyz_str(&quat));
        }
    }

    /// 异步函数，用于测试给定 refno 的方向
    ///
    /// # 参数
    ///
    /// * `refno` - 需要查询的 RefnoEnum
    /// * `assert_ori_str` - 预期的方向字符串
    async fn test_ori(refno: RefnoEnum, assert_ori_str: &str) {
        let transform = rs_surreal::get_world_mat4(refno, false)
            .await
            .unwrap()
            .unwrap();
        let (scale, rot, translation) = transform.to_scale_rotation_translation();

        let assert_ori_value = parse_ori_str_to_dquat(assert_ori_str).unwrap();
        let tol = 0.001;
        let dot_product = rot.dot(assert_ori_value);
        let diff = (1.0 - dot_product.abs()).abs() <= tol || (1.0 + dot_product.abs()).abs() <= tol;
        if !diff {
            println!("Expected ori_str: {}", assert_ori_str);
            println!("Actual ori_str: {}", dquat_to_pdms_ori_xyz_str(&rot, true));
        }
        assert!(diff, "Rotation difference exceeds tolerance: {:?}", tol);

        // dbg!(translation);
        // //如果包含其中的任意一个，则不需要转成XYZ
        // let convert_xyz = ["E", "N", "U", "W", "S", "D"]
        //     .into_iter()
        //     .any(|x| assert_ori.contains(x));
        // let ori_str = dquat_to_pdms_ori_xyz_str(&rot, !convert_xyz);
        // dbg!(&ori_str);
        // assert_eq!(ori_str, assert_ori);
    }

    async fn test_transform(refno: RefnoEnum, assert_ori: &str, pos_str: &str) {
        let transform = rs_surreal::get_world_mat4(refno, false)
            .await
            .unwrap()
            .unwrap();
        let (scale, rot, translation) = transform.to_scale_rotation_translation();

        // dbg!(translation);
        //如果包含其中的任意一个，则不需要转成XYZ
        // let convert_xyz = ["E", "N", "U", "W", "S", "D"]
        //     .into_iter()
        //     .any(|x| assert_ori.contains(x));
        // let ori_str = dquat_to_pdms_ori_xyz_str(&rot, !convert_xyz);
        // dbg!(&ori_str);
        // dbg!(math_tool::dvec3_to_xyz_str(translation));
        // if !assert_ori.is_empty() {
        //     assert_eq!(ori_str, assert_ori);
        // }
        println!(
            "Actual position: {}",
            math_tool::dvec3_to_xyz_str(translation)
        );
        if !assert_ori.is_empty() {
            test_ori(refno, assert_ori).await;
        }

        if !pos_str.is_empty() {
            let tol = 0.01;
            let d = translation - parse_str_to_vec3(pos_str).unwrap();
            dbg!(&d);
            let diff = d.length() <= tol;
            if !diff {
                println!("Expected position: {}", pos_str);
                println!(
                    "Actual position: {}",
                    math_tool::dvec3_to_xyz_str(translation)
                );
            }
            assert!(diff, "Position difference exceeds tolerance: {:?}", tol);
            // assert_eq!(math_tool::dvec3_to_xyz_str(translation), pos_str);
        }
    }

    #[tokio::test]
    async fn test_query_transform_DRNS_DRNE() -> anyhow::Result<()> {
        init_test_surreal().await;
        let refno = "17496/202374".into();
        let mat = crate::get_world_mat4(refno, true).await?.unwrap();
        let spine_att = crate::get_named_attmap(refno).await?;
        let drns = spine_att.get_dvec3("DRNS").unwrap();
        let drne = spine_att.get_dvec3("DRNE").unwrap();

        let mat_inv = mat.inverse();
        let local_drns = mat_inv.transform_vector3(drns);
        let local_drne = mat_inv.transform_vector3(drne);
        // dbg!(to_pdms_dvec_str(&local_drns));
        // dbg!(to_pdms_dvec_str(&local_drne));

        let angle_x = (local_drns.x / local_drns.z).atan();
        let angle_y = (local_drns.y / local_drns.z).atan();
        let scale_drns = DVec3::new(1.0 / angle_x.cos(), 1.0 / angle_y.cos(), 1.0);

        let angle_x = (local_drne.x / local_drne.z).atan();
        let angle_y = (local_drne.y / local_drne.z).atan();
        let scale_drne = DVec3::new(1.0 / angle_x.cos(), 1.0 / angle_y.cos(), 1.0);

        dbg!(scale_drns);
        dbg!(scale_drne);

        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_ENDATU() -> anyhow::Result<()> {
        init_test_surreal().await;

        test_ori(
            "17496/268348".into(),
            "Y is X 89.891 Z and Z is -X 0.1089 Z",
        )
        .await;
        // test_ori("17496/273497".into(), "Y is X and Z is Z").await;
        test_ori("24384/25783".into(), "Y is X 89.969 Z and Z is -X 0.031 Z").await;

        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_SNODE() -> anyhow::Result<()> {
        init_test_surreal().await;

        test_transform(
            "24383/93573".into(),
            "",
            "X 10492.213mm Y 24025.362mm Z 12560mm",
        )
        .await;
        // let w1 = parse_ori_str_to_quat("Y is X 21 -Y and Z is Z").unwrap().as_dquat();
        // let w2 = parse_ori_str_to_quat("Y is -Y and Z is X").unwrap().as_dquat();
        // let w3 = parse_ori_str_to_quat(" Y is X and Z is Y").unwrap().as_dquat();
        // let w4 = parse_ori_str_to_quat(" Y is -X and Z is Z").unwrap().as_dquat();
        // dbg!(w4);
        // let w = w1 * w2;
        // let nw  = w3 * w2.inverse();
        // // dbg!(dquat_to_pdms_ori_xyz_str(&w, true));
        // dbg!(dquat_to_pdms_ori_xyz_str(&(w4 * w2), true));
        // dbg!(dquat_to_pdms_ori_xyz_str(&nw, true));
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_SCOJ() -> anyhow::Result<()> {
        init_test_surreal().await;
        //如果是SCOJ, 需要获取两边的连接点，组合出来的方向位置

        test_transform(
            "17496/172030".into(),
            "Y is X and Z is Y",
            "X -42531.5mm Y 11821mm Z 26008mm",
        )
        .await;
        // test_transform("24383/80522".into(), "Y is -X 41 Y and Z is -Y 41 -X", "").await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_CMFI() -> anyhow::Result<()> {
        init_test_surreal().await;
        //如果是SCOJ, 需要获取两边的连接点，组合出来的方向位置

        test_transform(
            "17496/140425".into(),
            "Y is -X 22.452 -Y and Z is Y 22.452 -X",
            "X 3018.841mm Y -7725.884mm Z 900mm",
        )
        .await;
        Ok(())
    }

    #[test]
    fn get_cal_ori() {
        use crate::tool::direction_parse::parse_expr_to_dir;
        let zdir = parse_expr_to_dir("Y 7.2448 X").unwrap();
        let rot = cal_quat_by_zdir_with_xref(zdir);
        // dbg!(dquat_to_pdms_ori_xyz_str(&rot, true));
        assert_eq!(
            dquat_to_pdms_ori_xyz_str(&rot, true),
            "Y is -X 7.245 Y and Z is Y 7.245 X"
        );

        // return;
        let zdir = parse_expr_to_dir("-Z").unwrap();
        let rot = cal_quat_by_zdir_with_xref(zdir);
        // dbg!(dquat_to_pdms_ori_xyz_str(&rot, true));
        assert_eq!(dquat_to_pdms_ori_xyz_str(&rot, true), "Y is Y and Z is -Z");

        let zdir = parse_expr_to_dir("-Y 84.452 -Z").unwrap();
        let rot = cal_quat_by_zdir_with_xref(zdir);
        // dbg!(dquat_to_pdms_ori_xyz_str(&rot, true));
        assert_eq!(
            dquat_to_pdms_ori_xyz_str(&rot, true),
            "Y is Y 5.548 -Z and Z is -Y 84.452 -Z"
        );

        let zdir = parse_expr_to_dir("-Y 84.452 Z").unwrap();
        let rot = cal_quat_by_zdir_with_xref(zdir);
        // dbg!(dquat_to_pdms_ori_xyz_str(&rot, true));
        assert_eq!(
            dquat_to_pdms_ori_xyz_str(&rot, true),
            "Y is Y 5.548 Z and Z is -Y 84.452 Z"
        );
        let mat3 = DMat3::from_quat(
            parse_ori_str_to_quat("Y is X 89.969 Z and Z is -X 0.0307 Z")
                .unwrap()
                .as_dquat(),
        );
        dbg!(mat3);
    }

    #[tokio::test]
    async fn test_query_transform_SBFI() -> anyhow::Result<()> {
        init_test_surreal().await;
        //如果是SCOJ, 需要获取两边的连接点，组合出来的方向位置
        //
        test_transform(
            "17496/140426".into(),
            "Y is X 28.02 Y and Z is -Y 28.02 X",
            "X 3018.841mm Y -7725.884mm Z 900mm",
        )
        .await;
        //17496/140428
        test_transform(
            "17496/140428".into(),
            "Y is -X 25 Y and Z is Y 25 X",
            "X 8964.131mm Y -6883.685mm Z -20mm",
        )
        .await;
        let m1 = parse_ori_str_to_quat("Y is -Y 5.548 Z and Z is -Y 84.452 -Z")
            .unwrap()
            .as_dquat();
        // let m2 = parse_ori_str_to_quat("Y is -X 22.452 -Y and Z is Y 22.452 -X").unwrap().as_dquat();
        // let m3 = m2 * m1;
        // println!("matrix: {}", dquat_to_pdms_ori_xyz_str(&m3, true));
        // let m1 = parse_ori_str_to_quat("Y is -Y 5.548 Z and Z is -Y 84.452 -Z").unwrap().as_dquat();
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_PNODE() -> anyhow::Result<()> {
        init_test_surreal().await;

        test_transform(
            "17496/172032".into(),
            "",
            "X -42531.5mm Y 11821mm Z 25908mm",
        )
        .await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_PJOI() -> anyhow::Result<()> {
        init_test_surreal().await;

        //PJOI里有OPDI
        // test_transform("17496/172033".into(), "Y is Y and Z is -Z", "X -42531.5mm Y 11821mm Z 25908mm").await;
        test_transform(
            "17496/274161".into(),
            "Y is -Y and Z is Z",
            "X 153mm Y -248.5mm Z 35mm",
        )
        .await;
        // test_transform("24381/178550".into(), "Y is -X 27.581 Y and Z is -Y 27.581 -X", "X 12850.226mm Y 24922.073mm Z -4194.68mm").await;
        // let m1 = parse_ori_str_to_quat("Y is X and Z is Y").unwrap();
        // let m2 = parse_ori_str_to_quat("Y is -Y 27.581 -X and Z is Z").unwrap();
        // let m3 = m2 * m1;
        // dbg!(dquat_to_pdms_ori_xyz_str(&m3.as_dquat(), true));
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_SJOI() -> anyhow::Result<()> {
        use crate::tool::dir_tool::*;
        init_test_surreal().await;

        test_transform(
            "23708/2475".into(),
            "Y is -X and Z is Z",
            "X 1971.3mm Y 3174mm Z -89mm",
        )
        .await;

        // SJOI 里的CUTP，如果有CUTP，x轴必须为Z轴，如果CUTP轴为(-)Z轴了，z轴改为X，
        test_transform(
            "17496/274158".into(),
            "Y is Z and Z is Y",
            "X 175mm Y -233.5mm Z -25mm",
        )
        .await;
        // test_transform(
        //     "23713/2430".into(),
        //     "Y is -X and Z is Z",
        //     "X 1982.3mm Y 2786.6mm Z -100.2mm",
        // )
        // .await;
        // test_transform(
        //     "23713/2699".into(),
        //     "Y is Z and Z is X",
        //     "X 3000.1mm Y 4786.6mm Z -18.6mm",
        // )
        // .await;
        // return Ok(());
        //with cutplane
        // let sjoi_strs = vec![
        //     "Y is -X 22.452 -Y and Z is -Y 22.452 X",
        //     "Y is -X 20 -Y and Z is -Y 20 X",
        //     "Y is -X 20 Y and Z is -Y 20 -X",
        //     "Y is -Y and Z is X",
        //     "Y is -X and Z is -Y",
        //     "Y is X and Z is Y",
        //     "Y is Y 30.484 X and Z is -X 30.484 Y",
        // ];
        // for s in sjoi_strs {
        //     let m = DMat3::from_quat(parse_ori_str_to_quat(s).unwrap().as_dquat());
        //     dbg!(m.x_axis);
        // }
        let test_cases = vec![
            // ("Y 36.85 -X", "Z 30 Y", "Y is -X 36.85 -Y 19.099 Z and Z is X 36.85 Y 70.901 Z"),
            // ("Y 36.85 -X", "Z", "Y is -X 36.85 -Y and Z is Z"),
            // ("Y 36.85 X", "Z", "Y is -X 36.85 Y and Z is Z"),
            // ("Y 36.85 X", "Y 36.85 X", "Y is Z and Z is X 36.85 -Y"),
            // ("Y 36.85 X", "Y 36.85 X", "Y is Z and Z is X 36.85 -Y"),
            ("-X", "-X", "Y is Z and Z is Y"),
        ];
        for (cutp, axis_dir, result) in test_cases {
            let cutp = parse_expr_to_dir(cutp).unwrap();
            let axis_dir = parse_expr_to_dir(axis_dir).unwrap();
            let assert_mat = parse_ori_str_to_dmat3(result).unwrap();
            // dbg!(to_pdms_dvec_str(&assert_mat.x_axis,true));
            //cal_cutp_ori
            let ori = crate::cal_cutp_ori(cutp, axis_dir);
            let mat3 = DMat3::from_quat(ori);
            let delta = mat3 * assert_mat.inverse();
            let ori_str = dquat_to_pdms_ori_xyz_str(&ori, true);
            dbg!(&ori_str);
            assert!(delta.determinant() > 0.99);
        }
        // let ori1 = parse_ori_str_to_quat(" Y is -X 36.85 -Y 77.573 Z and Z is X 36.85 Y 12.427 Z")
        //     .unwrap()
        //     .as_dquat();
        // let ori2 = parse_ori_str_to_quat(" Y is -X 36.85 -Y and Z is Z")
        //     .unwrap()
        //     .as_dquat();
        // let ori = ori2.inverse() * ori1;
        // //ori1 = ori2 * xxx;
        // // dbg!(DMat3::from_quat(ori));
        // dbg!(dquat_to_pdms_ori_xyz_str(&ori, true));

        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_STWALL() -> anyhow::Result<()> {
        init_test_surreal().await;
        //如果是SCOJ, 需要获取两边的连接点，组合出来的方向位置

        test_ori("17496/105740".into(), "Y is Z and Z is -X 22.452 -Y").await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_SCTN() -> anyhow::Result<()> {
        init_test_surreal().await;

        //todo fix
        // test_transform("17496/274032".into(), "Y is -Z and Z is X").await;
        test_transform("24381/178543".into(), "Y is -Y 27.581 -X and Z is Z", "").await;

        // test_transform(
        //     "24381/77310".into(),
        //     "Y is Z and Z is Y 43 -X",
        //     "X 9574.353mm Y 8934.174mm Z 4345.8mm",
        // )
        // .await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_TMPL() -> anyhow::Result<()> {
        init_test_surreal().await;
        test_ori("8196/14755".into(), "Y is -X and Z is Z").await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_JLDATU() -> anyhow::Result<()> {
        init_test_surreal().await;
        // test_ori("17496/152079".into(), "Y is Z and Z is -Y 29.298 X").await;
        test_transform("17496/274326".into(), "", "").await;
        test_transform("17496/274327".into(), "", "").await;
        test_transform("17496/274328".into(), "", "").await;
        // test_transform("17496/274328".into(), "Y is Z and Z is Y", "").await;
        // test_transform("17496/268340".into(), "Y is -Y and Z is -Z", "X -218mm Y -21mm Z 400mm").await;
        // test_transform("25688/48820".into(), "Y is Z and Z is X 33.956 Y", "").await;
        // test_transform("24384/28751".into(), "Y is Y 31 X 89.969 Z and Z is -Y 31 -X 0.031 Z", "X -12934.344mm Y -21974.977mm Z -1090.3mm").await;
        // test_transform("17496/137181".into(), "Y is Z and Z is -Y 34.6032 -X").await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_SPINE() -> anyhow::Result<()> {
        init_test_surreal().await;
        test_ori(
            "17496/268345".into(),
            "Y is X 89.891 Z and Z is -X 0.1089 Z",
        )
        .await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_BOX() -> anyhow::Result<()> {
        init_test_surreal().await;
        test_ori(
            "17496/171666".into(),
            "Y is -X 5 -Y 40 -Z and Z is -X 5 -Y 50 Z",
        )
        .await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_GENSEC() -> anyhow::Result<()> {
        init_test_surreal().await;
        test_ori("24384/28745".into(), "Y is -X 31 Y and Z is Z").await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_PLDATU() -> anyhow::Result<()> {
        init_test_surreal().await;

        test_transform(
            "17496/171258".into(),
            "Y is S 3.9973 W and Z is E 3.9973 S",
            "X 9177.61mm Y -641.369mm Z -3520mm",
        )
        .await;
        // test_ori("24384/25786".into(), "Y is -X 0.031 Z and Z is -Y").await;
        // test_ori("17496/268334".into(), "Y is -Z and Z is -X").await;
        //
        // test_ori("25688/48689".into(), "Y is Y 43.307 X and Z is X 43.307 -Y").await;
        // test_ori("25688/48821".into(), "Y is X 33.955 Y and Z is Y 33.955 -X").await;

        //现在问题剩下为，怎么判断z_dir 是 X 还是Y
        // let ydir = parse_expr_to_dir("X 30 Y 30 -Z").unwrap();
        // let ori = cal_ori_by_ydir(ydir, DVec3::Y);
        // dbg!(dquat_to_pdms_ori_xyz_str(&ori, true));
        //
        // let ydir = parse_expr_to_dir("X 30 Y 30 -Z").unwrap();
        // let ori = cal_ori_by_ydir(ydir, DVec3::X);
        // dbg!(dquat_to_pdms_ori_xyz_str(&ori, true));

        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_FIXING() -> anyhow::Result<()> {
        init_test_surreal().await;
        test_ori("17496/268335".into(), "Y is -Z and Z is -X").await;
        // test_ori(
        //     "24384/28753".into(),
        //     "Y is -Y 31 -X 0.031 Z and Z is Y 31 X 89.969 Z",
        // )
        // .await;
        // test_ori("17496/152081".into(), "Y is Z and Z is X 30.659 Y").await;
        // test_ori("24384/25787".into(), "Y is -X 0.031 Z and Z is X 89.969 Z").await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_FIT() -> anyhow::Result<()> {
        init_test_surreal().await;
        // test_transform("24381/55590".into(), "Y is Z and Z is -X", "X 15455.2mm Y -39949.8mm Z 34500mm").await;
        test_transform(
            "17496/496443".into(),
            "Y is -Y and Z is -X",
            "X -16831.5mm Y 96510mm Z 27375mm",
        )
        .await;

        // test_transform(
        //     "17496/215727".into(),
        //     "Y is E and Z is S",
        //     "X -28560mm Y 41050mm Z -5400mm",
        // )
        //     .await;

        //17496/215727
        // test_ori("24381/77311".into(), "Y is -Y 43 X and Z is Z").await;
        // test_ori("17496/202352".into(), "Y is X and Z is -Y").await;
        // test_ori("24381/38388".into(), "Y is -X 13 Y and Z is Y 13 X").await;
        // test_ori("17496/106463".into(), "Y is X 25 -Y and Z is -Y 25 -X").await;
        Ok(())
    }
}

#[tokio::test]
async fn test_query_transform() -> anyhow::Result<()> {
    crate::init_test_surreal().await;

    // //X
    test_print_ori("Y is -X 14 -Y and Z is Y 14 -X");
    // //Y
    test_print_ori("Y is -Y 14 X and Z is -X 14 -Y");
    // //Z
    test_print_ori("Y is Y 14 -X and Z is Z");

    // test_cal_ori(DVec3::X);
    // test_cal_ori(DVec3::NEG_X);
    // test_cal_ori(DVec3::Y);
    // test_cal_ori(DVec3::NEG_Y);
    // test_cal_ori(DVec3::Z);
    // test_cal_ori(DVec3::NEG_Z);
    // //
    // let dir = parse_expr_to_dir("X 45 Y").unwrap();
    // test_cal_ori(dir);

    // let ori = Quat::from_rotation_arc(Vec3::Z, Vec3::X);
    // dbg!(math_tool::quat_to_pdms_ori_xyz_str(&ori));
    //
    // dbg!(math_tool::quat_to_pdms_ori_xyz_str(&Quat::from_rotation_arc(Vec3::Z, Vec3::Y)));

    let transform = rs_surreal::get_world_transform("17496/202374".into())
        .await
        .unwrap()
        .unwrap();
    dbg!(transform);
    let rot_mat = Mat3::from_quat(transform.rotation);
    dbg!(rot_mat);
    // let ori_str = math_tool::to_pdms_ori_xyz_str(&rot_mat);
    // dbg!(&ori_str);

    // let transform = rs_surreal::get_world_transform("24383/89691".into())
    //     .await
    //     .unwrap().unwrap();
    // dbg!(transform);
    // let rot_mat = Mat3::from_quat(transform.rotation);
    // let ori_str = math_tool::to_pdms_ori_xyz_str(&rot_mat);
    // dbg!(&ori_str);

    Ok(())
}

#[tokio::test]
async fn test_query_fixing() -> anyhow::Result<()> {
    crate::init_test_surreal().await;
    let transform = rs_surreal::get_world_transform("25688_43205".into())
        .await
        .unwrap()
        .unwrap();
    dbg!(transform);
    let rot_mat = Mat3::from_quat(transform.rotation);
    // let ori_str = math_tool::to_pdms_ori_xyz_str(&rot_mat);
    // dbg!(&ori_str);
    Ok(())
}

#[tokio::test]
async fn test_query_nearest_along() -> anyhow::Result<()> {
    crate::init_test_surreal().await;
    load_aabb_tree().await.unwrap();
    let nearest = rs_surreal::query_neareast_along_axis("24383/66745".into(), Vec3::NEG_Z, "FLOOR")
        .await
        .unwrap();
    dbg!(nearest);
    // assert_eq!(nearest.to_string().as_str(), "25688_71674");

    let nearest = rs_surreal::query_neareast_along_axis("24383/66771".into(), Vec3::NEG_Z, "FLOOR")
        .await
        .unwrap();
    dbg!(nearest);
    // assert_eq!(nearest.to_string(), "25688_45314");
    Ok(())
}
