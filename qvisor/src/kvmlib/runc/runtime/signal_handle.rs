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

use nix::sys::signal;
use lazy_static::lazy_static;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

use super::super::super::qlib::common::*;
use super::super::super::qlib::control_msg::*;
use super::super::super::vmspace::*;
use super::super::super::ucall::usocket::*;

lazy_static! {
    static ref SIGNAL_HANDLE_ENABLE : AtomicBool = AtomicBool::new(false);
    static ref CONSOLE : AtomicBool = AtomicBool::new(false);
}

pub fn StartSignalHandle() {
    SIGNAL_HANDLE_ENABLE.store(true, Ordering::SeqCst);
}

pub fn StopSignalHandle() {
    SIGNAL_HANDLE_ENABLE.store(false, Ordering::SeqCst);
}

pub fn SetConole(terminal: bool) {
    CONSOLE.store(terminal, Ordering::SeqCst);
}


#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SignalFaultInfo {
    pub Signo: i32,
    // Signal number
    pub Errno: i32,
    // Errno value
    pub Code: i32,
    // Signal code
    pub _r: u32,

    pub addr: u64,
    pub lsb: u16,
}

extern fn handle_sigintAct(signal :i32, signInfo: *mut libc::siginfo_t, _: *mut libc::c_void) {
    let console = CONSOLE.load(Ordering::SeqCst);

    {
        let vms = super::super::super::VMS.lock();

        error!("signal debug");
        for i in 0..8 {
            error!("vcpu[{}] state is {}/{}", i, vms.GetShareSpace().GetValue(i, 0), vms.GetShareSpace().GetValue(i, 1))
        }

        error!("vcpus is {:x?}", vms.GetShareSpace().scheduler.VcpuArr);
    }

    if SIGNAL_HANDLE_ENABLE.load(Ordering::Relaxed) {
        let sigfault: &SignalFaultInfo = unsafe {
            &*(signInfo as u64 as * const SignalFaultInfo)
        };

        error!("get signal {}, action is {:x?}", signal, sigfault);


        let payload = Payload::Signal({
            SignalArgs {
                Signo: signal,
                PID: 0,
                Mode: if console {
                    SignalDeliveryMode::DeliverToForegroundProcessGroup
                } else {
                    SignalDeliveryMode::DeliverToProcess
                }
            }
        });
        SendControlMsg(USocket::DummyUSocket(), ControlMsg::New(payload))
            .expect("handle_sigint fail when SendControlMsg")
    }
}

// numSignals is the number of normal (non-realtime) signals on Linux.
pub const NUM_SIGNALS : usize = 32;

pub fn SignAction() {
    let sig_action = signal::SigAction::new(signal::SigHandler::SigAction(handle_sigintAct),
                                            signal::SaFlags::empty(),
                                            signal::SigSet::empty());

    unsafe {
        signal::sigaction(signal::SIGINT, &sig_action).expect("sigaction set fail");
    }
}

pub fn PrepareHandler() -> Result<()> {
    unsafe {
        libc::ioctl( 0, libc::TIOCSCTTY, 0);
    }

    let sig_action = signal::SigAction::new(signal::SigHandler::SigAction(handle_sigintAct),
                                            signal::SaFlags::empty(),
                                            signal::SigSet::empty());

    for i in 1..NUM_SIGNALS {
        if i == 9           //SIGKILL
            || i == 19 {    //SIGSTOP
            continue
        }

        unsafe {
            signal::sigaction(signal::Signal::from_c_int(i as i32).unwrap(), &sig_action)
                .map_err(|e| Error::Common(format!("sigaction fail with err {:?} for signal {}", e, i)))?;
        }
    }

    return Ok(())
}