extern crate serde_json;

use std::path::Path;
use std::fs::File;
use std::ops::{Deref, DerefMut};
use std::collections::BTreeSet;
use std::cmp::{Ordering};
use fnv::FnvHashMap;
use regex::Regex;
use chrono::{Local, DateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Info {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub subtitle: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub author: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub year: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub language: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub publisher: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub series: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub edition: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub volume: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub number: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub isbn: String, // International Standard Book Number
    // #[serde(skip_serializing_if = "String::is_empty")]
    // pub issn: String, // International Standard Serial Number
    // #[serde(skip_serializing_if = "String::is_empty")]
    // pub ismn: String, // International Standard Music Number
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub keywords: BTreeSet<String>,
    pub file: FileInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reader: Option<ReaderInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FileInfo {
    pub path: String,
    pub kind: String,
    pub size: u64,
}

impl Default for FileInfo {
    fn default() -> Self {
        FileInfo {
            path: String::default(),
            kind: String::default(),
            size: u64::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ReaderInfo {
    pub opened: DateTime<Local>,
    pub last_page: usize,
    pub pages_count: usize,
    pub columns: u8,
}

impl Default for ReaderInfo {
    fn default() -> Self {
        ReaderInfo {
            opened: Local::now(),
            last_page: 0,
            pages_count: 0,
            columns: 1,
        }
    }
}

impl Default for Info {
    fn default() -> Self {
        Info {
            title: String::default(),
            subtitle: String::default(),
            author: String::default(),
            year: String::default(),
            language: String::default(),
            publisher: String::default(),
            series: String::default(),
            edition: String::default(),
            volume: String::default(),
            number: String::default(),
            isbn: String::default(),
            keywords: BTreeSet::new(),
            file: FileInfo::default(),
            reader: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata(pub Vec<Info>);

impl Deref for Metadata {
    type Target = Vec<Info>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Metadata {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Metadata {
    pub fn load<P: AsRef<Path>>(path: P) -> Metadata {
        let reader = File::open(path).unwrap();
        serde_json::from_reader(reader).unwrap()
    }

    pub fn keywords(&self) -> BTreeSet<String> {
        self.0.iter().flat_map(|info| info.keywords.clone()).collect()
    }
}

fn sort_opened(i1: &Info, i2: &Info) -> Ordering {
    match (&i1.reader, &i2.reader) {
        (&None, &None) => Ordering::Equal,
        (&None, &Some(_)) => Ordering::Less,
        (&Some(_), &None) => Ordering::Greater,
        (&Some(ref r1), &Some(ref r2)) => r1.opened.cmp(&r2.opened),
    }
}

fn combine_sort_methods<'a, T, F1, F2>(mut f1: F1, mut f2: F2) -> Box<FnMut(&T, &T) -> Ordering + 'a>
where F1: FnMut(&T, &T) -> Ordering + 'a,
      F2: FnMut(&T, &T) -> Ordering + 'a {
    Box::new(move |x, y| {
        match f1(x, y) {
            ord @ Ordering::Less | ord @ Ordering::Greater => ord,
            Ordering::Equal => f2(x, y),
        }
    })
}

lazy_static! {
    pub static ref TITLE_PREFIXES: FnvHashMap<&'static str, Regex> = {
        let mut p = FnvHashMap::default();
        p.insert("english", Regex::new(r"^(The|An?)\s").unwrap());
        p.insert("french", Regex::new(r"^(Les?\s|La\s|L['’]|Une?\s|Des?\s|Du\s)").unwrap());
        p
    };
}
