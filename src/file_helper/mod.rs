use std::fs;
use std::path::{Path, PathBuf};

///获得目录下下面的指定project 数据库文件（000结尾）
use std::io;

pub fn collect_db_dirs<'a, T: AsRef<Path>>(
    dir: T,
    projects: impl IntoIterator<Item = &'a str>,
) -> io::Result<Vec<PathBuf>> {
    projects
        .into_iter()
        .filter_map(|x| {
            let path = dir.as_ref().to_path_buf().join(x);
            match fs::read_dir(&path) {
                Ok(entries) => entries
                    .into_iter()
                    .filter_map(|entry| match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            if path.is_dir()
                                && path.file_name().unwrap().to_str().unwrap().ends_with("000")
                            {
                                Some(Ok(path))
                            } else {
                                None
                            }
                        }
                        Err(e) => Some(Err(e)),
                    })
                    .next(),
                Err(e) => Some(Err(e)),
            }
        })
        .collect()
}

// #[inline]
// pub fn collect_db_dirs<'a, T: AsRef<Path>>(dir: T, projects: impl IntoIterator<Item=&'a str>) -> Vec<PathBuf> {
//     projects.into_iter().filter_map(|x| {
//         fs::read_dir(dir.as_ref().to_path_buf().join(x))
//             .unwrap()
//             .into_iter()
//             .map(|entry| entry.unwrap().path())
//             .find(|x| x.is_dir() && x.file_name().unwrap().to_str().unwrap().ends_with("000"))
//     }).collect()
// }
