use chrono::*;
use serde_derive::*;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io::Write;
use std::ops::Bound;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use str_tools::traits::*;

#[derive(Clone, Debug)]
pub struct Store {
    mp: BTreeMap<String, HistoryItem>,
    recent: Vec<String>,
    pub guesses: Option<Vec<String>>,
    pub pos: Option<usize>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            mp: BTreeMap::new(),
            recent: Vec::new(),
            pos: None,
            guesses: None,
        }
    }

    pub async fn load_history(&mut self, path: &Path) {
        load_history(path, 2, &mut self.mp);
    }

    pub fn push_command(&mut self, cmd: String) -> anyhow::Result<()> {
        let time = SystemTime::now();
        let pwd = std::env::current_dir().unwrap_or(PathBuf::from(""));
        match self.mp.get_mut(&cmd) {
            Some(mut cv) => {
                if !cv.pwds.contains(&pwd) {
                    cv.pwds.push(pwd.clone());
                }
                cv.time = time;
                cv.hits += 1;
                HistorySaver::new(&cmd, &cv).save()?;
            }
            None => {
                let item = HistoryItem {
                    pwds: vec![pwd],
                    time,
                    hits: 1,
                };
                HistorySaver::new(&cmd, &item).save()?;
                self.mp.insert(cmd.clone(), item);
            }
        }

        //Unstable self.recent.drain_filter(|i| i != cmd);
        let mut i = 0;
        while i < self.recent.len() {
            if self.recent[i] == cmd {
                self.recent.remove(i);
            } else {
                i += 1;
            }
        }

        self.recent.push(cmd);
        if self.recent.len() > 200 {
            self.recent.remove(0);
        }
        Ok(())
    }

    pub fn guess(&mut self, cmd: &str) -> bool {
        let mut g = self.cmd_complete(cmd);
        if g.len() == 0 {
            return false;
        }
        let cpwd = std::env::current_dir().unwrap_or(PathBuf::from(""));
        g.sort_by(|(_, a), (_, b)| {
            let mut sca = a.hits;
            if a.pwds.contains(&cpwd) {
                sca += 10;
            }
            let mut scb = b.hits;
            if b.pwds.contains(&cpwd) {
                scb += 10;
            }
            match a.time.cmp(&b.time) {
                Ordering::Greater => sca += 10,
                Ordering::Less => scb += 10,
                _ => {}
            }
            sca.cmp(&scb)
        });
        self.guesses = Some(g.into_iter().map(|(a, _)| a.clone()).collect());
        true
    }

    pub fn cmd_complete<'a>(&'a mut self, cmd: &str) -> Vec<(&'a String, &'a HistoryItem)> {
        if cmd == "" {
            return self.mp.iter().collect();
        }
        //Get exclude str with next char.
        let mut cend = cmd.to_string();
        let ec = cend
            .del_char()
            .and_then(|c| std::char::from_u32((c as u32) + 1));
        match ec {
            Some(c) => cend.push(c),
            None => return Vec::new(),
        }
        self.mp
            .range::<str, _>((Bound::Included(cmd), Bound::Excluded(cend.as_str())))
            .collect()
    }

    pub fn up_recent(&mut self) -> Option<&String> {
        match self.pos {
            Some(n) => self.select_recent(n + 1),
            None => self.select_recent(0),
        }
    }

    pub fn down_recent(&mut self) -> Option<&String> {
        match self.pos {
            Some(0) => {
                self.pos = None;
                None
            }
            Some(n) => self.select_recent(n - 1),
            None => self.select_recent(0),
        }
    }

    pub fn select_recent(&mut self, n: usize) -> Option<&String> {
        let v = match &self.guesses {
            Some(g) => g,
            None => &self.recent,
        };

        let l = v.len();
        if n >= l {
            self.pos = None;
            return None;
        }
        self.pos = Some(n);
        v.get(l - 1 - n)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryItem {
    pwds: Vec<PathBuf>,
    time: SystemTime,
    hits: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct SaveArray<'a> {
    item: Vec<&'a HistorySaver>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LoadArray {
    item: Vec<HistorySaver>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistorySaver {
    cmd: String,
    pwds: Vec<PathBuf>,
    time: SystemTime,
    hits: usize,
}

fn date_to_history_path(t: SystemTime) -> PathBuf {
    let res = history_path();
    let (y, m) = year_month(t);
    on_year_month(&res, y, m)
}
fn history_path() -> PathBuf {
    let mut tdir = PathBuf::from(std::env::var("HOME").unwrap_or(String::new()));
    tdir.push(".config/rushell/history");
    tdir
}

fn on_year_month(p: &Path, y: i32, m: u32) -> PathBuf {
    let dt_s = format!("history_{}_{}.toml", y, m);
    p.join(&dt_s)
}

fn year_month(t: SystemTime) -> (i32, u32) {
    let dt: DateTime<offset::Local> = DateTime::from(t);
    (dt.year(), dt.month())
}

impl HistorySaver {
    pub fn new(cmd: &str, item: &HistoryItem) -> Self {
        HistorySaver {
            cmd: cmd.to_string(),
            pwds: item.pwds.clone(),
            time: item.time,
            hits: item.hits,
        }
    }

    //Currently just append to file and hope for the best.
    pub fn save(&self) -> anyhow::Result<()> {
        let a = SaveArray { item: vec![self] };

        let tdir = date_to_history_path(self.time);
        if let Some(par) = tdir.parent() {
            std::fs::create_dir_all(par)?;
        }

        let tv = toml::Value::try_from(&a)?;
        let s = toml::to_string(&tv)?;

        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .open(&tdir)?;
        write!(f, "{}", &s)?;

        Ok(())
    }
}

pub async fn load_history(path: &Path, months: u32, mp: &mut BTreeMap<String, HistoryItem>) {
    let (y, m) = year_month(SystemTime::now());

    for n in 1..=months {
        let sub = months - n;
        let y2 = y - (sub as i32 / 12);
        let m2 = ((m + 11 - sub as u32) % 12) + 1;
        let p2 = on_year_month(path, y2, m2);

        if let Err(e) = load_history_file(on_year_month(path, y2, m2), mp).await {
            println!(
                "Could not load History file : '{}' because '{}'",
                p2.display(),
                e
            );
        }
    }
}

pub async fn load_history_file<P: AsRef<Path>>(
    path: P,
    mp: &mut BTreeMap<String, HistoryItem>,
) -> anyhow::Result<()> {
    let b = String::from_utf8(tokio::fs::read(path.as_ref()).await?)?;
    let la: LoadArray = toml::from_str(&b)?;
    for i in la.item {
        mp.insert(
            i.cmd,
            HistoryItem {
                pwds: i.pwds,
                hits: i.hits,
                time: i.time,
            },
        );
    }

    Ok(())
}
