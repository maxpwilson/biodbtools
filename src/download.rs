use anyhow::{Result, anyhow};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Client;
use std::fs::remove_file;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

pub trait MultiDownload {
    fn download_pool(&self) -> Option<Vec<impl Downloadable>>;
}
pub async fn download_all<M: MultiDownload>(pool: &M, verbose: Verbose) -> Result<()> {
    let mut handles = vec![];
    let dls = match pool.download_pool() {
        Some(dl) => dl,
        _ => return Err(anyhow!("No download pool")),
    };
    let mp = match verbose {
        Verbose::Int(m) => Arc::new(Mutex::new(m)),
        _ => Arc::new(Mutex::new(MultiProgress::new())),
    };
    for dl in dls {
        let inner_mp = Arc::clone(&mp);
        let inner_verbose = verbose.clone();
        handles.push(tokio::task::spawn(async move {
            let pb = inner_mp.lock().unwrap().add(ProgressBar::new(0));
            let ipb = match inner_verbose {
                Verbose::Quiet => Verbose::Quiet,
                _ => Verbose::Ext(pb),
            };
            download(&dl, Verbose::Quiet);
        }));
    }
    futures::future::join_all(handles).await;
    mp.lock();
    Ok(())
}

pub trait Downloadable {
    fn download_info(&self) -> Option<&DownloadInfo>;
    fn pb_style(&self) -> ProgressStyle {
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
        ).unwrap()
        .progress_chars("##-")
    }
    fn localfile(&self) -> Option<String> {
        match self.download_info() {
            Some(dl) => dl.localfile(),
            _ => None,
        }
    }
    fn serverfile(&self) -> Option<String> {
        match self.download_info() {
            Some(dl) => dl.serverfile(),
            _ => None,
        }
    }
    fn is_local(&self) -> Option<LocalFile> {
        match self.download_info() {
            Some(dl) => dl.is_local(),
            _ => None,
        }
    }
    fn remove_local(&self) -> Result<()> {
        match self.download_info() {
            Some(dl) => dl.remove_local(),
            _ => Err(anyhow!("No DL Info")),
        }
    }
}

pub async fn download<D: Downloadable>(dl: &D, verbose: Verbose) -> Result<()> {
    let client = Client::new();
    let serverpath = match dl.serverfile() {
        Some(serverpath) => serverpath,
        _ => return Err(anyhow!("Failed to get server path")),
    };
    let localfilepath = match dl.localfile() {
        Some(localfilepath) => localfilepath,
        _ => return Err(anyhow!("Failed to get local path")),
    };
    let mut response = client.get(serverpath).send().await?;
    let download_size = match response.content_length() {
        Some(ds) => ds,
        _ => 0,
    };
    let pb = match verbose {
        Verbose::Ext(pb) => {
            pb.set_style(dl.pb_style());
            Some(pb)
        }
        Verbose::Loud => {
            let p = ProgressBar::new(download_size);
            p.set_style(dl.pb_style());
            Some(p)
        }
        _ => None,
    };
    let file = File::create(localfilepath).await?;
    let mut writer = BufWriter::new(file);
    while let Some(chunk) = response.chunk().await? {
        writer.write(&chunk).await?;
        match pb.as_ref() {
            Some(p) => p.inc(chunk.len() as u64),
            _ => (),
        };
    }
    writer.flush().await?;
    match pb.as_ref() {
        Some(p) => p.finish(),
        _ => (),
    };
    Ok(())
}
pub trait Checked: Downloadable {
    fn check(&self) -> Result<()>;
}

#[derive(Clone)]
pub struct DownloadInfo {
    pub filename: String,
    pub server: String,
    pub localpath: String,
}
impl DownloadInfo {
    pub fn new(filename: String, server: String, localpath: String) -> DownloadInfo {
        DownloadInfo {
            filename: filename,
            server: server,
            localpath: localpath,
        }
    }
    fn localfile(&self) -> Option<String> {
        Some(self.localpath.clone() + &self.filename)
    }
    fn serverfile(&self) -> Option<String> {
        Some(self.server.clone() + &self.filename)
    }
    fn is_local(&self) -> Option<LocalFile> {
        if Path::new(&self.localfile().unwrap()).exists() {
            return Some(LocalFile::Exists);
        }
        Some(LocalFile::None)
    }
    fn remove_local(&self) -> Result<()> {
        let localfile = match self.localfile() {
            Some(localfile) => localfile,
            _ => String::new(),
        };
        match self.is_local() {
            Some(lf) => match lf {
                LocalFile::Exists => {
                    remove_file(localfile)?;
                    Ok(())
                }
                LocalFile::None => Ok(()),
            },
            _ => Err(anyhow!("Failed to calc is_local")),
        }
    }
}

pub enum Checksum {
    None,
    Hash(String),
}
pub enum LocalFile {
    Exists,
    None,
}
#[derive(Clone)]
enum Verbose {
    Quiet,
    Loud,
    Ext(ProgressBar),
    Int(MultiProgress),
}
