use super::download::{DownloadInfo, Downloadable, MultiDownload};
use lazy_static::lazy_static;

const SERVER: &str = "https://ftp.ncbi.nlm.nih.gov/genomes/refseq/vertebrate_mammalian/Homo_sapiens/reference/GCF_000001405.40_GRCh38.p14/";
const MD5FILE: &str = "md5checksums.txt";
const KNOWNFILE: &str = "GCF_000001405.40_GRCh38.p14_knownrefseq_alns.bam";
const MODELFILE: &str = "GCF_000001405.40_GRCh38.p14_modelrefseq_alns.bam";
const LOCALPATH: &str = "downloads/";

lazy_static! {
    pub static ref ALIGNMENTS: Alns = Alns::new(
        BamFile::new(
            KNOWNFILE.to_string(),
            SERVER.to_string() + "RefSeq_transcripts_alignments/",
            LOCALPATH.to_string(),
            AlnType::Known
        ),
        BamFile::new(
            MODELFILE.to_string(),
            SERVER.to_string() + "RefSeq_transcripts_alignments/",
            LOCALPATH.to_string(),
            AlnType::Model
        )
    );
}

pub struct Alns {
    known: BamFile,
    model: BamFile,
}
impl Alns {
    fn new(known: BamFile, model: BamFile) -> Alns {
        Alns {
            known: known,
            model: model,
        }
    }
}
impl MultiDownload for Alns {
    fn download_pool(&self) -> Option<Vec<impl Downloadable>> {
        Some(vec![
            AlnFileType::BAM(&self.known),
            AlnFileType::BAM(&self.model),
            AlnFileType::BAI(&self.known.bai),
            AlnFileType::BAI(&self.model.bai),
        ])
    }
}

struct BaiFile {
    dlinfo: DownloadInfo,
}
impl BaiFile {
    fn new(filename: String, server: String, localpath: String) -> BaiFile {
        let dlinfo = DownloadInfo::new(filename, server, localpath);
        BaiFile { dlinfo: dlinfo }
    }
}
struct BamFile {
    dlinfo: DownloadInfo,
    aln_type: AlnType,
    bai: BaiFile,
}
impl BamFile {
    fn new(filename: String, server: String, localpath: String, aln_type: AlnType) -> BamFile {
        let dlinfo = DownloadInfo::new(filename.clone(), server.clone(), localpath.clone());
        let bai_file = BaiFile::new(filename + ".bai", server, localpath);
        BamFile {
            dlinfo: dlinfo,
            aln_type: aln_type,
            bai: bai_file,
        }
    }
}

impl Downloadable for BamFile {
    fn download_info(&self) -> Option<&DownloadInfo> {
        Some(&self.dlinfo)
    }
}
impl Downloadable for BaiFile {
    fn download_info(&self) -> Option<&DownloadInfo> {
        Some(&self.dlinfo)
    }
}
enum AlnFileType<'a> {
    BAI(&'a BaiFile),
    BAM(&'a BamFile),
}
impl<'a> Downloadable for AlnFileType<'a> {
    fn download_info(&self) -> Option<&DownloadInfo> {
        match self {
            AlnFileType::BAI(fl) => fl.download_info(),
            AlnFileType::BAM(fi) => fi.download_info(),
        }
    }
}
enum AlnType {
    Known,
    Model,
}
