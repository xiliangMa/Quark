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

use alloc::vec::Vec;
use alloc::str;
use alloc::string::String;

use super::super::qlib::common::*;
use super::super::qlib::linux_def::*;
use super::super::task::*;

pub struct CString {
    pub data: Vec<u8>
}

impl CString {
    pub fn New(s: &str) -> Self {
        let s = s.as_bytes();
        let mut data = Vec::with_capacity(s.len() + 1);
        for i in 0..s.len() {
            data.push(s[i])
        }

        data.push(0);
        return Self {
            data
        }
    }

    pub fn Ptr(&self) -> u64 {
        return &self.data[0] as *const _ as u64;
    }

    pub fn Len(&self) -> usize {
        return self.data.len();
    }

    pub fn Slice(&self) -> &[u8] {
        return &self.data[..]
    }

    pub const MAX_STR_LEN: usize = 4096;
    pub fn ToString(task: &Task, addr: u64) -> Result<String> {
        return Self::ToStringWithLen(task, addr, Self::MAX_STR_LEN);
    }

    pub fn ToStringWithLen(task: &Task, addr: u64, len: usize) -> Result<String> {
        let (str, err) = task.CopyInString(addr, len);
        match err {
            Err(_) => return Err(Error::SysError(SysErr::EINVAL)),
            Ok(()) => return Ok(str)
        }
    }

    pub fn CopyInString(task: &Task, addr: u64, len: usize) -> (String, Result<()>) {
        return task.CopyInString(addr, len);
    }
}