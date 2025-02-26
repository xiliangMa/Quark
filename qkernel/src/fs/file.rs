// Copyright (c) 2021 Quark Container Authors / 2018 The gVisor Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;
use ::qlib::mutex::*;
use alloc::string::String;
use alloc::string::ToString;
use core::any::Any;
use core::ops::Deref;

use super::super::uid::*;
use super::super::kernel::waiter::qlock::*;
use super::super::kernel::time::*;
use super::super::qlib::metric::*;
use super::super::qlib::common::*;
use super::super::qlib::auth::*;
use super::super::qlib::range::*;
use super::super::kernel::waiter::*;
use super::super::qlib::linux_def::*;
//use super::super::socket::unix::transport::unix::*;
use super::super::task::*;
use super::super::memmgr::*;
use super::super::fs::flags::*;
use super::super::fs::host::hostfileop::*;
use super::super::tcpip::tcpip::*;
use super::super::kernel::fasync::*;
use super::super::qlib::singleton::*;

use super::attr::*;
use super::dirent::*;
//use super::flags::*;
use super::dentry::*;
use super::inode::*;
use super::mount::*;
use super::filesystems::*;
use super::host::fs::*;
use super::host::tty::*;
use super::host::util::*;
use super::host::hostinodeop::*;

pub static READS : Singleton<Arc<U64Metric>> = Singleton::<Arc<U64Metric>>::New();

pub unsafe fn InitSingleton() {
    READS.Init(NewU64Metric("/fs/reads", false, "Number of file reads."));
}

// SpliceOpts define how a splice works.
#[derive(Default, Clone, Copy, Debug)]
pub struct SpliceOpts {
    // Length is the length of the splice operation.
    pub Length: i64,

    // SrcOffset indicates whether the existing source file offset should
    // be used. If this is true, then the Start value below is used.
    //
    // When passed to FileOperations object, this should always be true as
    // the offset will be provided by a layer above, unless the object in
    // question is a pipe or socket. This value can be relied upon for such
    // an indicator.
    pub SrcOffset: bool,

    // SrcStart is the start of the source file. This is used only if
    // SrcOffset is false.
    pub SrcStart: i64,

    // Dup indicates that the contents should not be consumed from the
    // source (e.g. in the case of a socket or a pipe), but duplicated.
    pub Dup: bool,

    // DstOffset indicates that the destination file offset should be used.
    //
    // See SrcOffset for additional information.
    pub DstOffset: bool,

    // DstStart is the start of the destination file. This is used only if
    // DstOffset is false.
    pub DstStart: i64,
}

pub const FILE_MAX_OFFSET: i64 = core::i64::MAX;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SyncType {
    SyncAll,
    SyncData,
    SyncBackingStorage,
}

pub trait SockOperations: Sync + Send {
    fn Connect(&self, _task: &Task, _socketaddr: &[u8], _blocking: bool) -> Result<i64> {
        return Err(Error::SysError(SysErr::ENOTSOCK))
    }

    fn Accept(&self, _task: &Task, _addr: &mut [u8], _addrlen: &mut u32, _flags: i32, _blocking: bool) -> Result<i64> {
        return Err(Error::SysError(SysErr::ENOTSOCK))
    }

    fn Bind(&self, _task: &Task, _sockaddr: &[u8]) -> Result<i64> {
        return Err(Error::SysError(SysErr::ENOTSOCK))
    }

    fn Listen(&self, _task: &Task, _backlog: i32) -> Result<i64> {
        return Err(Error::SysError(SysErr::ENOTSOCK))
    }

    fn Shutdown(&self, _task: &Task, _how: i32) -> Result<i64> {
        return Err(Error::SysError(SysErr::ENOTSOCK))
    }

    fn GetSockOpt(&self, _task: &Task, _level: i32, _name: i32, _addr: &mut [u8]) -> Result<i64> {
        return Err(Error::SysError(SysErr::ENOTSOCK))
    }

    fn SetSockOpt(&self, _task: &Task, _level: i32, _name: i32, _opt: &[u8]) -> Result<i64> {
        return Err(Error::SysError(SysErr::ENOTSOCK))
    }

    fn GetSockName(&self, _task: &Task, _socketaddr: &mut [u8]) -> Result<i64> {
        return Err(Error::SysError(SysErr::ENOTSOCK))
    }

    fn GetPeerName(&self, _task: &Task, _socketaddr: &mut [u8]) -> Result<i64> {
        return Err(Error::SysError(SysErr::ENOTSOCK))
    }

    //return (receive bytes, msgFlags, (senderAddr, senderAddrLen), controlMessages)
    fn RecvMsg(&self, _task: &Task, _dst: &mut [IoVec], _flags: i32, _deadline: Option<Time>, _senderRequested: bool, _controlDataLen: usize)
        -> Result<(i64, i32, Option<(SockAddr, usize)>, Vec<u8>)> {
        return Err(Error::SysError(SysErr::ENOTSOCK))
    }

    fn SendMsg(&self, _task: &Task, _src: &[IoVec], _flags: i32, _msgHdr: &mut MsgHdr, _deadline: Option<Time>) -> Result<i64> {
        return Err(Error::SysError(SysErr::ENOTSOCK))
    }

    fn SetRecvTimeout(&self, _nanoseconds: i64) {
        return
    }

    fn RecvTimeout(&self) -> i64 {
        return 0
    }

    fn SetSendTimeout(&self, _nanoseconds: i64) {
        return
    }

    fn SendTimeout(&self) -> i64 {
        return 0
    }
}

pub trait SpliceOperations {
    fn WriteTo(&self, _task: &Task, file: &File, dst: &File, opts: &SpliceOpts) -> Result<i64> {
        if opts.SrcOffset && !file.FileOp.Seekable() {
            return Err(Error::SysError(SysErr::EINVAL))
        }

        if opts.DstOffset && !dst.FileOp.Seekable() {
            return Err(Error::SysError(SysErr::EINVAL))
        }

        return Err(Error::SysError(SysErr::ENOSYS))
    }

    fn ReadFrom(&self, _task: &Task, file: &File, src: &File, opts: &SpliceOpts) -> Result<i64> {
        if opts.DstOffset && !file.FileOp.Seekable() {
            return Err(Error::SysError(SysErr::EINVAL))
        }

        if opts.SrcOffset && !src.FileOp.Seekable() {
            return Err(Error::SysError(SysErr::EINVAL))
        }

        return Err(Error::SysError(SysErr::ENOSYS))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum FileOpsType {
    OverlayFileOperations,
    MockFileOperations,
    TimerOperations,
    FullFileOperations,
    NullFileOperations,
    RandomFileOperations,
    TTYFileOperations,
    ZeroFileOperations,
    FileOptionsUtil,
    NoReadWriteFile,
    StaticDirFileOperations,
    StaticFile,
    HostFileOp,
    TTYFileOps,
    RootProcFile,
    SeqFileOperations,
    DirFileOperation,
    SocketFileOps,
    SymlinkFileOperations,
    DirFileOperations,
    MasterFileOperations,
    SlaveFileOperations,
    EventOperations,
    EventPoll,
    Reader,
    ReaderWriter,
    Writer,
    SocketOperations,
    UnixSocketOperations,
    ReadonlyFileOperations,
    DynamicDirFileOperations,
    SignalOperation,
}

pub trait FileOperations: Sync + Send + Waitable + SockOperations + SpliceOperations {
    fn as_any(&self) -> &Any;
    fn FopsType(&self) -> FileOpsType;
    fn Seekable(&self) -> bool;

    fn Seek(&self, task: &Task, f: &File, whence: i32, current: i64, offset: i64) -> Result<i64>;
    fn ReadDir(&self, task: &Task, f: &File, offset: i64, serializer: &mut DentrySerializer) -> Result<i64>;
    fn ReadAt(&self, task: &Task, f: &File, dsts: &mut [IoVec], offset: i64, _blocking: bool) -> Result<i64>;
    fn WriteAt(&self, task: &Task, f: &File, srcs: &[IoVec], offset: i64, _blocking: bool) -> Result<i64>;

    // atomic operation to append data to seekable file. return (write size, current file len).
    /*
    fn Append(&self, _task: &Task, _f: &File, _srcs: &[IoVec]) -> Result<(i64, i64)> {
        return Err(Error::SysError(SysErr::ESPIPE))
    }

    fn Append(&self, task: &Task, f: &File, srcs: &[IoVec]) -> Result<(i64, i64)> {
        let n = self.WriteAt(task, f, srcs, 0, false)?;
        return Ok((n, 0))
    }

    */

    fn Append(&self, _task: &Task, _f: &File, _srcs: &[IoVec]) -> Result<(i64, i64)>;

    fn Fsync(&self, task: &Task, f: &File, start: i64, end: i64, syncType: SyncType) -> Result<()>;
    fn Flush(&self, task: &Task, f: &File) -> Result<()>;

    fn UnstableAttr(&self, task: &Task, f: &File) -> Result<UnstableAttr>;
    fn Ioctl(&self, task: &Task, f: &File, fd: i32, request: u64, val: u64) -> Result<()>;

    fn IterateDir(&self, task: &Task,d: &Dirent, dirCtx: &mut DirCtx, offset: i32) -> (i32, Result<i64>);

    fn Mappable(&self) -> Result<HostInodeOp>;
}

pub struct FileInternal {
    pub UniqueId: u64,
    pub Dirent: Dirent,
    pub flags: QMutex<(FileFlags, Option<FileAsync>)>,

    //when we need to update the offset, we need to lock the offset lock
    //it is qlock, so the thread can switch when lock
    //pub offsetLock: QLock,
    pub offset: QLock<i64>,

    pub FileOp: Arc<FileOperations>,
}

#[derive(Clone)]
pub struct FileWeak(pub Weak<FileInternal>);

impl FileWeak {
    pub fn Upgrade(&self) -> Option<File> {
        let f = match self.0.upgrade() {
            None => return None,
            Some(f) => f,
        };

        return Some(File(f));
    }
}

impl File {
    pub fn Downgrade(&self) -> FileWeak {
        return FileWeak(Arc::downgrade(&self.0))
    }
}

#[derive(Clone)]
pub struct File(pub Arc<FileInternal>);

impl Drop for File {
    fn drop(&mut self) {
        //error!("File::Drop {}", Arc::strong_count(&self.0));
        if Arc::strong_count(&self.0) == 1 {
            // Drop BSD style locks.
            let inode = self.Dirent.Inode();
            let lockCtx = inode.lock().LockCtx.clone();
            let task = Task::Current();
            let lockUniqueID = task.fdTbl.ID();
            lockCtx.BSD.UnlockRegion(task, lockUniqueID, &Range::Max());
            lockCtx.Posix.UnlockRegion(task, lockUniqueID, &Range::Max());

            // Only unregister if we are currently registered. There is nothing
            // to register if f.async is nil (this happens when async mode is
            // enabled without setting an owner). Also, we unregister during
            // save.
            let mut f = self.flags.lock();
            if f.0.Async && f.1.is_some() {
                f.1.as_ref().unwrap().Unregister(task, self)
            }

            f.1 = None;
        }
    }
}

impl Ord for File {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.UniqueId.cmp(&other.UniqueId)
    }
}

impl PartialOrd for File {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for File {
    fn eq(&self, other: &Self) -> bool {
        return self.UniqueId == other.UniqueId
    }
}

impl Eq for File {}

impl Deref for File {
    type Target = Arc<FileInternal>;

    fn deref(&self) -> &Arc<FileInternal> {
        &self.0
    }
}

impl Waitable for File {
    fn Readiness(&self, task: &Task,mask: EventMask) -> EventMask {
        return self.FileOp.Readiness(task, mask);
    }

    fn EventRegister(&self, task: &Task,e: &WaitEntry, mask: EventMask) {
        self.FileOp.EventRegister(task, e, mask);
    }

    fn EventUnregister(&self, task: &Task,e: &WaitEntry) {
        self.FileOp.EventUnregister(task, e);
    }
}

impl Mapping for File {
    fn MappedName(&self, task: &Task) -> String {
        let root = task.Root();

        let (name, _) = self.Dirent.FullName(&root);
        return name;
    }

    // DeviceID returns the device number shown in /proc/[pid]/maps.
    fn DeviceID(&self) -> u64 {
        let inode = self.Dirent.Inode();
        return inode.lock().StableAttr.InodeId;
    }

    // InodeID returns the inode number shown in /proc/[pid]/maps.
    fn InodeID(&self) -> u64 {
        let inode = self.Dirent.Inode();
        return inode.lock().StableAttr().InodeId;
    }
}

impl File {
    pub fn WouldBlock(&self) -> bool {
        return self.Dirent.Inode().WouldBlock()
    }

    pub fn FileType(&self) -> InodeFileType {
        let d = self.Dirent.clone();
        let inode = d.Inode();
        return inode.lock().InodeOp.InodeFileType();
    }

    fn GetFileFlags(fd: i32) -> Result<FileFlags> {
        let ret = Fcntl(fd, Cmd::F_GETFL, 0) as i32;
        if ret < 0 {
            return Err(Error::SysError(-ret))
        }

        let mask = ret as u32;

        return Ok(FileFlags::FromFcntl(mask))
    }

    pub fn Blocking(&self) -> bool {
        return !self.flags.lock().0.NonBlocking;
    }

    pub fn Mappable(&self) -> Result<HostInodeOp> {
        return self.FileOp.Mappable();
    }

    // Async gets the stored FileAsync or creates a new one with the supplied
    // function. If the supplied function is nil, no FileAsync is created and the
    // current value is returned.
    pub fn Async(&self, task: &Task, newAsync: Option<FileAsync>) -> Option<FileAsync> {
        let mut f = self.flags.lock();

        if f.1.is_none() && newAsync.is_some() {
            f.1 = newAsync;
            if f.0.Async {
                f.1.as_ref().unwrap().Register(task, self)
            }
        }

        return f.1.clone()
    }

    pub fn New<T: FileOperations + 'static>(dirent: &Dirent, flags: &FileFlags, fops: T) -> Self {
        let f = FileInternal {
            UniqueId: NewUID(),
            Dirent: dirent.clone(),
            flags: QMutex::new((*flags, None)),
            //offsetLock: QLock::default(),
            offset: QLock::New(0),
            FileOp: Arc::new(fops),
        };

        return File(Arc::new(f));
    }

    pub fn NewFileFromFd(task: &Task, fd: i32, mounter: &FileOwner, isTTY: bool) -> Result<Self> {
        let mut fstat = LibcStat::default();

        let ret = Fstat(fd, &mut fstat) as i32;
        if ret < 0 {
            return Err(Error::SysError(-ret as i32))
        }

        let fileFlags = Self::GetFileFlags(fd)?;

        match fstat.st_mode as u16 & ModeType::S_IFMT {
            ModeType::S_IFSOCK => {
                if isTTY {
                    return Err(Error::Common("cannot import host socket as TTY".to_string()))
                }

                panic!("NewFileFromFd: not support socket fd");
                //let s = NewUnixSocket(task, fd, fileFlags.NonBlocking)?;
                //return Ok(s)
            }

            _ => {
                let msrc = MountSource::NewHostMountSource(&"/".to_string(), mounter, &WhitelistFileSystem::New(), &MountSourceFlags::default(), false);
                let inode = Inode::NewHostInode(&Arc::new(QMutex::new(msrc)), fd, &fstat, fileFlags.Write)?;
                let name = format!("host:[{}]", inode.lock().StableAttr.InodeId);
                let dirent = Dirent::New(&inode, &name);

                let iops = inode.lock().InodeOp.clone();
                let hostiops = iops.as_any().downcast_ref::<HostInodeOp>().unwrap();

                //let fops = iops.GetFileOp(task)?;
                let fops = hostiops.GetHostFileOp(task);
                let wouldBlock = inode.lock().InodeOp.WouldBlock();

                if isTTY {
                    return Ok(Self::NewTTYFile(&dirent, &fileFlags, fops))
                }

                return Ok(Self::NewHostFile(&dirent, &fileFlags, fops, wouldBlock))
            }
        }
    }

    pub fn NewHostFile(dirent: &Dirent, flags: &FileFlags, fops: Arc<FileOperations>, wouldBlock: bool) -> Self {
        let mut flags = *flags;

        if !wouldBlock {
            flags.Pread = true;
            flags.PWrite = true;
        }

        return File(Arc::new(FileInternal {
            UniqueId: NewUID(),
            Dirent: dirent.clone(),
            flags: QMutex::new((flags, None)),
            //offsetLock: QLock::default(),
            offset: QLock::New(0),
            FileOp: fops,
        }))
    }

    pub fn NewTTYFile(dirent: &Dirent, flags: &FileFlags, fops: Arc<HostFileOp>) -> Self {
        let ttyfileops = TTYFileOps::New(fops);

        return Self::New(dirent, flags, ttyfileops)
    }

    pub fn UniqueId(&self) -> u64 {
        return self.UniqueId;
    }

    pub fn Flags(&self) -> FileFlags {
        return self.flags.lock().0;
    }

    pub fn SetFlags(&self, task: &Task, newFlags: SettableFileFlags) {
        let mut f = self.flags.lock();
        f.0.Direct = newFlags.Direct;
        f.0.NonBlocking = newFlags.NonBlocking;
        f.0.Append = newFlags.Append;

        match &f.1 {
            None => (),
            Some(ref a) =>{
                if newFlags.Async && !f.0.Async {
                    a.Register(task, self)
                }
                if !newFlags.Async && f.0.Async {
                    a.Unregister(task, self)
                }
            }
        }

        f.0.Async = newFlags.Async;
    }

    pub fn Offset(&self, task: &Task) -> Result<i64> {
        return Ok(*self.offset.Lock(task)?);
    }

    pub fn Seek(&self, task: &Task, whence: i32, offset: i64) -> Result<i64> {
        let fops = self.FileOp.clone();

        let mut offsetLock = self.offset.Lock(task)?;

        let current = *offsetLock;
        let newOffset = fops.Seek(task, self, whence, current, offset)?;
        *offsetLock = newOffset;
        return Ok(newOffset)
    }

    pub fn ReadDir(&self, task: &Task, serializer: &mut DentrySerializer) -> Result<()> {
        let fops = self.FileOp.clone();
        let mut offsetLock = self.offset.Lock(task)?;

        let current = *offsetLock;
        *offsetLock = fops.ReadDir(task, self, current, serializer)?;
        return Ok(())
    }

    pub fn Readv(&self, task: &Task, dsts: &mut [IoVec]) -> Result<i64> {
        let fops = self.FileOp.clone();
        let seekable = fops.Seekable();

        //let inode = self.Dirent.Inode();
        //error!("Readv inodetype is {:?}, fopstype is {:?}", inode.InodeType(), fops.FopsType());
        if seekable {
            let mut offsetLock = self.offset.Lock(task)?;

            let current = *offsetLock;

            READS.Incr();
            let blocking = self.Blocking();
            let n = fops.ReadAt(task, self, dsts, current, blocking)?;

            if n > 0 {
                *offsetLock = current + n;
            }

            return Ok(n)
        } else {
            let blocking = self.Blocking();
            let n = fops.ReadAt(task, self, dsts, 0, blocking)?;
            return Ok(n)
        }
    }

    pub fn Preadv(&self, task: &Task, dsts: &mut [IoVec], offset: i64) -> Result<i64> {
        let fops = self.FileOp.clone();
        let blocking = self.Blocking();
        let n = fops.ReadAt(task, self, dsts, offset, blocking)?;
        return Ok(n)
    }

    pub fn offsetForAppend(&self, task: &Task) -> Result<i64> {
        let inode = self.Dirent.Inode();
        let uattr = match inode.UnstableAttr(task) {
            Err(_) => return Err(Error::SysError(SysErr::EIO)),
            Ok(u) => u,
        };

        return Ok(uattr.Size)
    }

    // checkLimit checks the offset that the write will be performed at. The
    // returned boolean indicates that the write must be limited. The returned
    // integer indicates the new maximum write length.
    pub fn checkLimit(&self, _offset: i64) -> (i64, bool) {
        //todo: implement this
        return (0, false)
    }

    pub fn Writev(&self, task: &Task, srcs: &[IoVec]) -> Result<i64> {
        let fops = self.FileOp.clone();
        let seekable = fops.Seekable();

        //let inode = self.Dirent.Inode();
        //error!("writev inodetype is {:?}, fopstype is {:?}", inode.InodeType(), fops.FopsType());

        if seekable {
            let mut offsetLock = self.offset.Lock(task)?;
            if self.flags.lock().0.Append {
                let (cnt, len) = fops.Append(task, self, srcs)?;
                *offsetLock = len;
                return Ok(cnt)
            }

            let current = *offsetLock;

            let (limit, ok) = self.checkLimit(current);
            if ok && limit == 0 {
                return Err(Error::ErrExceedsFileSizeLimit);
            }

            let blocking = self.Blocking();
            let n = fops.WriteAt(task, self, srcs, current, blocking)?;
            if n > 0 {
                *offsetLock = current + n;
            }

            return Ok(n)
        } else {
            let blocking = self.Blocking();
            let n = fops.WriteAt(task, self, srcs, 0, blocking)?;

            return Ok(n)
        }
    }

    pub fn Pwritev(&self, task: &Task, srcs: &[IoVec], offset: i64) -> Result<i64> {
        let fops = self.FileOp.clone();

        /*
        POSIX requires that opening a file with the O_APPEND flag should have
       no effect on the location at which pwrite() writes data.  However, on
       Linux, if a file is opened with O_APPEND, pwrite() appends data to
       the end of the file, regardless of the value of offset.

       //todo: study whether we need to enable this

        if self.flags.lock().0.Append {
            let (cnt, _len) = fops.Append(task, self, srcs)?;
            return Ok(cnt)
        }*/

        let (limit, ok) = self.checkLimit(offset);
        if ok && limit == 0 {
            return Err(Error::ErrExceedsFileSizeLimit);
        }

        let blocking = self.Blocking();
        let n = fops.WriteAt(task, self, srcs, offset, blocking)?;

        return Ok(n)
    }

    pub fn Fsync(&self, task: &Task, start: i64, end: i64, syncType: SyncType) -> Result<()> {
        let fops = self.FileOp.clone();
        return fops.Fsync(task, self, start, end, syncType);
    }

    pub fn Flush(&self, task: &Task) -> Result<()> {
        let fops = self.FileOp.clone();

        let flags = self.Flags();
        if flags.Write {
            let res = fops.Flush(task, self);
            return res;
        }

        return Ok(());
    }

    pub fn UnstableAttr(&self, task: &Task) -> Result<UnstableAttr> {
        let fops = self.FileOp.clone();
        return fops.UnstableAttr(task, self);
    }

    pub fn Ioctl(&self, task: &Task, fd: i32, request: u64, val: u64) -> Result<()> {
        let fops = self.FileOp.clone();
        let res = fops.Ioctl(task, self, fd, request, val);
        return res;
    }
}
