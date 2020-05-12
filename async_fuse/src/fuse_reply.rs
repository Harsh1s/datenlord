use anyhow;
use libc::{S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFREG, S_IFSOCK};
use log::{debug, error};
use nix::sys::stat::SFlag;
use nix::sys::uio::{self, IoVec};
use smol::{self, Task};
use std::convert::AsRef;
use std::ffi::OsStr;
use std::marker::PhantomData;
use std::os::raw::c_int;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{mem, ptr, slice};

use super::protocal::*;

fn time_from_system_time(system_time: &SystemTime) -> (u64, u32) {
    let duration = system_time
        .duration_since(UNIX_EPOCH)
        .expect("failed to convert SystemTime to Duration");
    (duration.as_secs(), duration.subsec_nanos())
}

// TODO: remove it
fn mode_from_kind_and_perm(kind: SFlag, perm: u16) -> u32 {
    (match kind {
        SFlag::S_IFIFO => S_IFIFO,
        SFlag::S_IFCHR => S_IFCHR,
        SFlag::S_IFBLK => S_IFBLK,
        SFlag::S_IFDIR => S_IFDIR,
        SFlag::S_IFREG => S_IFREG,
        SFlag::S_IFLNK => S_IFLNK,
        SFlag::S_IFSOCK => S_IFSOCK,
        _ => unreachable!(),
    }) as u32
        | perm as u32
}

#[derive(Debug)]
enum ToBytes<T> {
    Struct(T),
    Bytes(Vec<u8>),
    Null,
}

#[derive(Debug)]
struct ReplyRaw<T: Send + Sync + 'static> {
    unique: u64,
    fd: RawFd,
    marker: PhantomData<T>,
}

impl<T: Send + Sync + 'static> ReplyRaw<T> {
    fn new(unique: u64, fd: RawFd) -> Self {
        Self {
            unique,
            fd,
            marker: PhantomData,
        }
    }

    async fn send(self, to_bytes: ToBytes<T>, err: c_int) -> anyhow::Result<usize> {
        let fd = self.fd;
        let wsize = Task::blocking(async move {
            let instance: T; // to hold the instance of ToBytes::Struct
            let byte_vec: Vec<u8>; // to hold the Vec<u8> of ToBytes::Bytes
            let empty_vec: Vec<u8>; // to hold the emtpy Vec<u8> of ToBytes::Null
            let (data_len, bytes) = match to_bytes {
                ToBytes::Struct(inst) => {
                    instance = inst;
                    let len = mem::size_of::<T>();
                    let bytes = match len {
                        0 => &[],
                        len => {
                            let p = &instance as *const T as *const u8;
                            unsafe { slice::from_raw_parts(p, len) }
                        }
                    };
                    (len, bytes)
                }
                ToBytes::Bytes(bv) => {
                    byte_vec = bv;
                    (byte_vec.len(), &byte_vec[..])
                }
                ToBytes::Null => {
                    empty_vec = Vec::new();
                    (0, &empty_vec[..])
                }
            };
            let header_len = mem::size_of::<FuseOutHeader>();
            let header = FuseOutHeader {
                len: (header_len + data_len) as u32,
                error: -err, // FUSE requires the error number to be negative
                unique: self.unique,
            };
            let h = &header as *const FuseOutHeader as *const u8;
            let header_bytes = unsafe { slice::from_raw_parts(h, header_len) };
            let iovecs: Vec<_> = if data_len > 0 {
                debug_assert_eq!(err, 0);
                vec![IoVec::from_slice(header_bytes), IoVec::from_slice(bytes)]
            } else {
                debug_assert_ne!(err, 0);
                vec![IoVec::from_slice(header_bytes)]
            };
            uio::writev(fd, &iovecs)
        })
        .await?;

        Ok(wsize)
    }

    async fn send_bytes(self, byte_vec: Vec<u8>) {
        match self.send(ToBytes::Bytes(byte_vec), 0).await {
            Ok(wsize) => {
                debug!("sent {} bytes successfully", wsize);
            }
            Err(err) => {
                error!("failed to send bytes, the error is: {}", err);
            }
        }
    }

    async fn send_data(self, instance: T) {
        match self.send(ToBytes::Struct(instance), 0).await {
            Ok(wsize) => {
                debug!("sent {} bytes data successfully", wsize);
            }
            Err(err) => {
                error!("failed to send data, the error is: {}", err);
            }
        }
    }

    async fn error(self, err: c_int) {
        match self.send(ToBytes::Null, err).await {
            Ok(wsize) => {
                debug!("sent {} bytes error successfully", wsize);
            }
            Err(err) => {
                error!("failed to send error, the error is: {}", err);
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct ReplyInit {
    reply: ReplyRaw<FuseInitOut>,
}

impl ReplyInit {
    pub fn new(unique: u64, fd: RawFd) -> ReplyInit {
        ReplyInit {
            reply: ReplyRaw::new(unique, fd),
        }
    }
    pub async fn init(
        self,
        major: u32,
        minor: u32,
        max_readahead: u32,
        flags: u32,
        #[cfg(not(feature = "abi-7-13"))] unused: u32,
        #[cfg(feature = "abi-7-13")] max_background: u16,
        #[cfg(feature = "abi-7-13")] congestion_threshold: u16,
        max_write: u32,
        #[cfg(feature = "abi-7-23")] time_gran: u32,
        #[cfg(all(feature = "abi-7-23", not(feature = "abi-7-28")))] unused: [u32; 9],
        #[cfg(feature = "abi-7-28")] max_pages: u16,
        #[cfg(feature = "abi-7-28")] padding: u16,
        #[cfg(feature = "abi-7-28")] unused: [u32; 8],
    ) {
        self.reply
            .send_data(FuseInitOut {
                major,
                minor,
                max_readahead,
                flags,
                #[cfg(not(feature = "abi-7-13"))]
                unused,
                #[cfg(feature = "abi-7-13")]
                max_background,
                #[cfg(feature = "abi-7-13")]
                congestion_threshold,
                max_write,
                #[cfg(feature = "abi-7-23")]
                time_gran,
                #[cfg(all(feature = "abi-7-23", not(feature = "abi-7-28")))]
                unused,
                #[cfg(feature = "abi-7-28")]
                max_pages,
                #[cfg(feature = "abi-7-28")]
                padding,
                #[cfg(feature = "abi-7-28")]
                unused,
            })
            .await;
    }
    pub async fn error(self, err: c_int) {
        self.reply.error(err).await;
    }
}

#[derive(Debug)]
pub(crate) struct ReplyEmpty {
    reply: ReplyRaw<()>,
}

impl ReplyEmpty {
    pub fn new(unique: u64, fd: RawFd) -> ReplyEmpty {
        ReplyEmpty {
            reply: ReplyRaw::new(unique, fd),
        }
    }
    pub async fn ok(self) {
        self.reply.send_data(()).await;
    }
    pub async fn error(self, err: c_int) {
        self.reply.error(err).await;
    }
}

#[derive(Debug)]
pub(crate) struct ReplyData {
    reply: ReplyRaw<Vec<u8>>,
}

impl ReplyData {
    pub fn new(unique: u64, fd: RawFd) -> ReplyData {
        ReplyData {
            reply: ReplyRaw::new(unique, fd),
        }
    }
    pub async fn data(self, bytes: Vec<u8>) {
        self.reply.send_bytes(bytes).await;
    }
    pub async fn error(self, err: c_int) {
        self.reply.error(err).await;
    }
}

#[derive(Debug)]
pub(crate) struct ReplyEntry {
    reply: ReplyRaw<FuseEntryOut>,
}

impl ReplyEntry {
    pub fn new(unique: u64, fd: RawFd) -> ReplyEntry {
        ReplyEntry {
            reply: ReplyRaw::new(unique, fd),
        }
    }
    /// Reply to a request with the given entry
    pub async fn entry(self, ttl: &Duration, attr: FuseAttr, generation: u64) {
        self.reply
            .send_data(FuseEntryOut {
                nodeid: attr.ino,
                generation: generation,
                entry_valid: ttl.as_secs(),
                attr_valid: ttl.as_secs(),
                entry_valid_nsec: ttl.subsec_nanos(),
                attr_valid_nsec: ttl.subsec_nanos(),
                attr,
            })
            .await;
    }

    /// Reply to a request with the given error code
    pub async fn error(self, err: c_int) {
        self.reply.error(err).await;
    }
}

#[derive(Debug)]
pub(crate) struct ReplyAttr {
    reply: ReplyRaw<FuseAttrOut>,
}

impl ReplyAttr {
    pub fn new(unique: u64, fd: RawFd) -> ReplyAttr {
        ReplyAttr {
            reply: ReplyRaw::new(unique, fd),
        }
    }
    /// Reply to a request with the given attribute
    pub async fn attr(self, ttl: &Duration, attr: FuseAttr) {
        self.reply
            .send_data(FuseAttrOut {
                attr_valid: ttl.as_secs(),
                attr_valid_nsec: ttl.subsec_nanos(),
                dummy: 0,
                attr,
            })
            .await;
    }

    /// Reply to a request with the given error code
    pub async fn error(self, err: c_int) {
        self.reply.error(err).await;
    }
}

#[cfg(target_os = "macos")]
#[derive(Debug)]
pub(crate) struct ReplyXTimes {
    reply: ReplyRaw<FuseGetXTimesOut>,
}

#[cfg(target_os = "macos")]
impl ReplyXTimes {
    pub fn new(unique: u64, fd: RawFd) -> ReplyXTimes {
        ReplyXTimes {
            reply: ReplyRaw::new(unique, fd),
        }
    }
    /// Reply to a request with the given xtimes
    pub async fn xtimes(self, bkuptime: SystemTime, crtime: SystemTime) {
        let (bkuptime_secs, bkuptime_nanos) = time_from_system_time(&bkuptime);
        let (crtime_secs, crtime_nanos) = time_from_system_time(&crtime);
        self.reply
            .send_data(FuseGetXTimesOut {
                bkuptime: bkuptime_secs,
                crtime: crtime_secs,
                bkuptimensec: bkuptime_nanos,
                crtimensec: crtime_nanos,
            })
            .await;
    }

    /// Reply to a request with the given error code
    pub async fn error(self, err: c_int) {
        self.reply.error(err).await;
    }
}

#[derive(Debug)]
pub(crate) struct ReplyOpen {
    reply: ReplyRaw<FuseOpenOut>,
}

impl ReplyOpen {
    pub fn new(unique: u64, fd: RawFd) -> ReplyOpen {
        ReplyOpen {
            reply: ReplyRaw::new(unique, fd),
        }
    }
    /// Reply to a request with the given open result
    pub async fn opened(self, fh: u64, flags: u32) {
        self.reply
            .send_data(FuseOpenOut {
                fh: fh,
                open_flags: flags,
                padding: 0,
            })
            .await;
    }

    /// Reply to a request with the given error code
    pub async fn error(self, err: c_int) {
        self.reply.error(err).await;
    }
}

#[derive(Debug)]
pub(crate) struct ReplyWrite {
    reply: ReplyRaw<FuseWriteOut>,
}

impl ReplyWrite {
    pub fn new(unique: u64, fd: RawFd) -> ReplyWrite {
        ReplyWrite {
            reply: ReplyRaw::new(unique, fd),
        }
    }
    /// Reply to a request with the given open result
    pub async fn written(self, size: u32) {
        self.reply
            .send_data(FuseWriteOut {
                size: size,
                padding: 0,
            })
            .await;
    }

    /// Reply to a request with the given error code
    pub async fn error(self, err: c_int) {
        self.reply.error(err).await;
    }
}

#[derive(Debug)]
pub(crate) struct ReplyStatFs {
    reply: ReplyRaw<FuseStatFsOut>,
}

impl ReplyStatFs {
    pub fn new(unique: u64, fd: RawFd) -> ReplyStatFs {
        ReplyStatFs {
            reply: ReplyRaw::new(unique, fd),
        }
    }

    pub async fn statfs(
        self,
        blocks: u64,
        bfree: u64,
        bavail: u64,
        files: u64,
        ffree: u64,
        bsize: u32,
        namelen: u32,
        frsize: u32,
    ) {
        self.reply
            .send_data(FuseStatFsOut {
                st: FuseKStatFs {
                    blocks: blocks,
                    bfree: bfree,
                    bavail: bavail,
                    files: files,
                    ffree: ffree,
                    bsize: bsize,
                    namelen: namelen,
                    frsize: frsize,
                    padding: 0,
                    spare: [0; 6],
                },
            })
            .await;
    }

    /// Reply to a request with the given error code
    pub async fn error(self, err: c_int) {
        self.reply.error(err).await;
    }
}

#[derive(Debug)]
pub(crate) struct ReplyCreate {
    reply: ReplyRaw<(FuseEntryOut, FuseOpenOut)>,
}

impl ReplyCreate {
    pub fn new(unique: u64, fd: RawFd) -> ReplyCreate {
        ReplyCreate {
            reply: ReplyRaw::new(unique, fd),
        }
    }
    /// Reply to a request with the given entry
    pub async fn created(
        self,
        ttl: &Duration,
        attr: FuseAttr,
        generation: u64,
        fh: u64,
        flags: u32,
    ) {
        self.reply
            .send_data((
                FuseEntryOut {
                    nodeid: attr.ino,
                    generation: generation,
                    entry_valid: ttl.as_secs(),
                    attr_valid: ttl.as_secs(),
                    entry_valid_nsec: ttl.subsec_nanos(),
                    attr_valid_nsec: ttl.subsec_nanos(),
                    attr,
                },
                FuseOpenOut {
                    fh: fh,
                    open_flags: flags,
                    padding: 0,
                },
            ))
            .await;
    }

    /// Reply to a request with the given error code
    pub async fn error(self, err: c_int) {
        self.reply.error(err).await;
    }
}

#[derive(Debug)]
pub(crate) struct ReplyLock {
    reply: ReplyRaw<FuseLockOut>,
}

impl ReplyLock {
    pub fn new(unique: u64, fd: RawFd) -> ReplyLock {
        ReplyLock {
            reply: ReplyRaw::new(unique, fd),
        }
    }
    /// Reply to a request with the given open result
    pub async fn locked(self, start: u64, end: u64, typ: u32, pid: u32) {
        self.reply
            .send_data(FuseLockOut {
                lk: FuseFileLock {
                    start: start,
                    end: end,
                    typ: typ,
                    pid: pid,
                },
            })
            .await;
    }

    /// Reply to a request with the given error code
    pub async fn error(self, err: c_int) {
        self.reply.error(err).await;
    }
}

#[derive(Debug)]
pub(crate) struct ReplyBMap {
    reply: ReplyRaw<FuseBMapOut>,
}

impl ReplyBMap {
    pub fn new(unique: u64, fd: RawFd) -> ReplyBMap {
        ReplyBMap {
            reply: ReplyRaw::new(unique, fd),
        }
    }
    /// Reply to a request with the given open result
    pub async fn bmap(self, block: u64) {
        self.reply.send_data(FuseBMapOut { block: block }).await;
    }

    /// Reply to a request with the given error code
    pub async fn error(self, err: c_int) {
        self.reply.error(err).await;
    }
}

#[derive(Debug)]
pub(crate) struct ReplyDirectory {
    reply: ReplyRaw<()>,
    data: Vec<u8>,
}

impl ReplyDirectory {
    /// Creates a new ReplyDirectory with a specified buffer size.
    pub fn new(unique: u64, fd: RawFd, size: usize) -> ReplyDirectory {
        ReplyDirectory {
            reply: ReplyRaw::new(unique, fd),
            data: Vec::with_capacity(size),
        }
    }

    /// Add an entry to the directory reply buffer. Returns true if the buffer is full.
    /// A transparent offset value can be provided for each entry. The kernel uses these
    /// value to request the next entries in further readdir calls
    pub fn add<T: AsRef<OsStr>>(&mut self, ino: u64, offset: i64, kind: SFlag, name: T) -> bool {
        let name = name.as_ref().as_bytes();
        let entlen = mem::size_of::<FuseDirEnt>() + name.len();
        let entsize = (entlen + mem::size_of::<u64>() - 1) & !(mem::size_of::<u64>() - 1); // 64bit align
        let padlen = entsize - entlen;
        if self.data.len() + entsize > self.data.capacity() {
            return true;
        }
        unsafe {
            let p = self.data.as_mut_ptr().offset(self.data.len() as isize);
            let pdirent: *mut FuseDirEnt = mem::transmute(p);
            (*pdirent).ino = ino;
            (*pdirent).off = offset as u64;
            (*pdirent).namelen = name.len() as u32;
            (*pdirent).typ = mode_from_kind_and_perm(kind, 0) >> 12;
            let p = p.offset(mem::size_of_val(&*pdirent) as isize);
            ptr::copy_nonoverlapping(name.as_ptr(), p, name.len());
            let p = p.offset(name.len() as isize);
            ptr::write_bytes(p, 0u8, padlen);
            let newlen = self.data.len() + entsize;
            self.data.set_len(newlen);
        }
        false
    }

    /// Reply to a request with the filled directory buffer
    pub async fn ok(self) {
        self.reply.send_bytes(self.data).await;
    }

    /// Reply to a request with the given error code
    pub async fn error(self, err: c_int) {
        self.reply.error(err).await;
    }
}

#[derive(Debug)]
pub(crate) struct ReplyXAttr {
    reply: ReplyRaw<FuseGetXAttrOut>,
}

impl ReplyXAttr {
    pub fn new(unique: u64, fd: RawFd) -> ReplyXAttr {
        ReplyXAttr {
            reply: ReplyRaw::new(unique, fd),
        }
    }
    /// Reply to a request with the size of the xattr.
    pub async fn size(self, size: u32) {
        self.reply
            .send_data(FuseGetXAttrOut {
                size: size,
                padding: 0,
            })
            .await;
    }

    /// Reply to a request with the data in the xattr.
    pub async fn data(self, bytes: Vec<u8>) {
        self.reply.send_bytes(bytes).await;
    }

    /// Reply to a request with the given error code.
    pub async fn error(self, err: c_int) {
        self.reply.error(err).await;
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_slice() {
        let s = [1, 2, 3, 4, 5, 6];
        let v = s.to_owned();
        println!("{:?}", v);
        let v1 = s.to_vec();
        println!("{:?}", v1);

        let s1 = [1, 2, 3];
        let s2 = [4, 5, 6];
        let s3 = [7, 8, 9];
        let l1 = [&s1];
        let l2 = [&s2, &s3];
        let mut v1 = l1.to_vec();
        v1.extend(&l2);

        println!("{:?}", l1);
        println!("{:?}", v1);
    }
}
