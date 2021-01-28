use chrono::*;
use serde_derive::*;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::ops::Bound;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use str_tools::traits::*;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Clone, Debug)]
pub struct Store {
    mp: BTreeMap<String, HistoryItem>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            mp: BTreeMap::new(),
        }
    }
    pub fn len(&self) -> usize {
        self.mp.len()
    }

    pub async fn load_history(&mut self, path: &Path) {
        load_history(path, 2, &mut self.mp).await;
    }

    pub fn push_command(&mut self, cmd: String, pwd: String) -> anyhow::Result<()> {
        let time = SystemTime::now();
        match self.mp.get_mut(&cmd) {
            Some(cv) => {
                cv.update(time, pwd, true);
            }
            None => {
                let item = HistoryItem {
                    pwds: vec![pwd],
                    time,
                    hits: 1,
                    changed: true,
                };
                self.mp.insert(cmd, item);
            }
        }

        Ok(())
    }

    pub fn guess(&mut self, cmd: &str, pwd: &String) -> Vec<String> {
        let mut g = self.cmd_complete(cmd);
        g.sort_by(|(_, a), (_, b)| {
            let mut sca = a.hits;
            if a.pwds.contains(pwd) {
                sca += 10;
            }
            let mut scb = b.hits;
            if b.pwds.contains(pwd) {
                scb += 10;
            }
            match a.time.cmp(&b.time) {
                Ordering::Greater => sca += 100,
                Ordering::Less => scb += 100,
                _ => {}
            }
            sca.cmp(&scb)
        });
        g.into_iter().map(|(a, _)| a.clone()).collect()
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

    ///self is mut because when something is saved it loses it's "changed" status
    pub async fn save_append<P: AsRef<Path>>(&mut self, fname: P) -> anyhow::Result<()> {
        let mut f = tokio::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(fname.as_ref())
            .await?;
        for (cmd, v) in &mut self.mp {
            if v.changed {
                v.write_to(&mut f, cmd).await?;
                v.changed = false;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryItem {
    pwds: Vec<String>,
    time: SystemTime,
    hits: usize,
    changed: bool,
}

impl HistoryItem {
    fn update(&mut self, time: SystemTime, pwd: String, change: bool) {
        self.time = time;
        if !self.pwds.contains(&pwd) {
            self.pwds.push(pwd);
        }
        self.hits += 1;
        self.changed |= change;
    }

    pub fn save_arr<'a>(&'a self, cmd: &'a str) -> SaveArray<'a> {
        SaveArray {
            item: vec![self.saver(cmd)],
        }
    }
    pub fn saver<'a>(&'a self, cmd: &'a str) -> HistorySaver<'a> {
        HistorySaver {
            cmd,
            pwds: self.pwds.clone(),
            time: self
                .time
                .duration_since(UNIX_EPOCH)
                .map(|a| a.as_secs())
                .unwrap_or(0),
            hits: self.hits,
        }
    }

    pub async fn write_to<W: AsyncWrite + Unpin>(
        &self,
        w: &mut W,
        cmd: &str,
    ) -> anyhow::Result<()> {
        let sc = self.save_arr(cmd);
        let tv = toml::Value::try_from(&sc)?;
        let s = toml::to_string(&tv)?;
        w.write_all(s.as_bytes()).await.map_err(|e| e.into())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct HistorySaver<'a> {
    cmd: &'a str,
    pwds: Vec<String>,
    time: u64,
    hits: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct SaveArray<'a> {
    item: Vec<HistorySaver<'a>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LoadArray {
    item: Vec<HistoryLoader>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HistoryLoader {
    cmd: String,
    pwds: Vec<String>,
    time: u64,
    hits: usize,
}

pub fn date_to_history_path(t: SystemTime) -> PathBuf {
    let res = history_path();
    let (y, m) = year_month(t);
    on_year_month(&res, y, m)
}

pub fn history_path() -> PathBuf {
    let mut tdir = PathBuf::from(std::env::var("HOME").unwrap_or(String::new()));
    tdir.push(".config/rushell/history");
    tdir
}

fn on_year_month(p: &Path, y: i32, m: u32) -> PathBuf {
    let dt_s = format!("s_history_{}_{}.toml", y, m);
    p.join(&dt_s)
}

fn year_month(t: SystemTime) -> (i32, u32) {
    let dt: DateTime<offset::Local> = DateTime::from(t);
    (dt.year(), dt.month())
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
                time: UNIX_EPOCH + Duration::from_secs(i.time),
                changed: false,
            },
        );
    }
    Ok(())
}
