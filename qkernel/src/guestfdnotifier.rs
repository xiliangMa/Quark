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

use alloc::collections::btree_map::BTreeMap;
use ::qlib::mutex::*;
use core::ops::Deref;

use super::Kernel::HostSpace;
use super::kernel::waiter::*;
use super::fs::host::hostinodeop::*;
use super::qlib::common::*;
use super::qlib::linux_def::*;
use super::qlib::singleton::*;
use super::IOURING;

pub static GUEST_NOTIFIER : Singleton<Notifier> = Singleton::<Notifier>::New();

pub fn AddFD(fd: i32, iops: &HostInodeOp) {
    GUEST_NOTIFIER.AddFD(fd, iops);
}

pub fn RemoveFD(fd: i32) {
    GUEST_NOTIFIER.RemoveFD(fd);
}

pub fn UpdateFD(fd: i32) -> Result<()> {
    return GUEST_NOTIFIER.UpdateFD(fd);
}

pub fn NonBlockingPoll(fd: i32, mask: EventMask) -> EventMask {
    return HostSpace::NonBlockingPoll(fd, mask) as EventMask
}

pub fn Notify(fd: i32, mask: EventMask) {
    GUEST_NOTIFIER.Notify(fd, mask);
}

pub fn HostLogFlush() {
    //GUEST_NOTIFIER.PrintStrRespHandler(addr, len)
    super::IOURING.LogFlush();
}

pub struct GuestFdInfo {
    pub queue: Queue,
    pub mask: EventMask,
    pub iops: HostInodeOpWeak,
    pub userdata: Option<usize>,
}

// notifier holds all the state necessary to issue notifications when IO events
// occur in the observed FDs.
pub struct NotifierInternal {
    // fdMap maps file descriptors to their notification queues and waiting
    // status.
    fdMap: BTreeMap<i32, GuestFdInfo>,
}

pub struct Notifier(QMutex<NotifierInternal>);

impl Deref for Notifier {
    type Target = QMutex<NotifierInternal>;

    fn deref(&self) -> &QMutex<NotifierInternal> {
        &self.0
    }
}

impl Notifier {
    pub fn New() -> Self {
        let internal = NotifierInternal {
            fdMap: BTreeMap::new()
        };

        return Self(QMutex::new(internal))
    }

    fn Waitfd(&self, fd: i32, mask: EventMask) -> Result<()> {
        let mut n = self.lock();
        let fi = match n.fdMap.get_mut(&fd) {
            None => {
                panic!("Notifier::waitfd can't find fd {}", fd)
            }
            Some(fi) => fi,
        };

        if fi.mask == mask {
            return Ok(())
        }

        if fi.mask != 0 {
            let userdata = fi.userdata.take();

            match userdata {
                None => {
                    panic!("Notifier::Waitfd get non userdata");
                },
                Some(idx) => {
                   IOURING.AsyncPollRemove(idx as u64);
                }
            }
        }

        if mask != 0 {
            let idx = IOURING.AsyncPollAdd(fd, mask as _);
            fi.userdata = Some(idx);
        }
        fi.mask = mask;

        return Ok(())
    }

    pub fn UpdateFD(&self, fd: i32) -> Result<()> {
        let mask = {
            let mut n = self.lock();
            let fi = match n.fdMap.get_mut(&fd) {
                None => {
                    return Ok(())
                }
                Some(fi) => fi,
            };

            let mask = fi.queue.Events();

            mask
        };

        return self.Waitfd(fd, mask);
    }

    pub fn AddFD(&self, fd: i32, iops: &HostInodeOp) {
        let mut n = self.lock();

        let queue = iops.lock().queue.clone();

        if n.fdMap.contains_key(&fd) {
            panic!("GUEST_NOTIFIER::AddFD fd {} added twice", fd);
        }

        n.fdMap.insert(fd, GuestFdInfo {
            queue: queue.clone(),
            mask: 0,
            iops: iops.Downgrade(),
            userdata: None,
        });
    }

    pub fn RemoveFD(&self, fd: i32) {
        let mut n = self.lock();
        let mut fi = match n.fdMap.remove(&fd) {
            None => {
                panic!("Notifier::RemoveFD can't find fd {}", fd)
            }
            Some(fi) => fi
        };

        let userdata = fi.userdata.take();
        match userdata {
            None => (),
            Some(idx) => {
                IOURING.AsyncPollRemove(idx as u64);
            }
        }
    }

    pub fn Notify(&self, fd: i32, mask: EventMask) {
        let n = self.lock();
        match n.fdMap.get(&fd) {
            None => (),
            Some(fi) => {
                fi.queue.Notify(EventMaskFromLinux(mask as u32));
            }
        }
    }
}